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
    /// Recognized data structures, AI/ML pipelines, named libraries and memory techniques.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub insights: Option<Insights>,
    /// Example runs: what happens, hop by hop, when a user uses one UI element. Every hop
    /// carries the code or config line that proves it; nothing is guessed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<ExampleRun>>", optional)]
    pub example_runs: Vec<ExampleRun>,
    /// What the project does, split into features (one per route family or screen
    /// group), each with its own files, tables, services and tech stack.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<Feature>>", optional)]
    pub features: Vec<Feature>,
    /// Where each kind of data lives while the system runs: GPU memory, RAM, disk, the
    /// browser or a remote provider. Every entry carries the line that proves it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<MemoryPlacement>>", optional)]
    pub memory_map: Vec<MemoryPlacement>,
    /// Limits and thresholds written as named constants in the code (`MAX_CHARS = 2000`),
    /// each with the line that defines it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<CodeRule>>", optional)]
    pub rules: Vec<CodeRule>,
    /// Problems that stop the code from running as written: syntax errors, imports
    /// that point nowhere, packages missing from the manifest, calls to routes that
    /// do not exist. The rest of the analysis still runs on broken code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub health: Option<CodeHealth>,
}

/// A named number in the code that bounds behaviour: a size, count, limit or timeout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodeRule {
    /// The constant's name as written, e.g. `MAX_SUMMARY_CHARS`.
    pub name: String,
    /// The value as written, e.g. `2000`.
    pub value: String,
    /// The name in words, e.g. `max summary chars`.
    pub meaning: String,
    /// `path:line` of the definition.
    pub proof: String,
}

/// One thing the project does for its users, e.g. "Todos" or "Sign in".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct Feature {
    /// `feature:<slug>`.
    pub id: String,
    pub name: String,
    /// Plain sentence, e.g. "Handles 3 routes under /todos and 2 screens."
    pub explain: String,
    /// How the feature was found: `routes` (a URL family) or `cluster` (a code group).
    pub basis: String,
    pub routes: Vec<String>,
    pub ui: Vec<String>,
    /// Files only this feature uses.
    pub files: Vec<String>,
    /// Files this feature shares with other features.
    pub shared_files: Vec<String>,
    pub tables: Vec<String>,
    pub services: Vec<String>,
    pub ai_stages: Vec<String>,
    /// Packages the feature's files import, most used first.
    pub stack: Vec<String>,
}

/// Where a piece of runtime data is kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct MemoryPlacement {
    pub id: String,
    /// What is stored, e.g. "Chat history (last N turns)".
    pub what: String,
    pub place: MemoryPlace,
    /// Service id (`svc:redis`), tier box id (`tier:backend`) or `browser` / `remote`.
    pub holder: String,
    pub holder_name: String,
    pub lifetime: MemoryLifetime,
    /// Limit or eviction, e.g. `maxmemory 256mb, allkeys-lru` or `ttl`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub bound: Option<String>,
    /// `path:line` (or `path`) for every fact used.
    pub proof: Vec<String>,
    pub explain: String,
    /// Memory technique, AI stage or service id this came from.
    pub source: String,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema, TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum MemoryPlace {
    /// GPU memory.
    Vram,
    Ram,
    Disk,
    Browser,
    /// Kept by a hosted provider (model API, hosted vector DB).
    Remote,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema, TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum MemoryLifetime {
    /// Gone when the request finishes.
    Request,
    /// Kept for one conversation or user session.
    Session,
    /// Kept until the process or container restarts.
    Process,
    /// Survives restarts.
    Persistent,
}

/// What is broken, found without running anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct CodeHealth {
    pub files_checked: u32,
    pub files_with_errors: u32,
    pub issues: Vec<HealthIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct HealthIssue {
    pub id: String,
    pub kind: HealthKind,
    pub severity: Severity,
    pub file_id: String,
    pub line: u32,
    pub message: String,
    /// What to try, in one sentence.
    pub fix: String,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema, TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum HealthKind {
    SyntaxError,
    UnresolvedImport,
    MissingPackage,
    DeadRoute,
    MissingEntry,
    /// GPU code (vLLM, CUDA tensors) inside a container that reserves no GPU.
    NoGpu,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema, TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
}

/// One request followed from a UI element through the services and back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct ExampleRun {
    /// `run:<ui element id>`.
    pub id: String,
    pub title: String,
    /// The UI element the user acts on.
    pub start: String,
    /// Text the user would type, taken from the screen's placeholder or label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub sample_input: Option<String>,
    pub hops: Vec<RunHop>,
}

/// One step of an example run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct RunHop {
    /// Graph id of what runs at this step (ui, fn, route, ai stage or svc id).
    pub node: String,
    pub kind: RunHopKind,
    pub label: String,
    /// The service (container) this step runs in, when the deployment is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub service: Option<String>,
    /// How control got here from the previous hop.
    pub via: RunVia,
    /// `path:line` locations and evidence ids that prove the hop.
    pub proof: Vec<String>,
    /// 1.0 for a resolved call or declared link; lower when matched by import or kind.
    pub confidence: f64,
    /// One sentence for the narrator.
    pub explain: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum RunHopKind {
    Ui,
    Function,
    Route,
    Proxy,
    Stage,
    Data,
    Service,
    Response,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum RunVia {
    Start,
    Handler,
    Call,
    Http,
    Proxy,
    Import,
    Pipeline,
    Sql,
    Network,
    Return,
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
    /// A desktop app: a web UI in a native window (Tauri, Electron) calling native code.
    Desktop,
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

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema, TS,
)]
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
    /// Third-party packages this file imports (standard library left out), sorted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<String>>", optional)]
    pub packages: Vec<String>,
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
    /// Control-flow outline (branches, loops, try, early exits) for the ideal-path view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub flow: Option<Vec<FlowNode>>,
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

// ---------------------------------------------------------------------------
// Control flow (ideal-path view)
// ---------------------------------------------------------------------------

/// One step of a function's control-flow outline. Built from the syntax tree only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct FlowNode {
    pub kind: FlowKind,
    pub line: u32,
    /// Condition, loop header, callee or statement text, clipped.
    pub label: String,
    /// True when this step leaves the function on an error path (raise, throw,
    /// panic, `return Err`, `return nil, err`, `?`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ts(as = "Option<bool>", optional)]
    pub error: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<FlowNode>>", optional)]
    pub children: Vec<FlowNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum FlowKind {
    Branch,
    Else,
    Loop,
    Try,
    Catch,
    Finally,
    Switch,
    Case,
    Call,
    Return,
    Raise,
}

// ---------------------------------------------------------------------------
// Insights: data structures, AI/ML architecture, libraries, memory techniques
// ---------------------------------------------------------------------------

/// Everything the pattern recognizers found. Deterministic: rule ids, sorted output.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct Insights {
    pub ruleset_version: String,
    pub data_structures: Vec<DsaFinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ai: Option<AiArchitecture>,
    pub libraries: Vec<KnownLibrary>,
    pub memory_techniques: Vec<MemoryTechnique>,
    /// Containers and services from docker-compose / Dockerfiles, and how they connect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub deployment: Option<Deployment>,
}

impl Insights {
    pub fn is_empty(&self) -> bool {
        self.data_structures.is_empty()
            && self.deployment.is_none()
            && self.ai.is_none()
            && self.libraries.is_empty()
            && self.memory_techniques.is_empty()
    }
}

/// A data structure or algorithmic pattern used inside one function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct DsaFinding {
    pub id: String,
    pub kind: DsaKind,
    pub file_id: String,
    pub function_id: String,
    pub line: u32,
    /// Variable or type the structure lives in, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub subject: Option<String>,
    pub rule_id: String,
    pub confidence: f64,
    /// Short facts that fired the rule, e.g. `stack.append (line 12)`.
    pub evidence: Vec<String>,
    /// Operations in source order, replayed by the animated view.
    pub ops: Vec<DsaOp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct DsaOp {
    /// Normalized: push, pop, peek, enqueue, dequeue, insert, extract, visit, lookup, recurse, union, find.
    pub op: String,
    pub line: u32,
    /// The call as written, e.g. `heapq.heappush`.
    pub call: String,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema, TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum DsaKind {
    Stack,
    Queue,
    Deque,
    Heap,
    HashMap,
    HashSet,
    LinkedList,
    BinaryTree,
    Trie,
    Graph,
    UnionFind,
    Recursion,
    Memoization,
    BinarySearch,
    Sorting,
}

/// The recognized AI/ML system: its stages in data-flow order and the edges between them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct AiArchitecture {
    pub kind: AiSystemKind,
    pub headline: String,
    pub frameworks: Vec<String>,
    pub stages: Vec<AiStage>,
    pub edges: Vec<AiEdge>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum AiSystemKind {
    /// Retrieve-then-generate over one index.
    Rag,
    /// Dense + keyword (BM25) retrieval fused, usually followed by a reranker.
    HybridRag,
    /// A model that picks tools in a loop.
    Agent,
    /// An agent whose tools include retrieval.
    AgenticRag,
    /// Calls a model, no retrieval or tools.
    Inference,
    /// Defines or trains a neural network.
    ModelTraining,
    /// Serves a model (vLLM, llama.cpp, TGI, Ollama).
    ModelServing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct AiStage {
    pub id: String,
    pub role: AiRole,
    /// The class or function as written, e.g. `RecursiveCharacterTextSplitter`.
    pub name: String,
    pub library: String,
    pub file_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub function_id: Option<String>,
    pub line: u32,
    /// Literal settings found at the call: chunk_size, chunk_overlap, k, top_n, model, temperature ...
    pub params: BTreeMap<String, String>,
    pub rule_id: String,
    /// Student-facing explanation of what this stage does.
    pub explain: String,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema, TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum AiRole {
    Loader,
    Splitter,
    Embedder,
    VectorStore,
    KeywordIndex,
    Retriever,
    Fusion,
    Reranker,
    Prompt,
    Router,
    Llm,
    Tool,
    Agent,
    Memory,
    Cache,
    Guardrail,
    Evaluator,
    ModelServer,
    Tokenizer,
    NeuralLayer,
    Trainer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct AiEdge {
    pub from: String,
    pub to: String,
    pub label: String,
    /// True for the edge that closes an agent / re-query loop.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ts(as = "Option<bool>", optional)]
    pub loop_back: bool,
}

/// A named library, framework or format the project uses, with a plain explanation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct KnownLibrary {
    pub name: String,
    pub category: String,
    pub explain: String,
    /// Import specifiers or files that proved it.
    pub evidence: Vec<String>,
}

/// A memory-management technique configured in code (KV cache, prompt caching, quantization ...).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct MemoryTechnique {
    pub id: String,
    pub name: String,
    pub engine: String,
    pub file_id: String,
    pub line: u32,
    pub setting: String,
    pub explain: String,
}

/// How the project runs: the services declared in compose files and Dockerfiles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct Deployment {
    /// Compose files and Dockerfiles that were read.
    pub sources: Vec<String>,
    pub services: Vec<Service>,
    pub links: Vec<ServiceLink>,
}

/// One container or process in the deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct Service {
    /// `svc:<name>`.
    pub id: String,
    pub name: String,
    pub kind: ServiceKind,
    /// Image as written (`qdrant/qdrant:v1.12.4`), or the Dockerfile base image when built.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub image: Option<String>,
    /// Build context folder, relative to the repository root, when the image is built here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub build_context: Option<String>,
    /// Start command (compose `command` or Dockerfile CMD/ENTRYPOINT).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub command: Option<String>,
    /// File id of the program the command starts (e.g. `uvicorn app.main:app` -> app/main.py).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub entry_file: Option<String>,
    /// Published or exposed ports as written.
    pub ports: Vec<String>,
    /// Memory limit in bytes (`deploy.resources.limits.memory` or `mem_limit`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub memory_limit: Option<u64>,
    /// Memory reservation in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub memory_reservation: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub cpu_limit: Option<String>,
    /// Asks for a GPU (device reservation, `gpus:` or the nvidia runtime).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ts(as = "Option<bool>", optional)]
    pub gpu: bool,
    /// Volume mounts as written (`qdrant_data:/qdrant/storage`).
    pub volumes: Vec<String>,
    /// Environment variable names only; values are never stored.
    pub env_keys: Vec<String>,
    /// Number of source files inside the build context.
    pub code_files: u32,
    /// Where the service is declared, `path:line`.
    pub declared_in: String,
    /// Student-facing sentence: what this service is for.
    pub explain: String,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum ServiceKind {
    Frontend,
    Proxy,
    Api,
    Worker,
    Agent,
    ModelServer,
    VectorStore,
    SearchEngine,
    Database,
    Cache,
    Broker,
    ObjectStore,
    Observability,
    Other,
}

/// A connection between services, with the line that proves it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct ServiceLink {
    pub from: String,
    pub to: String,
    pub via: LinkVia,
    /// e.g. `QDRANT_URL -> qdrant:6333`, or `depends_on`.
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum LinkVia {
    /// An environment variable points at the other service's host name.
    Env,
    /// `depends_on` / `links`: start order, not proof of traffic on its own.
    DependsOn,
    /// A proxy config routes to it.
    Proxy,
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
