//! What is broken, found without running anything: syntax errors, relative imports
//! that point at no file, packages the code imports but no manifest declares, same-origin
//! HTTP calls that match no route, and start commands whose program is not in the repo.
//! Every issue names a file and a line. Package checks are skipped when a language has
//! no manifest, so an unpackaged script is never flagged.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use ark_link::{Linked, ParsedFile, file_id};
use ark_scan::{Language, ScannedFile};
use ark_schema::{CodeHealth, HealthIssue, HealthKind, Service, Severity};

const PY_STDLIB: &str = "__future__ abc antigravity argparse array ast asyncio atexit base64 bdb binascii bisect builtins bz2 cProfile calendar cmath cmd code codecs codeop collections colorsys compileall concurrent configparser contextlib contextvars copy copyreg csv ctypes curses dataclasses datetime dbm decimal difflib dis doctest email encodings ensurepip enum errno faulthandler fcntl filecmp fileinput fnmatch fractions ftplib functools gc genericpath getopt getpass gettext glob graphlib grp gzip hashlib heapq hmac html http idlelib imaplib importlib inspect io ipaddress itertools json keyword linecache locale logging lzma mailbox marshal math mimetypes mmap modulefinder msvcrt multiprocessing netrc nt ntpath nturl2path numbers opcode operator optparse os pathlib pdb pickle pickletools pkgutil platform plistlib poplib posix posixpath pprint profile pstats pty pwd py_compile pyclbr pydoc pydoc_data pyexpat queue quopri random re readline reprlib resource rlcompleter runpy sched secrets select selectors shelve shlex shutil signal site smtplib socket socketserver sqlite3 sre_compile sre_constants sre_parse ssl stat statistics string stringprep struct subprocess symtable sys sysconfig syslog tabnanny tarfile tempfile termios textwrap this threading time timeit tkinter token tokenize tomllib trace traceback tracemalloc tty turtle turtledemo types typing unicodedata unittest urllib uuid venv warnings wave weakref webbrowser winreg winsound wsgiref xml xmlrpc zipapp zipfile zipimport zlib zoneinfo";

const NODE_BUILTINS: &[&str] = &[
    "assert",
    "async_hooks",
    "buffer",
    "child_process",
    "cluster",
    "console",
    "constants",
    "crypto",
    "dgram",
    "diagnostics_channel",
    "dns",
    "domain",
    "events",
    "fs",
    "http",
    "http2",
    "https",
    "inspector",
    "module",
    "net",
    "os",
    "path",
    "perf_hooks",
    "process",
    "punycode",
    "querystring",
    "readline",
    "repl",
    "stream",
    "string_decoder",
    "sys",
    "timers",
    "tls",
    "trace_events",
    "tty",
    "url",
    "util",
    "v8",
    "vm",
    "wasi",
    "worker_threads",
    "zlib",
    "test",
    "sqlite",
];

/// Import name -> distribution names that provide it, where they differ.
const PY_ALIASES: &[(&str, &[&str])] = &[
    ("yaml", &["pyyaml"]),
    ("sklearn", &["scikit_learn"]),
    ("skimage", &["scikit_image"]),
    (
        "cv2",
        &[
            "opencv_python",
            "opencv_python_headless",
            "opencv_contrib_python",
        ],
    ),
    ("PIL", &["pillow"]),
    ("bs4", &["beautifulsoup4"]),
    ("dotenv", &["python_dotenv"]),
    ("jwt", &["pyjwt"]),
    ("jose", &["python_jose"]),
    ("dateutil", &["python_dateutil"]),
    ("attr", &["attrs"]),
    ("serial", &["pyserial"]),
    ("fitz", &["pymupdf"]),
    ("docx", &["python_docx"]),
    ("pptx", &["python_pptx"]),
    ("Crypto", &["pycryptodome", "pycryptodomex"]),
    ("OpenSSL", &["pyopenssl"]),
    ("zmq", &["pyzmq"]),
    ("magic", &["python_magic"]),
    ("multipart", &["python_multipart"]),
    ("psycopg2", &["psycopg2_binary"]),
    (
        "google",
        &[
            "google_cloud",
            "google_genai",
            "google_generativeai",
            "google_adk",
            "protobuf",
            "google_api_python_client",
        ],
    ),
    ("faiss", &["faiss_cpu", "faiss_gpu"]),
    ("llama_cpp", &["llama_cpp_python"]),
    ("pydantic_settings", &["pydantic_settings"]),
    ("FlagEmbedding", &["flagembedding"]),
    ("rank_bm25", &["rank_bm25"]),
    ("pymilvus", &["pymilvus"]),
    ("mcp", &["mcp", "fastmcp"]),
    ("agents", &["openai_agents"]),
    ("tiktoken", &["tiktoken"]),
    ("websocket", &["websocket_client"]),
    ("git", &["gitpython"]),
    ("redis", &["redis", "redis_om"]),
];

/// Packages a declared one brings along, so importing them is not an error.
const IMPLIED: &[(&str, &[&str])] = &[
    ("fastapi", &["pydantic", "starlette", "anyio"]),
    ("langchain", &["langchain_core"]),
    ("langchain_community", &["langchain_core"]),
    ("langgraph", &["langchain_core"]),
    (
        "transformers",
        &["tokenizers", "huggingface_hub", "safetensors"],
    ),
    (
        "sentence_transformers",
        &["transformers", "torch", "huggingface_hub", "numpy"],
    ),
    ("vllm", &["torch", "transformers"]),
    ("torch", &["numpy"]),
    ("pandas", &["numpy"]),
    ("scikit_learn", &["numpy", "scipy"]),
    ("flask", &["werkzeug", "jinja2", "click"]),
    ("celery", &["kombu"]),
    ("requests", &["urllib3", "idna", "certifi"]),
    ("uvicorn", &["click"]),
    ("sqlmodel", &["sqlalchemy", "pydantic"]),
    ("pydantic_settings", &["pydantic"]),
];

/// Standard library module or Node built-in.
pub fn is_stdlib(p: &str) -> bool {
    let bare = p.trim_start_matches("node:");
    PY_STDLIB.split(' ').any(|s| s == p) || NODE_BUILTINS.contains(&bare) || p.starts_with("node:")
}

fn norm_py(s: &str) -> String {
    s.trim().to_ascii_lowercase().replace(['-', '.'], "_")
}

#[derive(Default)]
struct Manifests {
    /// dir -> declared names, per ecosystem.
    js: BTreeMap<String, BTreeSet<String>>,
    py: BTreeMap<String, BTreeSet<String>>,
    /// tsconfig/jsconfig path alias prefixes (`@app`, `~`), any directory.
    js_aliases: BTreeSet<String>,
}

fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
}

fn read_manifests(root: &Path, files: &[ScannedFile]) -> Manifests {
    let mut m = Manifests::default();
    for f in files {
        if f.size > 512 * 1024 {
            continue;
        }
        let name = f.path.rsplit('/').next().unwrap_or(&f.path);
        let dir = dir_of(&f.path).to_owned();
        let read = || fs::read_to_string(root.join(&f.path)).ok();
        if name == "package.json" {
            let Some(t) = read() else { continue };
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) else {
                continue;
            };
            let set = m.js.entry(dir).or_default();
            if let Some(n) = v.get("name").and_then(|n| n.as_str()) {
                set.insert(n.to_owned());
            }
            for k in [
                "dependencies",
                "devDependencies",
                "peerDependencies",
                "optionalDependencies",
            ] {
                if let Some(o) = v.get(k).and_then(|o| o.as_object()) {
                    set.extend(o.keys().cloned());
                }
            }
        } else if (name.starts_with("tsconfig") || name.starts_with("jsconfig"))
            && name.ends_with(".json")
        {
            let Some(t) = read() else { continue };
            // tsconfig allows comments; pick alias keys out of the "paths" block by text.
            if let Some(i) = t.find("\"paths\"") {
                let rest = &t[i..];
                let end = rest.find('}').unwrap_or(rest.len());
                for part in rest[..end].split('"').skip(3).step_by(2) {
                    let p = part.trim_end_matches("/*").trim_end_matches('*');
                    if !p.is_empty() && !p.contains(' ') && !p.starts_with('.') {
                        m.js_aliases
                            .insert(p.split('/').next().unwrap_or(p).to_owned());
                    }
                }
            }
        } else if name.starts_with("requirements") && name.ends_with(".txt") {
            let Some(t) = read() else { continue };
            let set = m.py.entry(dir).or_default();
            for l in t.lines() {
                let l = l.split('#').next().unwrap_or("").trim();
                if l.is_empty() || l.starts_with('-') {
                    continue;
                }
                let n: String = l
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
                    .collect();
                if !n.is_empty() {
                    set.insert(norm_py(&n));
                }
            }
        } else if name == "pyproject.toml" || name == "setup.py" || name == "Pipfile" {
            let Some(t) = read() else { continue };
            let set = m.py.entry(dir).or_default();
            // Quoted requirement strings anywhere, plus `name = ...` keys in dependency tables.
            let mut in_table = false;
            for l in t.lines() {
                let tl = l.trim();
                if tl.starts_with('[') {
                    in_table =
                        tl.contains("dependencies") || tl == "[packages]" || tl == "[dev-packages]";
                    continue;
                }
                for q in tl.split(['"', '\'']).skip(1).step_by(2) {
                    let n: String = q
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
                        .collect();
                    if !n.is_empty() && n.len() < 60 {
                        set.insert(norm_py(&n));
                    }
                }
                if in_table && let Some((k, _)) = tl.split_once('=') {
                    let k = k.trim().trim_matches('"');
                    if !k.is_empty() && !k.contains(' ') {
                        set.insert(norm_py(k));
                    }
                }
            }
        }
    }
    m
}

/// Declared names visible from a file: every manifest in its folder or a parent folder.
fn visible<'a>(
    map: &'a BTreeMap<String, BTreeSet<String>>,
    path: &str,
) -> Option<Vec<&'a BTreeSet<String>>> {
    let v: Vec<_> = map
        .iter()
        .filter(|(d, _)| d.is_empty() || path.starts_with(&format!("{d}/")))
        .map(|(_, s)| s)
        .collect();
    (!v.is_empty()).then_some(v)
}

fn py_declared(sets: &[&BTreeSet<String>], module: &str) -> bool {
    let n = norm_py(module);
    if sets.iter().any(|s| {
        IMPLIED
            .iter()
            .any(|(dep, gives)| s.contains(*dep) && gives.contains(&n.as_str()))
    }) {
        return true;
    }
    let mut names = vec![n.clone()];
    if let Some((_, al)) = PY_ALIASES.iter().find(|(k, _)| *k == module) {
        names.extend(al.iter().map(|s| s.to_string()));
    }
    sets.iter().any(|s| {
        names.iter().any(|n| s.contains(n))
            // `langchain_community` is provided by `langchain-community`, `qdrant_client` by
            // `qdrant-client`; also accept a declared name that is a prefix of the module
            // (`google_cloud_storage` -> `google`).
            || s.iter().any(|d| d.starts_with(&format!("{n}_")) || n.starts_with(&format!("{d}_")))
    })
}

pub fn check(
    root: &Path,
    scanned: &[ScannedFile],
    files: &[&ParsedFile],
    linked: &Linked,
    services: &[Service],
) -> Option<CodeHealth> {
    let mut issues: Vec<HealthIssue> = Vec::new();
    let mut push =
        |kind: HealthKind, severity, path: &str, line: u32, message: String, fix: String| {
            issues.push(HealthIssue {
                id: format!("health:{kind:?}:{path}:{line}:{}", issues.len()),
                kind,
                severity,
                file_id: file_id(path),
                line,
                message,
                fix,
            });
        };

    // Syntax errors.
    let mut broken = 0u32;
    for f in files {
        if f.facts.syntax_errors.is_empty() {
            continue;
        }
        broken += 1;
        for e in &f.facts.syntax_errors {
            push(
                HealthKind::SyntaxError,
                Severity::Error,
                &f.scan.path,
                e.line,
                format!("Syntax error: {}", e.text),
                "Fix this line first: the parser skipped the code around it, so functions here may be missing from the map.".into(),
            );
        }
    }

    // Relative imports to nowhere.
    for (path, list) in &linked.unresolved_imports {
        for (line, spec) in list {
            push(
                HealthKind::UnresolvedImport,
                Severity::Error,
                path,
                *line,
                format!("Import `{spec}` points at no file in the repository"),
                "Check the path and file name (case matters on Linux), or add the missing file."
                    .into(),
            );
        }
    }

    // Packages imported but not declared.
    let man = read_manifests(root, scanned);
    let py_std: BTreeSet<&str> = PY_STDLIB.split(' ').collect();
    let mut local_names: BTreeSet<String> = BTreeSet::new();
    for f in scanned {
        for seg in f.path.split('/') {
            let stem = seg.split('.').next().unwrap_or(seg);
            local_names.insert(stem.to_owned());
        }
    }
    for f in files {
        let Some(lang) = f.scan.language else {
            continue;
        };
        let path = f.scan.path.as_str();
        let Some(ext) = linked.externals.get(path) else {
            continue;
        };
        let first_line = |pkg: &str| {
            f.facts
                .imports
                .iter()
                .find(|i| {
                    let s = i.source.trim_start_matches("node:");
                    s == pkg
                        || s.starts_with(&format!("{pkg}/"))
                        || s.starts_with(&format!("{pkg}."))
                })
                .map(|i| i.line)
                .unwrap_or(1)
        };
        match lang {
            Language::Python => {
                let Some(sets) = visible(&man.py, path) else {
                    continue;
                };
                for pkg in ext {
                    if py_std.contains(pkg.as_str())
                        || local_names.contains(pkg)
                        || py_declared(&sets, pkg)
                    {
                        continue;
                    }
                    push(
                        HealthKind::MissingPackage,
                        Severity::Warning,
                        path,
                        first_line(pkg),
                        format!(
                            "`{pkg}` is imported but no requirements or pyproject file lists it"
                        ),
                        format!(
                            "Add the package that provides `{pkg}` to requirements.txt (or pyproject.toml), then reinstall."
                        ),
                    );
                }
            }
            Language::TypeScript | Language::Tsx | Language::JavaScript => {
                let Some(sets) = visible(&man.js, path) else {
                    continue;
                };
                for pkg in ext {
                    let bare = pkg.trim_start_matches("node:");
                    if NODE_BUILTINS.contains(&bare)
                        || bare.contains(':')
                        || bare.starts_with("bun")
                        || man
                            .js_aliases
                            .contains(bare.split('/').next().unwrap_or(bare))
                        || local_names.contains(bare)
                        || sets
                            .iter()
                            .any(|s| s.contains(bare) || s.contains(&format!("@types/{bare}")))
                    {
                        continue;
                    }
                    push(
                        HealthKind::MissingPackage,
                        Severity::Warning,
                        path,
                        first_line(bare),
                        format!("`{bare}` is imported but package.json does not list it"),
                        format!(
                            "Run `npm install {bare}` (or pnpm/yarn add) in the folder with package.json."
                        ),
                    );
                }
            }
            _ => {}
        }
    }

    // Same-origin calls that hit no route.
    for (path, line, method, url) in &linked.dead_calls {
        push(
            HealthKind::DeadRoute,
            Severity::Warning,
            path,
            *line,
            format!("{method} {url} matches no route declared in this repository"),
            "Check the URL and method against the backend routes; a typo or a removed route returns 404.".into(),
        );
    }

    // Start commands whose program is missing.
    let paths: BTreeSet<&str> = scanned.iter().map(|f| f.path.as_str()).collect();
    for s in services {
        let (Some(ctx), Some(cmd)) = (&s.build_context, &s.command) else {
            continue;
        };
        if s.entry_file.is_some() {
            continue;
        }
        let Some(target) = entry_target(cmd) else {
            continue;
        };
        let in_ctx = |p: &str| {
            if ctx.is_empty() {
                p.to_owned()
            } else {
                format!("{ctx}/{p}")
            }
        };
        let found = target.iter().any(|t| paths.contains(in_ctx(t).as_str()));
        if found {
            continue;
        }
        let (decl, line) = s
            .declared_in
            .rsplit_once(':')
            .and_then(|(p, l)| l.parse::<u32>().ok().map(|l| (p.to_owned(), l)))
            .unwrap_or((s.declared_in.clone(), 1));
        push(
            HealthKind::MissingEntry,
            Severity::Error,
            &decl,
            line,
            format!(
                "`{}` starts `{}`, but {} is not in the repository",
                s.name,
                cmd,
                in_ctx(&target[0])
            ),
            "Fix the module path in the start command, or add the file it names.".into(),
        );
    }

    issues.sort_by(|a, b| {
        (a.severity, a.kind, &a.file_id, a.line).cmp(&(b.severity, b.kind, &b.file_id, b.line))
    });
    for (i, x) in issues.iter_mut().enumerate() {
        x.id = format!("health:{i}");
    }
    let files_checked = files.len() as u32;
    (!issues.is_empty()).then_some(CodeHealth {
        files_checked,
        files_with_errors: broken,
        issues,
    })
}

/// Files a start command runs, relative to the build context: `uvicorn app.api:app`
/// -> `app/api.py` (or `app/api/__init__.py`), `python main.py`, `node server.js`.
fn entry_target(cmd: &str) -> Option<Vec<String>> {
    let toks: Vec<&str> = cmd.split_whitespace().collect();
    let pos = |name: &str| toks.iter().position(|t| t.ends_with(name));
    if let Some(i) = pos("uvicorn")
        .or_else(|| pos("gunicorn"))
        .or_else(|| pos("hypercorn"))
    {
        let m = toks[i + 1..]
            .iter()
            .find(|t| !t.starts_with('-') && t.contains(':'))?;
        let module = m.split(':').next()?.replace('.', "/");
        return Some(vec![
            format!("{module}.py"),
            format!("{module}/__init__.py"),
        ]);
    }
    if let Some(i) = pos("python").or_else(|| pos("python3")) {
        let next = toks.get(i + 1)?;
        if *next == "-m" {
            let module = toks.get(i + 2)?.replace('.', "/");
            return Some(vec![
                format!("{module}.py"),
                format!("{module}/__main__.py"),
            ]);
        }
        return next
            .ends_with(".py")
            .then(|| vec![next.trim_start_matches("./").to_owned()]);
    }
    if let Some(i) = pos("node") {
        let next = toks.get(i + 1)?;
        return (next.ends_with(".js") || next.ends_with(".mjs") || next.ends_with(".cjs"))
            .then(|| vec![next.trim_start_matches("./").to_owned()]);
    }
    None
}
