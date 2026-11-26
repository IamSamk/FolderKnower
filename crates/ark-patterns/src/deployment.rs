//! Deployment map: services from `docker-compose`/`compose` files and Dockerfiles, the
//! links between them, their memory limits and GPUs, and which code runs where.
//!
//! Only facts written in the files are used. Environment values are read to find host
//! names (`QDRANT_URL=http://qdrant:6333` links the service to `qdrant`) and are never
//! stored; only variable names are kept.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use ark_schema::{Deployment, LinkVia, Service, ServiceKind, ServiceLink, TierLabel};
use regex::Regex;
use serde::Deserialize;
use serde_yaml_ng::Value;

pub const SERVICES_YAML: &str = include_str!("../../../ontology/services-catalog.yaml");

/// A compose file, Dockerfile or proxy config with its text.
pub struct InfraFile<'a> {
    pub path: &'a str,
    pub text: &'a str,
}

/// A source file and the tier it was sorted into.
pub struct CodeFile<'a> {
    pub path: &'a str,
    pub tier: TierLabel,
}

#[derive(Deserialize)]
struct RawCatalog {
    version: String,
    images: Vec<RawImage>,
}

#[derive(Deserialize)]
struct RawImage {
    #[serde(rename = "match")]
    pattern: String,
    kind: ServiceKind,
    name: String,
    explain: String,
}

struct ImageRule {
    re: Regex,
    kind: ServiceKind,
    name: String,
    explain: String,
}

struct Catalog {
    version: String,
    images: Vec<ImageRule>,
}

static CATALOG: LazyLock<Catalog> = LazyLock::new(|| {
    let raw: RawCatalog = serde_yaml_ng::from_str(SERVICES_YAML).expect("services catalog parses");
    Catalog {
        version: raw.version,
        images: raw
            .images
            .into_iter()
            .map(|i| ImageRule {
                re: Regex::new(&i.pattern).expect("services catalog regex"),
                kind: i.kind,
                name: i.name,
                explain: i.explain,
            })
            .collect(),
    }
});

pub fn catalog_version() -> &'static str {
    &CATALOG.version
}

static INTERP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{([A-Za-z0-9_]+)(?::?[-?]([^}]*))?\}").unwrap());
static HOST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|://|@|[=,;\s])([A-Za-z][A-Za-z0-9_.-]*)(?::(\d{2,5}))?(?:$|[/,;\s?])")
        .unwrap()
});
static PROXY_PASS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"proxy_pass\s+https?://([A-Za-z0-9_.-]+)(?::(\d+))?").unwrap());

pub fn is_compose(path: &str) -> bool {
    let name = file_name(path).to_ascii_lowercase();
    (name.starts_with("docker-compose") || name.starts_with("compose."))
        && (name.ends_with(".yml") || name.ends_with(".yaml"))
}

pub fn is_dockerfile(path: &str) -> bool {
    let name = file_name(path).to_ascii_lowercase();
    name == "dockerfile" || name.starts_with("dockerfile.") || name.ends_with(".dockerfile")
}

pub fn is_proxy_conf(path: &str) -> bool {
    let name = file_name(path).to_ascii_lowercase();
    name == "nginx.conf" || (name.ends_with(".conf") && path.to_ascii_lowercase().contains("nginx"))
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
}

/// `a/b/../c` and `./x` -> normalized, relative, no leading `./`.
fn join(base: &str, rel: &str) -> String {
    let mut parts: Vec<&str> = if rel.starts_with('/') {
        Vec::new()
    } else {
        base.split('/').filter(|p| !p.is_empty()).collect()
    };
    for p in rel.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            x => parts.push(x),
        }
    }
    parts.join("/")
}

/// Resolve `${VAR:-default}` to its default; bare `${VAR}` to empty.
fn interpolate(s: &str) -> String {
    INTERP
        .replace_all(s, |c: &regex::Captures| {
            c.get(2).map(|m| m.as_str().to_owned()).unwrap_or_default()
        })
        .into_owned()
}

/// `512m`, `2g`, `1.5G`, `1024M`, `1gb`, `268435456` -> bytes.
pub fn parse_bytes(s: &str) -> Option<u64> {
    let t = s.trim().to_ascii_lowercase();
    let t = t.trim_end_matches('b');
    let (num, mult) = match t.chars().last()? {
        'k' => (&t[..t.len() - 1], 1024f64),
        'm' => (&t[..t.len() - 1], 1024f64 * 1024.0),
        'g' => (&t[..t.len() - 1], 1024f64 * 1024.0 * 1024.0),
        't' => (&t[..t.len() - 1], 1024f64.powi(4)),
        c if c.is_ascii_digit() => (t, 1.0),
        _ => return None,
    };
    let n: f64 = num.trim().parse().ok()?;
    (n >= 0.0).then_some((n * mult) as u64)
}

fn image_base(image: &str) -> String {
    // Strip digest, tag and a registry host (first segment with a dot or colon, or localhost).
    let no_digest = image.split('@').next().unwrap_or(image);
    let mut segs: Vec<&str> = no_digest.split('/').collect();
    if segs.len() > 1 && (segs[0].contains('.') || segs[0].contains(':') || segs[0] == "localhost")
    {
        segs.remove(0);
    }
    let last = segs.pop().unwrap_or("");
    let last = last.split(':').next().unwrap_or(last);
    segs.push(last);
    let s = segs.join("/");
    s.strip_prefix("library/")
        .unwrap_or(&s)
        .to_ascii_lowercase()
}

fn classify_image(image: &str) -> Option<&'static ImageRule> {
    let base = image_base(image);
    CATALOG.images.iter().find(|r| r.re.is_match(&base))
}

fn as_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn get<'a>(v: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut cur = v;
    for k in path {
        cur = cur.as_mapping()?.get(Value::String((*k).to_owned()))?;
    }
    Some(cur)
}

fn string_list(v: Option<&Value>) -> Vec<String> {
    match v {
        Some(Value::Sequence(s)) => s
            .iter()
            .filter_map(|x| {
                as_str(x).or_else(|| {
                    // long-form ports/volumes: {target: 80, published: 8080} / {source, target}
                    let m = x.as_mapping()?;
                    let pick = |k: &str| m.get(Value::String(k.to_owned())).and_then(as_str);
                    match (pick("published").or(pick("source")), pick("target")) {
                        (Some(a), Some(b)) => Some(format!("{a}:{b}")),
                        (None, Some(b)) => Some(b),
                        _ => None,
                    }
                })
            })
            .collect(),
        Some(Value::Mapping(m)) => m.keys().filter_map(as_str).collect(),
        Some(v) => as_str(v).into_iter().collect(),
        None => Vec::new(),
    }
}

/// Environment as (key, value) pairs; values only feed link detection.
fn env_pairs(v: Option<&Value>) -> Vec<(String, String)> {
    match v {
        Some(Value::Sequence(s)) => s
            .iter()
            .filter_map(as_str)
            .map(|e| match e.split_once('=') {
                Some((k, v)) => (k.trim().to_owned(), interpolate(v.trim())),
                None => (e.trim().to_owned(), String::new()),
            })
            .collect(),
        Some(Value::Mapping(m)) => m
            .iter()
            .filter_map(|(k, v)| {
                Some((
                    as_str(k)?,
                    as_str(v).map(|x| interpolate(&x)).unwrap_or_default(),
                ))
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn command_text(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::Sequence(s) => {
            let parts: Vec<String> = s.iter().filter_map(as_str).collect();
            (!parts.is_empty()).then(|| parts.join(" "))
        }
        v => as_str(v)
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty()),
    }
}

#[derive(Default)]
struct DockerInfo {
    base: Option<String>,
    expose: Vec<String>,
    command: Option<String>,
}

fn parse_dockerfile(text: &str) -> DockerInfo {
    let mut d = DockerInfo::default();
    let mut entry: Option<String> = None;
    let mut cmd: Option<String> = None;
    // Join line continuations first.
    let joined = text.replace("\\\r\n", " ").replace("\\\n", " ");
    for line in joined.lines() {
        let l = line.trim();
        if l.starts_with('#') || l.is_empty() {
            continue;
        }
        let (ins, rest) = l.split_once(char::is_whitespace).unwrap_or((l, ""));
        let rest = rest.trim();
        match ins.to_ascii_uppercase().as_str() {
            "FROM" => {
                // Last stage wins: that is the image that runs. Drop `--platform` and `AS x`.
                let img = rest
                    .split_whitespace()
                    .find(|t| !t.starts_with("--"))
                    .unwrap_or("");
                d.base = Some(img.to_owned());
                d.expose.clear();
                entry = None;
                cmd = None;
            }
            "EXPOSE" => d.expose.extend(rest.split_whitespace().map(str::to_owned)),
            "CMD" => cmd = Some(exec_form(rest)),
            "ENTRYPOINT" => entry = Some(exec_form(rest)),
            _ => {}
        }
    }
    d.command = match (entry, cmd) {
        (Some(e), Some(c)) => Some(format!("{e} {c}")),
        (e, c) => e.or(c),
    };
    d
}

fn exec_form(s: &str) -> String {
    if s.starts_with('[')
        && let Ok(v) = serde_json::from_str::<Vec<String>>(s)
    {
        return v.join(" ");
    }
    s.to_owned()
}

/// Which source file a start command runs, if it names one we have.
fn entry_file(command: &str, context: &str, code: &BTreeSet<&str>) -> Option<String> {
    let toks: Vec<&str> = command.split_whitespace().collect();
    let mut candidates: Vec<String> = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        let t = t.trim_matches(|c| c == '"' || c == '\'');
        // uvicorn/gunicorn/hypercorn module:app
        if let Some((module, _)) = t.split_once(':')
            && !module.contains('/')
            && module
                .chars()
                .all(|c| c.is_alphanumeric() || c == '.' || c == '_')
            && !module.is_empty()
        {
            let p = module.replace('.', "/");
            candidates.push(format!("{p}.py"));
            candidates.push(format!("{p}/__init__.py"));
        }
        // python -m pkg.mod / celery -A pkg.mod
        if matches!(t, "-m" | "-A" | "--app")
            && let Some(m) = toks.get(i + 1)
        {
            let p = m.split(':').next().unwrap_or(m).replace('.', "/");
            candidates.push(format!("{p}.py"));
            candidates.push(format!("{p}/__main__.py"));
            candidates.push(format!("{p}/__init__.py"));
        }
        // python app.py / node server.js / deno run main.ts
        let lower = t.to_ascii_lowercase();
        if [".py", ".js", ".mjs", ".cjs", ".ts", ".go", ".rs"]
            .iter()
            .any(|e| lower.ends_with(e))
        {
            candidates.push(t.trim_start_matches("./").to_owned());
        }
    }
    for c in candidates {
        // Relative to the build context, then to a typical WORKDIR copy of it.
        for p in [join(context, &c), c.clone()] {
            if code.contains(p.as_str()) {
                return Some(format!("file:{p}"));
            }
        }
    }
    None
}

fn kind_from_code(files: &[&CodeFile], command: Option<&str>, name: &str) -> ServiceKind {
    let lname = name.to_ascii_lowercase();
    let cmd = command.unwrap_or("").to_ascii_lowercase();
    if cmd.contains("celery")
        || cmd.contains("rq worker")
        || cmd.contains("dramatiq")
        || lname.contains("worker")
    {
        return ServiceKind::Worker;
    }
    if lname.contains("agent") {
        return ServiceKind::Agent;
    }
    let mut by: BTreeMap<u8, usize> = BTreeMap::new();
    for f in files {
        let k = match f.tier {
            TierLabel::Frontend => 0,
            TierLabel::Backend | TierLabel::Gateway | TierLabel::Storage => 1,
            TierLabel::Workers => 2,
            TierLabel::Shared => continue,
        };
        *by.entry(k).or_default() += 1;
    }
    match by.iter().max_by_key(|(_, n)| **n).map(|(k, _)| *k) {
        Some(0) => ServiceKind::Frontend,
        Some(2) => ServiceKind::Worker,
        Some(_) => ServiceKind::Api,
        None if lname.contains("front") || lname.contains("web") || lname.contains("ui") => {
            ServiceKind::Frontend
        }
        None => ServiceKind::Other,
    }
}

fn kind_explain(kind: ServiceKind, n: u32) -> String {
    let files = if n == 1 {
        "1 source file".to_owned()
    } else {
        format!("{n} source files")
    };
    match kind {
        ServiceKind::Frontend => format!(
            "Frontend built from this repository ({files}): the screens the user sees in the browser."
        ),
        ServiceKind::Api => format!(
            "Backend API built from this repository ({files}): receives HTTP requests and runs the app logic."
        ),
        ServiceKind::Worker => format!(
            "Background worker built from this repository ({files}): runs slow jobs outside the request."
        ),
        ServiceKind::Agent => format!(
            "Agent service built from this repository ({files}): runs the model-driven decide-act loop."
        ),
        _ => format!("Service built from this repository ({files})."),
    }
}

/// Memory settings passed on the command line or in env (Redis maxmemory, Ollama
/// parallelism and residency, vLLM memory share), written as plain sentences.
fn runtime_notes(command: Option<&str>, env: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    let toks: Vec<&str> = command.unwrap_or("").split_whitespace().collect();
    let flag = |name: &str| {
        toks.iter()
            .position(|t| *t == name)
            .and_then(|i| toks.get(i + 1))
            .map(|v| v.trim_matches('"').to_owned())
            .or_else(|| {
                toks.iter()
                    .find_map(|t| t.strip_prefix(&format!("{name}=")).map(str::to_owned))
            })
    };
    if let Some(m) = flag("--maxmemory") {
        let policy = flag("--maxmemory-policy").unwrap_or_else(|| "noeviction".into());
        out.push(format!(
            "It may hold {m} of data; when full it evicts keys by the {policy} policy."
        ));
    }
    if let Some(u) = flag("--gpu-memory-utilization") {
        out.push(format!(
            "vLLM may claim {u} of the GPU's memory for weights plus KV cache."
        ));
    }
    if let Some(l) = flag("--max-model-len") {
        out.push(format!(
            "Each request's KV cache is sized for up to {l} tokens."
        ));
    }
    for (k, v) in env {
        match k.as_str() {
            "OLLAMA_KEEP_ALIVE" => out.push(format!(
                "A model stays loaded for {v} after its last request."
            )),
            "OLLAMA_NUM_PARALLEL" => out.push(format!(
                "Up to {v} requests share one loaded model, each with its own KV cache."
            )),
            "OLLAMA_MAX_LOADED_MODELS" => {
                out.push(format!("At most {v} models are kept in memory at once."))
            }
            _ => {}
        }
    }
    out
}

/// The catalog's explanation, led by the product name unless it already starts with it.
fn named(r: &ImageRule) -> String {
    if r.explain
        .to_ascii_lowercase()
        .starts_with(&r.name.to_ascii_lowercase())
    {
        r.explain.clone()
    } else {
        format!("{}: {}", r.name, r.explain)
    }
}

fn line_of(text: &str, needle: &str) -> u32 {
    text.lines()
        .position(|l| {
            let t = l.trim_start();
            t.strip_prefix(needle).is_some_and(|r| r.starts_with(':')) && l.len() - t.len() <= 4
        })
        .map(|i| i as u32 + 1)
        .unwrap_or(1)
}

struct Draft {
    svc: Service,
    hosts: BTreeSet<String>,
    env: Vec<(String, String)>,
    depends: Vec<String>,
}

pub fn analyze(infra: &[InfraFile], code: &[CodeFile]) -> Option<Deployment> {
    let code_set: BTreeSet<&str> = code.iter().map(|c| c.path).collect();
    let dockerfiles: BTreeMap<&str, &str> = infra
        .iter()
        .filter(|f| is_dockerfile(f.path))
        .map(|f| (f.path, f.text))
        .collect();
    let mut sources: BTreeSet<String> = BTreeSet::new();
    let mut drafts: BTreeMap<String, Draft> = BTreeMap::new();
    let mut used_dockerfiles: BTreeSet<String> = BTreeSet::new();

    // Every folder with a Dockerfile is a build context; a file belongs to the deepest one.
    let contexts: BTreeSet<&str> = infra
        .iter()
        .filter(|f| is_dockerfile(f.path))
        .map(|f| dir_of(f.path))
        .collect();
    let under = |path: &str, ctx: &str| ctx.is_empty() || path.starts_with(&format!("{ctx}/"));
    let in_context = |ctx: &str| -> Vec<&CodeFile> {
        code.iter()
            .filter(|c| under(c.path, ctx))
            .filter(|c| {
                !contexts
                    .iter()
                    .any(|o| o.len() > ctx.len() && under(o, ctx) && under(c.path, o))
            })
            .collect()
    };

    let mut composes: Vec<&InfraFile> = infra.iter().filter(|f| is_compose(f.path)).collect();
    composes.sort_by_key(|f| (f.path.matches('/').count(), f.path));
    for f in composes {
        let Ok(doc) = serde_yaml_ng::from_str::<Value>(f.text) else {
            continue;
        };
        let Some(services) = get(&doc, &["services"]).and_then(Value::as_mapping) else {
            continue;
        };
        sources.insert(f.path.to_owned());
        let base = dir_of(f.path);
        for (k, v) in services {
            let Some(name) = as_str(k) else { continue };
            let image = get(v, &["image"]).and_then(as_str).map(|s| interpolate(&s));
            let (context, dockerfile) = match get(v, &["build"]) {
                Some(Value::String(s)) => (Some(join(base, &interpolate(s))), None),
                Some(b @ Value::Mapping(_)) => (
                    Some(join(
                        base,
                        &get(b, &["context"])
                            .and_then(as_str)
                            .unwrap_or_else(|| ".".into()),
                    )),
                    get(b, &["dockerfile"]).and_then(as_str),
                ),
                _ => (None, None),
            };
            let docker = context.as_ref().and_then(|ctx| {
                let p = join(ctx, dockerfile.as_deref().unwrap_or("Dockerfile"));
                dockerfiles.get(p.as_str()).map(|t| {
                    used_dockerfiles.insert(p.clone());
                    sources.insert(p);
                    parse_dockerfile(t)
                })
            });
            let command = command_text(get(v, &["command"]))
                .or_else(|| command_text(get(v, &["entrypoint"])))
                .or_else(|| docker.as_ref().and_then(|d| d.command.clone()));
            let mut ports = string_list(get(v, &["ports"]));
            if ports.is_empty() {
                ports = string_list(get(v, &["expose"]));
            }
            if ports.is_empty()
                && let Some(d) = &docker
            {
                ports = d.expose.clone();
            }
            let limits = get(v, &["deploy", "resources", "limits"]);
            let memory_limit = limits
                .and_then(|l| get(l, &["memory"]))
                .or_else(|| get(v, &["mem_limit"]))
                .and_then(as_str)
                .and_then(|s| parse_bytes(&interpolate(&s)));
            let memory_reservation = get(v, &["deploy", "resources", "reservations", "memory"])
                .or_else(|| get(v, &["mem_reservation"]))
                .and_then(as_str)
                .and_then(|s| parse_bytes(&interpolate(&s)));
            let cpu_limit = limits
                .and_then(|l| get(l, &["cpus"]))
                .or_else(|| get(v, &["cpus"]))
                .and_then(as_str);
            let gpu = get(v, &["gpus"]).is_some()
                || get(v, &["runtime"])
                    .and_then(as_str)
                    .is_some_and(|r| r == "nvidia")
                || get(v, &["deploy", "resources", "reservations", "devices"])
                    .and_then(Value::as_sequence)
                    .is_some_and(|ds| {
                        ds.iter().any(|d| {
                            string_list(get(d, &["capabilities"]))
                                .iter()
                                .any(|c| c == "gpu")
                                || get(d, &["driver"])
                                    .and_then(as_str)
                                    .is_some_and(|x| x == "nvidia")
                        })
                    });
            let env = env_pairs(get(v, &["environment"]));
            let mut depends = string_list(get(v, &["depends_on"]));
            depends.extend(
                string_list(get(v, &["links"]))
                    .into_iter()
                    .map(|l| l.split(':').next().unwrap_or(&l).to_owned()),
            );
            let mut hosts: BTreeSet<String> = BTreeSet::from([name.clone()]);
            for key in ["container_name", "hostname"] {
                if let Some(h) = get(v, &[key]).and_then(as_str) {
                    hosts.insert(h);
                }
            }
            if let Some(nets) = get(v, &["networks"]).and_then(Value::as_mapping) {
                for n in nets.values() {
                    hosts.extend(string_list(get(n, &["aliases"])));
                }
            }
            let rule = image.as_deref().and_then(classify_image);
            let files = context.as_deref().map(in_context).unwrap_or_default();
            let n_files = files.len() as u32;
            let base_rule = docker
                .as_ref()
                .and_then(|d| d.base.as_deref())
                .and_then(classify_image);
            let (kind, explain) = match rule.filter(|r| r.kind != ServiceKind::Other) {
                Some(r) => (r.kind, named(r)),
                None if context.is_some() => {
                    let k = kind_from_code(&files, command.as_deref(), &name);
                    (k, kind_explain(k, n_files))
                }
                None => match base_rule.filter(|r| r.kind != ServiceKind::Other) {
                    Some(r) => (r.kind, named(r)),
                    None => (
                        ServiceKind::Other,
                        "A service declared in compose.".to_owned(),
                    ),
                },
            };
            let entry = match (&command, &context) {
                (Some(c), Some(ctx)) => entry_file(c, ctx, &code_set),
                (Some(c), None) => entry_file(c, "", &code_set),
                _ => None,
            };
            let mut env_keys: Vec<String> = env.iter().map(|(k, _)| k.clone()).collect();
            env_keys.sort();
            env_keys.dedup();
            let mut explain = explain;
            for n in runtime_notes(command.as_deref(), &env) {
                explain.push(' ');
                explain.push_str(&n);
            }
            let svc = Service {
                id: format!("svc:{name}"),
                name: name.clone(),
                kind,
                image: image
                    .clone()
                    .or_else(|| docker.as_ref().and_then(|d| d.base.clone())),
                build_context: context.clone(),
                command,
                entry_file: entry,
                ports,
                memory_limit,
                memory_reservation,
                cpu_limit,
                gpu,
                volumes: string_list(get(v, &["volumes"])),
                env_keys,
                code_files: n_files,
                declared_in: format!("{}:{}", f.path, line_of(f.text, &name)),
                explain,
            };
            // A later compose file (override) for the same service replaces it.
            drafts.insert(
                name,
                Draft {
                    svc,
                    hosts,
                    env,
                    depends,
                },
            );
        }
    }

    // Dockerfiles not used by any compose service: one service per Dockerfile.
    for (path, text) in &dockerfiles {
        if used_dockerfiles.contains(*path) {
            continue;
        }
        let ctx = dir_of(path).to_owned();
        let d = parse_dockerfile(text);
        let name = if ctx.is_empty() {
            "app".to_owned()
        } else {
            file_name(&ctx).to_owned()
        };
        if drafts.contains_key(&name) {
            continue;
        }
        sources.insert((*path).to_owned());
        let files = in_context(&ctx);
        let n = files.len() as u32;
        let kind = kind_from_code(&files, d.command.as_deref(), &name);
        let entry = d
            .command
            .as_deref()
            .and_then(|c| entry_file(c, &ctx, &code_set));
        drafts.insert(
            name.clone(),
            Draft {
                svc: Service {
                    id: format!("svc:{name}"),
                    name: name.clone(),
                    kind,
                    image: d.base.clone(),
                    build_context: Some(ctx),
                    command: d.command.clone(),
                    entry_file: entry,
                    ports: d.expose.clone(),
                    memory_limit: None,
                    memory_reservation: None,
                    cpu_limit: None,
                    gpu: false,
                    volumes: Vec::new(),
                    env_keys: Vec::new(),
                    code_files: n,
                    declared_in: format!("{path}:1"),
                    explain: kind_explain(kind, n),
                },
                hosts: BTreeSet::from([name]),
                env: Vec::new(),
                depends: Vec::new(),
            },
        );
    }

    if drafts.is_empty() {
        return None;
    }

    // Links: env values naming another service's host, depends_on, proxy_pass.
    let host_owner: BTreeMap<String, String> = drafts
        .values()
        .flat_map(|d| {
            d.hosts
                .iter()
                .map(move |h| (h.to_ascii_lowercase(), d.svc.id.clone()))
        })
        .collect();
    let mut links: BTreeMap<(String, String), ServiceLink> = BTreeMap::new();
    for d in drafts.values() {
        for (k, v) in &d.env {
            for c in HOST.captures_iter(v) {
                let host = c[1].to_ascii_lowercase();
                if let Some(to) = host_owner.get(&host)
                    && *to != d.svc.id
                {
                    let port = c
                        .get(2)
                        .map(|p| format!(":{}", p.as_str()))
                        .unwrap_or_default();
                    links
                        .entry((d.svc.id.clone(), to.clone()))
                        .or_insert_with(|| ServiceLink {
                            from: d.svc.id.clone(),
                            to: to.clone(),
                            via: LinkVia::Env,
                            detail: format!("{k} -> {host}{port}"),
                        });
                }
            }
        }
        for dep in &d.depends {
            if let Some(to) = host_owner.get(&dep.to_ascii_lowercase())
                && *to != d.svc.id
            {
                links
                    .entry((d.svc.id.clone(), to.clone()))
                    .or_insert_with(|| ServiceLink {
                        from: d.svc.id.clone(),
                        to: to.clone(),
                        via: LinkVia::DependsOn,
                        detail: "depends_on".to_owned(),
                    });
            }
        }
    }
    for f in infra.iter().filter(|f| is_proxy_conf(f.path)) {
        let proxy = drafts
            .values()
            .find(|d| d.svc.kind == ServiceKind::Proxy)
            .map(|d| d.svc.id.clone());
        let Some(from) = proxy else { continue };
        for c in PROXY_PASS.captures_iter(f.text) {
            if let Some(to) = host_owner.get(&c[1].to_ascii_lowercase())
                && *to != from
            {
                sources.insert(f.path.to_owned());
                links.insert(
                    (from.clone(), to.clone()),
                    ServiceLink {
                        from: from.clone(),
                        to: to.clone(),
                        via: LinkVia::Proxy,
                        detail: format!("proxy_pass in {}", f.path),
                    },
                );
            }
        }
    }

    let mut services: Vec<Service> = drafts.into_values().map(|d| d.svc).collect();
    services.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.name.cmp(&b.name)));
    Some(Deployment {
        sources: sources.into_iter().collect(),
        services,
        links: links.into_values().collect(),
    })
}
