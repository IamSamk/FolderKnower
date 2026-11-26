//! `architekton` command-line indexer.

use std::{collections::BTreeMap, path::PathBuf, process::ExitCode};

use ark_core::{Options, analyze, ark_schema};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "architekton",
    version,
    about = "Offline, deterministic codebase architecture indexer"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Index a repository and write architecture.json.
    Index {
        /// Repository root.
        path: PathBuf,
        /// Output file ("-" for stdout).
        #[arg(short, long, default_value = "architecture.json")]
        out: String,
        /// Skip the incremental cache.
        #[arg(long)]
        no_cache: bool,
        /// Cache file location (default: per-user cache dir).
        #[arg(long)]
        cache: Option<PathBuf>,
        /// Force a tier for a path prefix, e.g. `--set scripts/=Workers`.
        #[arg(long = "set", value_name = "PREFIX=TIER")]
        overrides: Vec<String>,
        /// Do not print progress.
        #[arg(short, long)]
        quiet: bool,
    },
    /// Grounded example run: what happens, hop by hop, when a user uses a UI element.
    ///
    /// Built for AI narrators as a tool call: every hop carries the `path:line` or the
    /// compose link that proves it, and nothing outside the graph is ever returned.
    Run {
        /// Repository root, or an architecture.json written by `index`.
        path: PathBuf,
        /// UI element id (or part of it, or its label) to start from; default: deepest run.
        #[arg(long)]
        from: Option<String>,
        /// List the available runs (id, title, hop count) instead of printing one.
        #[arg(long)]
        list: bool,
        /// Print a numbered plain-text story instead of JSON.
        #[arg(long)]
        text: bool,
    },
    /// Find what is broken without running anything: syntax errors, imports of missing
    /// files, packages missing from the manifest, calls to undeclared routes, start
    /// commands that name a missing file. Exits 1 when any error is found.
    Check {
        /// Repository root.
        path: PathBuf,
        /// Print JSON instead of one line per issue.
        #[arg(long)]
        json: bool,
    },
    /// Print the JSON Schema of architecture.json.
    Schema,
    /// Print the Systems Ontology catalogue (concepts + teaching text) as JSON.
    Ontology,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Ontology => {
            let rules: Vec<serde_json::Value> = ark_ontology::Ontology::load()
                .rules
                .into_iter()
                .map(|r| serde_json::json!({"id": r.id, "layer": r.layer, "concept": r.concept, "explain": r.explain}))
                .collect();
            println!("{}", serde_json::to_string_pretty(&rules).expect("json"));
            ExitCode::SUCCESS
        }
        Cmd::Run {
            path,
            from,
            list,
            text,
        } => run_cmd(&path, from.as_deref(), list, text),
        Cmd::Check { path, json } => check_cmd(&path, json),
        Cmd::Schema => {
            print!("{}", ark_schema::json_schema_string());
            ExitCode::SUCCESS
        }
        Cmd::Index {
            path,
            out,
            no_cache,
            cache,
            overrides,
            quiet,
        } => {
            let mut map = BTreeMap::new();
            for o in overrides {
                let Some((p, t)) = o.split_once('=') else {
                    eprintln!("--set expects PREFIX=TIER, got {o}");
                    return ExitCode::from(2);
                };
                let tier: ark_schema::TierLabel = match serde_json::from_value(
                    serde_json::Value::String(t.to_owned()),
                ) {
                    Ok(t) => t,
                    Err(_) => {
                        eprintln!(
                            "unknown tier {t}; use Frontend|Gateway|Backend|Storage|Workers|Shared"
                        );
                        return ExitCode::from(2);
                    }
                };
                map.insert(p.to_owned(), tier);
            }
            let opts = Options {
                cache_path: cache,
                no_cache,
                overrides: map,
                ..Options::default()
            };
            let progress = |p: ark_core::Progress| {
                if !quiet {
                    eprintln!("[{:?}] {}", p.stage, p.message);
                }
            };
            match analyze(&path, &opts, &progress) {
                Ok((graph, stats)) => {
                    let json = ark_schema::to_canonical_json(&graph).expect("serializable");
                    if out == "-" {
                        print!("{json}");
                    } else if let Err(e) = std::fs::write(&out, json) {
                        eprintln!("cannot write {out}: {e}");
                        return ExitCode::FAILURE;
                    }
                    if !quiet {
                        eprintln!(
                            "{} | {} files, {} parsed, {} cached, {} ms -> {}",
                            graph.summary.headline,
                            stats.files,
                            stats.parsed,
                            stats.cached,
                            stats.millis,
                            out
                        );
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

fn check_cmd(path: &std::path::Path, json: bool) -> ExitCode {
    let opts = Options {
        no_cache: true,
        ..Options::default()
    };
    let g = match analyze(path, &opts, &|_| {}) {
        Ok((g, _)) => g,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let Some(h) = g.health else {
        if json {
            println!("{{\"issues\": []}}");
        } else {
            println!("No problems found.");
        }
        return ExitCode::SUCCESS;
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&h).expect("json"));
    } else {
        for i in &h.issues {
            let sev = match i.severity {
                ark_schema::Severity::Error => "error",
                ark_schema::Severity::Warning => "warning",
            };
            println!(
                "{}:{}: {sev}: {}\n    fix: {}",
                i.file_id.trim_start_matches("file:"),
                i.line,
                i.message,
                i.fix
            );
        }
        println!(
            "{} issue(s); {} of {} files have syntax errors.",
            h.issues.len(),
            h.files_with_errors,
            h.files_checked
        );
    }
    if h.issues
        .iter()
        .any(|i| i.severity == ark_schema::Severity::Error)
    {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn run_cmd(path: &std::path::Path, from: Option<&str>, list: bool, text: bool) -> ExitCode {
    let graph: ark_schema::SystemArchitectureGraph = if path.is_dir() {
        match analyze(path, &Options::default(), &|_| {}) {
            Ok((g, _)) => g,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        match std::fs::read_to_string(path).map(|s| serde_json::from_str(&s)) {
            Ok(Ok(g)) => g,
            Ok(Err(e)) => {
                eprintln!("{} is not an architecture.json: {e}", path.display());
                return ExitCode::from(2);
            }
            Err(e) => {
                eprintln!("cannot read {}: {e}", path.display());
                return ExitCode::FAILURE;
            }
        }
    };
    let runs = &graph.example_runs;
    if list {
        let rows: Vec<serde_json::Value> = runs
            .iter()
            .map(|r| serde_json::json!({"id": r.id, "title": r.title, "start": r.start, "hops": r.hops.len()}))
            .collect();
        println!("{}", serde_json::to_string_pretty(&rows).expect("json"));
        return ExitCode::SUCCESS;
    }
    let pick = match from {
        None => runs.first(),
        Some(q) => runs.iter().find(|r| {
            r.start == q
                || r.id == q
                || r.start.contains(q)
                || r.title.to_lowercase().contains(&q.to_lowercase())
        }),
    };
    let Some(run) = pick else {
        eprintln!(
            "no example run{}; a run needs a UI element whose handler reaches a backend route",
            from.map(|q| format!(" matches {q:?}")).unwrap_or_default()
        );
        return ExitCode::from(3);
    };
    if text {
        println!("{}", run.title);
        for (i, h) in run.hops.iter().enumerate() {
            println!(
                "{:>2}. [{}] {} - {} (proof: {})",
                i + 1,
                h.service
                    .as_deref()
                    .unwrap_or("-")
                    .trim_start_matches("svc:"),
                h.label,
                h.explain,
                h.proof.join(", ")
            );
        }
    } else {
        println!("{}", serde_json::to_string_pretty(run).expect("json"));
    }
    ExitCode::SUCCESS
}
