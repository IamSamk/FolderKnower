//! `ark-patterns`: deterministic recognizers that run over parsed facts.
//!
//! - [`dsa`]: stacks, queues, heaps, trees, graphs, recursion, memoization ...
//! - [`ai`]: AI/ML pipeline stages (loader, splitter, embedder, vector store, retriever,
//!   reranker, prompt, model, router, tools ...), their literal settings (chunk_size,
//!   top-k, model ids) and the data flow between them, plus memory techniques
//!   (PagedAttention, prefix/prompt caching, rolling caches, quantization) and
//!   named libraries/formats from `ontology/ai-catalog.yaml`.
//!
//! No model is involved: every finding carries the rule id that produced it.

pub mod ai;
pub mod catalog;
pub mod deployment;
pub mod dsa;

use ark_link::ParsedFile;
use ark_schema::Insights;
pub use deployment::{CodeFile, InfraFile};

/// Bump when rules or the catalog change meaningfully.
pub const RULESET_VERSION: &str = "patterns 0.1.0";

/// Run every recognizer. `okf_evidence` lists Open Knowledge Format bundle paths the
/// scanner found (markdown with `type:` frontmatter, index.md + log.md).
///
/// `infra` holds compose files, Dockerfiles and proxy configs; `code` maps each source
/// file to its tier so services can be matched to the code they build.
pub fn analyze(
    files: &[ParsedFile],
    okf_evidence: &[String],
    infra: &[InfraFile],
    code: &[CodeFile],
    texts: &[InfraFile],
) -> Option<Insights> {
    let data_structures = dsa::analyze(files);
    let deployment = deployment::analyze(infra, code);
    let ai = ai::analyze_texts(files, okf_evidence, texts);
    let ins = Insights {
        ruleset_version: format!(
            "{RULESET_VERSION} / catalog {} / {}",
            catalog::CATALOG.version,
            deployment::catalog_version()
        ),
        data_structures,
        ai: ai.ai,
        libraries: ai.libraries,
        memory_techniques: ai.memory,
        deployment,
        packages: Vec::new(),
    };
    (!ins.is_empty()).then_some(ins)
}

#[cfg(test)]
mod tests;
