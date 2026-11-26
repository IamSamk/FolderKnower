use super::*;
use std::path::PathBuf;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/repos/fullstack-todo")
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()));
        } else {
            fs::copy(&p, to.join(e.file_name())).unwrap();
        }
    }
}

fn run(root: &Path, cache: &Path) -> (String, RunStats) {
    let opts = Options {
        cache_path: Some(cache.to_path_buf()),
        ..Options::default()
    };
    let (g, s) = analyze(root, &opts, &|_| {}).unwrap();
    (ark_schema::to_canonical_json(&g).unwrap(), s)
}

#[test]
fn fullstack_fixture_end_to_end() {
    let tmp = tempfile::tempdir().unwrap();
    let opts = Options {
        cache_path: Some(tmp.path().join("c.sqlite")),
        ..Options::default()
    };
    let (g, _) = analyze(&fixture(), &opts, &|_| {}).unwrap();
    assert_eq!(g.summary.kind, ark_schema::ProjectKind::FullStack);
    let tier_of = |path: &str| {
        g.tiers
            .iter()
            .find(|t| {
                t.clusters
                    .iter()
                    .any(|c| c.files.iter().any(|f| f.path == path))
            })
            .map(|t| t.label)
    };
    assert_eq!(
        tier_of("web/src/components/TodoList.tsx"),
        Some(TierLabel::Frontend)
    );
    assert_eq!(
        tier_of("api/app/routers/todos.py"),
        Some(TierLabel::Backend)
    );
    assert_eq!(tier_of("api/app/models.py"), Some(TierLabel::Storage));
    assert_eq!(tier_of("worker/tasks.py"), Some(TierLabel::Workers));
    assert_eq!(
        tier_of("api/app/core/security.py"),
        Some(TierLabel::Gateway)
    );

    // Button -> handler -> api client -> route -> table
    let add = g
        .ui_elements
        .iter()
        .find(|u| u.label.as_deref() == Some("Add todo"))
        .unwrap();
    assert_eq!(
        add.handler.as_deref(),
        Some("fn:web/src/components/TodoList.tsx#handleAdd")
    );
    let x = |s: &str, t: &str| {
        g.cross_tier_edges
            .iter()
            .any(|e| e.source == s && e.target == t)
    };
    assert!(x(
        "fn:web/src/lib/api.ts#createTodo",
        "fn:api/app/routers/todos.py#create_todo"
    ));
    assert!(x(
        "fn:web/src/lib/api.ts#completeTodo",
        "fn:api/app/routers/todos.py#complete_todo"
    ));
    assert!(x(
        "fn:web/src/lib/api.ts#login",
        "fn:api/app/routers/auth.py#login"
    ));
    assert!(x(
        "fn:api/app/routers/todos.py#create_todo",
        "model:api/app/models.py#Todo"
    ));
    let create = g
        .tiers
        .iter()
        .flat_map(|t| &t.clusters)
        .flat_map(|c| &c.files)
        .flat_map(|f| &f.functions)
        .find(|f| f.id == "fn:api/app/routers/todos.py#create_todo")
        .unwrap();
    assert!(
        create
            .calls
            .iter()
            .any(|e| e.target == "fn:worker/tasks.py#send_digest")
    );
    assert!(g.data_models.iter().any(|m| m.name == "audit_log"));
    assert!(g.non_code_nodes.iter().any(|n| {
        n.kind == ark_schema::NonCodeKind::Test
            && n.relates_to
                .contains(&"file:api/app/routers/todos.py".to_owned())
    }));
}

#[test]
fn deterministic_and_incremental() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    copy_dir(&fixture(), &repo);
    let cache = tmp.path().join("c.sqlite");

    let (a, s1) = run(&repo, &cache);
    assert_eq!(s1.cached, 0);
    let (b, s2) = run(&repo, &cache);
    assert_eq!(a, b, "byte-identical across runs");
    assert_eq!(s2.parsed, 0, "second run re-parses nothing");

    // Cold run with no cache gives the same bytes.
    let (c, _) = analyze(
        &repo,
        &Options {
            no_cache: true,
            ..Options::default()
        },
        &|_| {},
    )
    .unwrap();
    assert_eq!(a, ark_schema::to_canonical_json(&c).unwrap());

    fs::write(repo.join("worker/tasks.py"), "def other():\n    pass\n").unwrap();
    let (_, s3) = run(&repo, &cache);
    assert_eq!(s3.parsed, 1, "one edit re-parses exactly one file");
}

/// Golden snapshot. Regenerate with `UPDATE_GOLDEN=1 cargo test -p ark-core golden`.
#[test]
fn golden_fullstack_todo() {
    golden("fullstack-todo");
}

#[test]
fn golden_rag_agent() {
    golden("rag-agent");
}

#[test]
fn golden_graph_rag() {
    golden("graph-rag");
}

#[test]
fn graph_rag_run_walks_both_lanes_before_fusion() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/repos/graph-rag");
    let (g, _) = analyze(
        &dir,
        &Options {
            no_cache: true,
            ..Options::default()
        },
        &|_| {},
    )
    .unwrap();
    let runs = crate::story::example_runs(&g);
    let run = runs.first().expect("the Ask button reaches the backend");
    assert_eq!(run.title, "Ask → POST /api/ask");
    let at = |name: &str| {
        run.hops
            .iter()
            .position(|h| h.label == name)
            .unwrap_or_else(|| panic!("{name} missing from the run"))
    };
    let fuse = at("rrf_fuse");
    assert!(at("lexical_search") < fuse && at("expand_entities") < fuse);
    assert!(at("pgvector") < fuse && at("bge-m3-Q8") < at("pgvector"));
    assert!(fuse < at("CrossEncoder"));
    assert!(at("CrossEncoder") < at("evidence_is_sufficient"));
    assert!(at("evidence_is_sufficient") < at("Qwen3-27B-Q4_K_M.gguf"));
}

fn golden(name: &str) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/repos")
        .join(name);
    let (g, _) = analyze(
        &dir,
        &Options {
            no_cache: true,
            ..Options::default()
        },
        &|_| {},
    )
    .unwrap();
    let mut g = g;
    g.repo_name = name.to_owned();
    g.generator.version = "golden".to_owned();
    let json = ark_schema::to_canonical_json(&g).unwrap();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../fixtures/expected/{name}.json"));
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        fs::write(&path, &json).unwrap();
    }
    let expected = fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
    assert!(
        expected == json,
        "golden mismatch; run with UPDATE_GOLDEN=1 and review the diff"
    );
}

#[test]
fn rag_agent_example_run_is_grounded() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/repos/rag-agent");
    let opts = Options {
        no_cache: true,
        ..Options::default()
    };
    let (g, _) = analyze(&dir, &opts, &|_| {}).unwrap();
    assert!(
        g.summary.headline.starts_with("AI system in 6 containers"),
        "{}",
        g.summary.headline
    );
    let run = g.example_runs.first().expect("a run");
    use ark_schema::RunHopKind as K;
    let kinds: Vec<K> = run.hops.iter().map(|h| h.kind).collect();
    let pos = |k: K| kinds.iter().position(|x| *x == k).unwrap();
    assert!(pos(K::Ui) < pos(K::Proxy) && pos(K::Proxy) < pos(K::Route));
    assert_eq!(kinds.last(), Some(&K::Response));
    let svcs: Vec<&str> = run
        .hops
        .iter()
        .filter(|h| h.kind == K::Service)
        .map(|h| h.node.as_str())
        .collect();
    assert_eq!(svcs, ["svc:cache", "svc:chroma", "svc:ollama"]);
    for h in &run.hops {
        assert!(!h.proof.is_empty(), "hop {} has no proof", h.label);
        assert!(h.confidence > 0.0 && h.confidence <= 1.0);
        if let Some(s) = &h.service {
            let d = g.insights.as_ref().unwrap().deployment.as_ref().unwrap();
            assert!(d.services.iter().any(|x| &x.id == s), "unknown service {s}");
        }
    }
    assert_eq!(run.sample_input.as_deref(), Some("What is chunk overlap?"));
}

#[test]
fn golden_broken_shop() {
    golden("broken-shop");
}

#[test]
fn golden_tauri_notes() {
    golden("tauri-notes");
}

#[test]
fn desktop_app_commands_features_and_storage() {
    use ark_schema::{HealthKind as K, MemoryPlace as P, Protocol, TierLabel};
    let g = analyze_fixture("tauri-notes");
    assert_eq!(g.summary.kind, ark_schema::ProjectKind::Desktop);
    assert!(
        g.summary.headline.starts_with("Desktop app (Tauri)"),
        "{}",
        g.summary.headline
    );
    let tier_of = |path: &str| {
        g.tiers
            .iter()
            .find(|t| {
                t.clusters
                    .iter()
                    .any(|c| c.files.iter().any(|f| f.path == path))
            })
            .map(|t| t.label)
    };
    assert_eq!(tier_of("src-tauri/src/lib.rs"), Some(TierLabel::Backend));
    assert_eq!(tier_of("src/App.tsx"), Some(TierLabel::Frontend));
    // The window's invoke() calls reach the Rust commands.
    let ipc: Vec<(&str, &str)> = g
        .cross_tier_edges
        .iter()
        .filter(|e| e.protocol == Protocol::Ipc)
        .map(|e| (e.source.as_str(), e.target.as_str()))
        .collect();
    assert!(ipc.contains(&(
        "fn:src/api.ts#saveNote",
        "fn:src-tauri/src/lib.rs#save_note"
    )));
    assert!(ipc.contains(&(
        "fn:src/api.ts#listNotes",
        "fn:src-tauri/src/lib.rs#list_notes"
    )));
    // `use notes_core::...` resolves to the workspace crate.
    let lib = g
        .tiers
        .iter()
        .flat_map(|t| &t.clusters)
        .flat_map(|c| &c.files)
        .find(|f| f.path == "src-tauri/src/lib.rs")
        .unwrap();
    assert!(
        lib.imports
            .iter()
            .any(|e| e.target == "file:crates/notes-core/src/lib.rs")
    );
    // Commands group into features by shared words: notes vs app version.
    let names: Vec<&str> = g.features.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["Notes", "App Version"]);
    // Where the data lives.
    let has = |what: &str, place: P| {
        g.memory_map
            .iter()
            .any(|m| m.what == what && m.place == place)
    };
    assert!(has("SQLite database file", P::Disk), "{:#?}", g.memory_map);
    assert!(has("App data folder", P::Disk));
    assert!(has("Shared app state", P::Ram));
    // One real problem (the typo), and no false alarm for `export type *` or `Notes & Ideas`.
    let h = g.health.expect("the typo");
    assert_eq!(h.issues.len(), 1, "{:#?}", h.issues);
    assert_eq!(h.issues[0].kind, K::DeadRoute);
    assert_eq!(
        (h.issues[0].file_id.as_str(), h.issues[0].line),
        ("file:src/api.ts", 13)
    );
}

#[test]
fn test_fixtures_inside_a_repo_are_not_mapped() {
    let tmp = tempfile::tempdir().unwrap();
    copy_dir(&fixture(), tmp.path());
    copy_dir(&fixture(), &tmp.path().join("tests/fixtures/sample"));
    let opts = Options {
        no_cache: true,
        ..Options::default()
    };
    let (g, _) = analyze(tmp.path(), &opts, &|_| {}).unwrap();
    let all: Vec<&str> = g
        .tiers
        .iter()
        .flat_map(|t| &t.clusters)
        .flat_map(|c| &c.files)
        .map(|f| f.path.as_str())
        .collect();
    assert!(!all.iter().any(|p| p.contains("fixtures/")), "{all:?}");
    assert!(!all.is_empty());
}

fn analyze_fixture(name: &str) -> SystemArchitectureGraph {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/repos")
        .join(name);
    let opts = Options {
        no_cache: true,
        ..Options::default()
    };
    analyze(&dir, &opts, &|_| {}).unwrap().0
}

#[test]
fn broken_code_is_reported_and_still_mapped() {
    use ark_schema::{HealthKind as K, Severity};
    let g = analyze_fixture("broken-shop");
    let h = g.health.as_ref().expect("health issues");
    let has = |k: K, file: &str, line: u32| {
        h.issues
            .iter()
            .any(|i| i.kind == k && i.file_id == format!("file:{file}") && i.line == line)
    };
    assert!(
        has(K::SyntaxError, "server/pricing.py", 9),
        "{:#?}",
        h.issues
    );
    assert!(has(K::UnresolvedImport, "server/api.py", 6));
    assert!(has(K::UnresolvedImport, "web/src/api.ts", 2));
    assert!(has(K::MissingPackage, "server/api.py", 3));
    assert!(has(K::MissingPackage, "web/src/api.ts", 1));
    assert!(has(K::DeadRoute, "web/src/api.ts", 10));
    assert!(has(K::MissingEntry, "docker-compose.yml", 2));
    assert_eq!(h.issues.len(), 7, "no false positives: {:#?}", h.issues);
    assert_eq!(h.files_with_errors, 1);
    // Declared packages, stdlib and the working files are not flagged.
    assert!(!h.issues.iter().any(|i| i.message.contains("fastapi")));
    assert!(
        h.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count()
            == 4
    );
    // The broken file is still parsed: the function before the error is in the map.
    let pricing = g
        .tiers
        .iter()
        .flat_map(|t| &t.clusters)
        .flat_map(|c| &c.files)
        .find(|f| f.path == "server/pricing.py")
        .expect("pricing.py mapped");
    assert!(pricing.functions.iter().any(|f| f.name == "total"));
}

#[test]
fn healthy_fixtures_have_no_false_alarms() {
    let g = analyze_fixture("fullstack-todo");
    assert!(g.health.is_none(), "{:#?}", g.health);
    // rag-agent runs vLLM inside the api container, which reserves no GPU: that one
    // warning is real, and it is the only one.
    let g = analyze_fixture("rag-agent");
    let h = g.health.expect("one warning");
    assert_eq!(h.issues.len(), 1, "{:#?}", h.issues);
    let i = &h.issues[0];
    assert_eq!(i.kind, ark_schema::HealthKind::NoGpu);
    assert_eq!((i.file_id.as_str(), i.line), ("file:app/serve.py", 4));
}

#[test]
fn features_split_by_route_family() {
    let g = analyze_fixture("fullstack-todo");
    let names: Vec<&str> = g.features.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["Todos", "Sign in"]);
    let todos = &g.features[0];
    assert_eq!(todos.routes.len(), 3);
    assert!(
        todos
            .files
            .contains(&"file:api/app/routers/todos.py".to_owned())
    );
    assert!(
        todos
            .tables
            .contains(&"model:api/app/models.py#Todo".to_owned())
    );
    let auth = &g.features[1];
    assert!(
        auth.tables
            .contains(&"model:api/app/models.py#User".to_owned())
    );
    assert!(
        auth.files
            .contains(&"file:web/src/components/LoginForm.tsx".to_owned())
    );
    // The API client is used by both, so it is shared, not owned.
    assert!(
        todos
            .shared_files
            .contains(&"file:web/src/lib/api.ts".to_owned())
    );
    assert!(
        auth.shared_files
            .contains(&"file:web/src/lib/api.ts".to_owned())
    );
    assert!(todos.stack.iter().any(|s| s == "FastAPI"));
}

#[test]
fn rag_features_keep_indexing_apart_from_asking() {
    let g = analyze_fixture("rag-agent");
    let f = |n: &str| g.features.iter().find(|f| f.name == n).expect(n);
    let ask = f("Ask");
    let ingest = f("Ingest");
    assert!(ask.ai_stages.iter().any(|s| s.contains("reranker")));
    assert!(!ask.ai_stages.iter().any(|s| s.contains("loader")));
    assert!(ingest.ai_stages.iter().any(|s| s.contains("loader")));
    assert!(!ingest.ai_stages.iter().any(|s| s.contains("reranker")));
    assert!(ask.services.contains(&"svc:ollama".to_owned()));
    assert!(ask.services.contains(&"svc:cache".to_owned()));
    assert!(!ingest.services.contains(&"svc:cache".to_owned()));
    assert!(f("History").ai_stages.is_empty());
}

#[test]
fn memory_map_places_each_store() {
    use ark_schema::{MemoryLifetime as L, MemoryPlace as P};
    let g = analyze_fixture("rag-agent");
    let find = |holder: &str, what: &str| {
        g.memory_map
            .iter()
            .find(|m| m.holder == holder && m.what.starts_with(what))
            .unwrap_or_else(|| panic!("{holder} {what}: {:#?}", g.memory_map))
    };
    let w = find("svc:ollama", "Model weights");
    assert_eq!(w.place, P::Vram, "ollama has a GPU reservation");
    assert!(w.bound.as_deref().unwrap_or("").contains("keep_alive"));
    let h = find("svc:cache", "Chat history");
    assert_eq!((h.place, h.lifetime), (P::Ram, L::Session));
    assert!(h.bound.as_deref().unwrap_or("").contains("maxmemory 256mb"));
    assert_eq!(find("svc:chroma", "Vector index").place, P::Ram);
    assert_eq!(
        find("svc:chroma", "Stored collections").lifetime,
        L::Persistent
    );
    assert_eq!(
        find("svc:api", "Embedding model").place,
        P::Ram,
        "api has no GPU"
    );
    assert_eq!(
        find("svc:api", "KV cache").place,
        P::Vram,
        "vLLM runs on the GPU"
    );
    assert_eq!(find("browser", "localStorage").lifetime, L::Persistent);
    for m in &g.memory_map {
        assert!(!m.proof.is_empty(), "{} has no proof", m.what);
    }
}
