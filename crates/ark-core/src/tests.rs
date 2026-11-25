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
