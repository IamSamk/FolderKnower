//! `ark-schema`: the v2 architecture graph. These Rust types are the source of truth.
//!
//! They are exported to `schema/architecture.schema.json` (schemars) and
//! `apps/desktop/src/gen/*.ts` (ts-rs) by the `gen-schema` binary. Every collection
//! is a `BTreeMap` or a `Vec` that producers keep sorted, so serialized output is
//! byte-stable across runs.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Current schema version string, written into every graph.
pub const SCHEMA_VERSION: &str = "2.0";

/// Schema version tag. Only `"2.0"` is valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema, TS)]
pub enum SchemaVersion {
    #[default]
    #[serde(rename = "2.0")]
    V2,
}

/// Root document: the whole repository as a tiered graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct SystemArchitectureGraph {
    pub schema_version: SchemaVersion,
    pub repo_name: String,
    /// Stored relative-safe; never leaves the machine.
    pub root_path: String,
    pub generator: GeneratorInfo,
    /// Merkle root (blake3) over sorted file hashes: the determinism key.
    pub content_root: String,
    pub tiers: Vec<TierNode>,
    pub cross_tier_edges: Vec<CrossTierEdge>,
    /// Config, docs, tests, migrations, infra and manifests.
    pub non_code_nodes: Vec<NonCodeNode>,
    /// What kind of project this looks like, and why.
    pub summary: ProjectSummary,
    /// ORM tables, DTO/schema classes and SQL/Prisma tables (the ER view).
    pub data_models: Vec<DataModel>,
    /// HTTP routes declared by backend code.
    pub routes: Vec<RouteNode>,
    /// Interactive UI elements (buttons, forms, inputs) found in components.
    pub ui_elements: Vec<UiElement>,
    /// Ambiguities the classifier wants a human (or the local model) to settle.
    pub questions: Vec<Question>,
}

/// Project-level classification with its evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub kind: ProjectKind,
    pub headline: String,
    pub findings: Vec<Finding>,
    pub stats: Stats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum ProjectKind {
    FullStack,
    Frontend,
    Backend,
    Library,
    Cli,
    Worker,
    #[default]
    Unknown,
}

/// One sentence the app can say about the repo, with the node/rule ids behind it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct Finding {
    pub text: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct Stats {
    pub files: u32,
    pub functions: u32,
    pub lines: u32,
    /// Language to file count.
    pub languages: BTreeMap<String, u32>,
}

/// An entity in the ER view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct DataModel {
    /// `model:<path>#<Name>`.
    pub id: String,
    pub name: String,
    pub kind: DataModelKind,
    /// Framework that declared it, e.g. `sqlalchemy`, `pydantic`, `prisma`.
    pub framework: String,
    pub file_id: String,
    pub line: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub table: Option<String>,
    pub fields: Vec<DataField>,
    pub relations: Vec<Relation>,
    /// Other models with the same shape: a DTO mirroring a table, or vice versa.
    pub mirrors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum DataModelKind {
    /// Persisted table (ORM class, Prisma model, SQL DDL).
    Table,
    /// Validation/transfer shape (Pydantic, zod, DTO).
    Schema,
    /// Document store collection (Mongoose).
    Document,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct DataField {
    pub name: String,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    #[ts(optional, rename = "type")]
    pub ty: Option<String>,
    pub primary_key: bool,
    pub nullable: bool,
    /// `table.column` or model name this field points at.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub references: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct Relation {
    pub target: String,
    pub kind: RelationKind,
    pub via: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum RelationKind {
    ManyToOne,
    OneToMany,
    OneToOne,
    ManyToMany,
}

/// An HTTP route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct RouteNode {
    pub id: String,
    pub method: String,
    pub path: String,
    pub framework: String,
    pub file_id: String,
    pub line: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub handler: Option<String>,
}

/// A clickable/typeable element in a UI component: the "inspect element" entry point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct UiElement {
    pub id: String,
    pub tag: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub label: Option<String>,
    pub file_id: String,
    pub line: u32,
    /// Enclosing component function id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub component: Option<String>,
    /// e.g. `onClick`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub event: Option<String>,
    /// Function id the event handler resolves to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub handler: Option<String>,
}

/// A classification the engine is unsure about.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct Question {
    pub id: String,
    pub text: String,
    /// File id or directory prefix the answer applies to.
    pub subject: String,
    pub options: Vec<TierLabel>,
    pub suggested: TierLabel,
    pub confidence: f64,
}

/// Which tool versions produced the graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct GeneratorInfo {
    pub version: String,
    pub ruleset_version: String,
    /// Grammar name to version, e.g. `"tree-sitter-rust" -> "0.24.2"`.
    pub grammars: BTreeMap<String, String>,
}

/// A macro tier such as `tier:backend`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct TierNode {
    pub id: String,
    pub label: TierLabel,
    /// Manifest/dir rule ids that fired.
    pub evidence: Vec<String>,
    pub clusters: Vec<SubsystemCluster>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema, TS,
)]
pub enum TierLabel {
    Frontend,
    Gateway,
    Backend,
    Storage,
    Workers,
    Shared,
}

/// A semantic subsystem inside a tier, e.g. `cluster:auth`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct SubsystemCluster {
    pub id: String,
    pub name: String,
    pub method: ClusterMethod,
    pub files: Vec<SourceFileNode>,
}

/// How a cluster was formed. Leiden runs with a fixed seed so it is reproducible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct ClusterMethod {
    pub algo: ClusterAlgo,
    pub seed: u32,
    pub resolution: f64,
    /// Anchor node ids (routers, data stores, tests) that guided the partition.
    pub anchors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum ClusterAlgo {
    Leiden,
    /// Deterministic Louvain modularity optimisation over the import/call graph.
    Louvain,
    /// Placeholder grouping by directory, used until Leiden lands (M1.8).
    Directory,
}

/// One source file, e.g. `file:src/auth/service.ts`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct SourceFileNode {
    pub id: String,
    pub path: String,
    pub language: String,
    /// blake3 of the file bytes.
    pub content_hash: String,
    pub lines: u32,
    /// 0..=1: how sure the tier assignment is.
    pub tier_confidence: f64,
    /// Short role guess, e.g. `component`, `route-handler`, `orm-models`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub role: Option<String>,
    pub imports: Vec<Edge>,
    pub functions: Vec<FunctionNode>,
}

/// A file that is not application code but relates to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct NonCodeNode {
    pub id: String,
    pub path: String,
    pub kind: NonCodeKind,
    /// File/function ids this relates to (test -> source, doc -> source).
    pub relates_to: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum NonCodeKind {
    Config,
    /// Prisma/SQL/GraphQL schema files.
    Schema,
    Doc,
    Test,
    Migration,
    Infra,
    Manifest,
}

/// A directed edge, shared by imports and calls. Every edge carries provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct Edge {
    pub target: String,
    pub provenance: Provenance,
    /// 0..=1, derived deterministically from provenance plus resolution rule.
    #[schemars(range(min = 0.0, max = 1.0))]
    pub confidence: f64,
    /// Unresolved targets ranked by a deterministic rule (never a model).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub candidates: Option<Vec<String>>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum Provenance {
    ScipExact,
    TsResolved,
    TsNameMatch,
    RuntimeObserved,
}

/// A function or method.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct FunctionNode {
    /// SCIP symbol if available, else `fn:<path>#<qualname>`.
    pub id: String,
    pub name: String,
    pub visibility: Visibility,
    /// 1-based inclusive line range.
    pub line_range: (u32, u32),
    /// Byte range, for the function locator.
    pub byte_range: (u32, u32),
    pub signature: Signature,
    pub complexity: Complexity,
    pub ontology_tags: Vec<OntologyTag>,
    /// Phase 5 model output; kept apart from static facts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub inferred_role: Option<InferredRole>,
    /// Phase 4 runtime sample.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub live_telemetry: Option<LiveTelemetry>,
    pub calls: Vec<Edge>,
    pub called_by: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct Signature {
    pub parameters: Vec<Parameter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub return_type: Option<String>,
}

/// Types are optional: the UI never shows a type the source did not state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct Parameter {
    pub name: String,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    #[ts(optional, rename = "type")]
    pub ty: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct Complexity {
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub max_nesting: u32,
    #[serde(rename = "timeBigO")]
    #[ts(rename = "timeBigO")]
    pub time_big_o: BigO,
    #[serde(rename = "spaceBigO")]
    #[ts(rename = "spaceBigO")]
    pub space_big_o: BigO,
    /// Rule ids that produced the Big-O labels.
    pub basis: Vec<String>,
    pub memory_flags: Vec<String>,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
    TS,
)]
pub enum BigO {
    #[serde(rename = "O(1)")]
    #[ts(rename = "O(1)")]
    Constant,
    #[serde(rename = "O(log N)")]
    #[ts(rename = "O(log N)")]
    Logarithmic,
    #[serde(rename = "O(N)")]
    #[ts(rename = "O(N)")]
    Linear,
    #[serde(rename = "O(N log N)")]
    #[ts(rename = "O(N log N)")]
    Linearithmic,
    #[serde(rename = "O(N^2)")]
    #[ts(rename = "O(N^2)")]
    Quadratic,
    #[serde(rename = "O(N^k)")]
    #[ts(rename = "O(N^k)")]
    Polynomial,
    #[serde(rename = "O(2^N)")]
    #[ts(rename = "O(2^N)")]
    Exponential,
    #[default]
    #[serde(rename = "unknown")]
    #[ts(rename = "unknown")]
    Unknown,
}

/// A systems-ontology concept matched at this function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct OntologyTag {
    pub concept: String,
    pub layer: OntologyLayer,
    pub rule_id: String,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema, TS,
)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum OntologyLayer {
    Os,
    Network,
    Cloud,
    Data,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct InferredRole {
    pub role: String,
    pub confidence: f64,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct LiveTelemetry {
    pub cpu_percent: f64,
    /// f64 so TS sees `number`; exact to 2^53 bytes.
    pub rss_bytes: f64,
    pub active_threads: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub syscalls_per_sec: Option<f64>,
}

/// An edge between tiers, e.g. frontend fetch -> backend route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct CrossTierEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub protocol: Protocol,
    /// e.g. ontology rule ids at the call sites.
    pub evidence: Vec<String>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema, TS,
)]
pub enum Protocol {
    #[serde(rename = "HTTP/REST")]
    #[ts(rename = "HTTP/REST")]
    HttpRest,
    WebSocket,
    #[serde(rename = "gRPC")]
    #[ts(rename = "gRPC")]
    Grpc,
    #[serde(rename = "SQL")]
    #[ts(rename = "SQL")]
    Sql,
    #[serde(rename = "IPC")]
    #[ts(rename = "IPC")]
    Ipc,
    Queue,
}

/// JSON Schema for [`SystemArchitectureGraph`], pretty-printed with a trailing newline.
pub fn json_schema_string() -> String {
    let schema = schemars::schema_for!(SystemArchitectureGraph);
    let mut s = serde_json::to_string_pretty(&schema).expect("schema serializes");
    s.push('\n');
    s
}

/// Serialize a graph the canonical way: pretty JSON with a trailing newline.
pub fn to_canonical_json(graph: &SystemArchitectureGraph) -> serde_json::Result<String> {
    let mut s = serde_json::to_string_pretty(graph)?;
    s.push('\n');
    Ok(s)
}
