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
    /// Print the JSON Schema of architecture.json.
    Schema,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.cmd {
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
