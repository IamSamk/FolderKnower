//! Optional local small-language-model layer.
//!
//! Architekton never needs a model: static analysis owns every edge. When a local
//! OpenAI-compatible server is running (Ollama on :11434 or llama.cpp `llama-server`
//! on :8080), the app can use it to
//!   1. narrate a selected subgraph in plain language, and
//!   2. suggest a tier for files the rules were unsure about.
//!
//! Both calls are grounded: the model only sees serialized facts with stable ids,
//! output is constrained by a JSON schema whose citation field is an enum of those
//! ids, and anything citing an unknown id is dropped before it reaches the UI.
//! Nothing leaves the machine: only loopback addresses are contacted.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const CANDIDATES: &[(&str, &str)] = &[
    ("ollama", "http://127.0.0.1:11434"),
    ("llama.cpp", "http://127.0.0.1:8080"),
    ("lm-studio", "http://127.0.0.1:1234"),
];

fn agent(timeout_s: u64) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(timeout_s)))
        .http_status_as_error(true)
        .build()
        .into()
}

fn is_loopback(url: &str) -> bool {
    let rest = url.strip_prefix("http://").unwrap_or_default();
    rest.starts_with("127.0.0.1") || rest.starts_with("localhost") || rest.starts_with("[::1]")
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SlmStatus {
    available: bool,
    server: Option<String>,
    base_url: Option<String>,
    models: Vec<String>,
    hint: String,
}

fn list_models(base: &str) -> Option<Vec<String>> {
    let mut resp = agent(2).get(format!("{base}/v1/models")).call().ok()?;
    let v: Value = resp.body_mut().read_json().ok()?;
    let mut models: Vec<String> = v["data"]
        .as_array()?
        .iter()
        .filter_map(|m| m["id"].as_str().map(str::to_owned))
        .collect();
    models.sort();
    Some(models)
}

#[tauri::command]
pub async fn slm_status(base_url: Option<String>) -> SlmStatus {
    tauri::async_runtime::spawn_blocking(move || {
        let tries: Vec<(String, String)> = match base_url {
            Some(u) if is_loopback(&u) => vec![("custom".to_owned(), u.trim_end_matches('/').to_owned())],
            _ => CANDIDATES.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
        };
        for (name, base) in tries {
            if let Some(models) = list_models(&base) {
                return SlmStatus {
                    available: !models.is_empty(),
                    server: Some(name),
                    base_url: Some(base),
                    hint: if models.is_empty() {
                        "Server is running but has no models. Pull one, e.g. `ollama pull qwen2.5-coder:3b`.".to_owned()
                    } else {
                        "Local model connected. It only narrates; edges still come from static analysis.".to_owned()
                    },
                    models,
                };
            }
        }
        SlmStatus {
            available: false,
            server: None,
            base_url: None,
            models: Vec::new(),
            hint: "No local model found. Optional: install Ollama and run `ollama pull qwen2.5-coder:3b`, or start llama.cpp's `llama-server`.".to_owned(),
        }
    })
    .await
    .unwrap_or(SlmStatus {
        available: false,
        server: None,
        base_url: None,
        models: Vec::new(),
        hint: "status check failed".to_owned(),
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroundedNode {
    id: String,
    kind: String,
    label: String,
    facts: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplainRequest {
    base_url: String,
    model: String,
    question: String,
    nodes: Vec<GroundedNode>,
    edges: Vec<Value>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Sentence {
    text: String,
    cites: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplainResponse {
    sentences: Vec<Sentence>,
    dropped: usize,
    model: String,
}

fn chat(base: &str, model: &str, system: &str, user: &str, schema: Value) -> Result<Value, String> {
    if !is_loopback(base) {
        return Err("only local (loopback) model servers are allowed".to_owned());
    }
    let body = json!({
        "model": model,
        "temperature": 0,
        "seed": 7,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user}
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {"name": "answer", "strict": true, "schema": schema}
        }
    });
    let mut resp = agent(120)
        .post(format!("{base}/v1/chat/completions"))
        .send_json(&body)
        .map_err(|e| format!("model server: {e}"))?;
    let v: Value = resp.body_mut().read_json().map_err(|e| e.to_string())?;
    let content = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("model returned no content")?;
    // Some servers wrap JSON in prose or fences; take the outermost object.
    let start = content.find('{').ok_or("model returned no JSON")?;
    let end = content.rfind('}').ok_or("model returned no JSON")?;
    serde_json::from_str(&content[start..=end]).map_err(|e| format!("bad JSON from model: {e}"))
}

#[tauri::command]
pub async fn slm_explain(req: ExplainRequest) -> Result<ExplainResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let ids: Vec<String> = req.nodes.iter().map(|n| n.id.clone()).collect();
        if ids.is_empty() {
            return Err("nothing selected".to_owned());
        }
        let facts = json!({
            "nodes": req.nodes.iter().map(|n| json!({"id": n.id, "kind": n.kind, "label": n.label, "facts": n.facts})).collect::<Vec<_>>(),
            "edges": req.edges,
        });
        let schema = json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["sentences"],
            "properties": {
                "sentences": {
                    "type": "array",
                    "maxItems": 8,
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["text", "cites"],
                        "properties": {
                            "text": {"type": "string"},
                            "cites": {"type": "array", "minItems": 1, "items": {"type": "string", "enum": ids}}
                        }
                    }
                }
            }
        });
        let system = "You are a systems teacher explaining a codebase to a developer. \
            Use ONLY the facts provided. Every sentence must cite the ids of the nodes it talks about. \
            Explain what the code does and how OS, network, database and cloud concepts are involved. \
            Never invent functions, files, tables or calls that are not in the facts. Be concise.";
        let user = format!("Question: {}\n\nFacts (JSON):\n{}", req.question, facts);
        let v = chat(&req.base_url, &req.model, system, &user, schema)?;
        let raw: Vec<Sentence> = serde_json::from_value(v["sentences"].clone()).unwrap_or_default();
        let total = raw.len();
        let kept: Vec<Sentence> = raw
            .into_iter()
            .filter(|s| !s.cites.is_empty() && s.cites.iter().all(|c| ids.contains(c)) && !s.text.trim().is_empty())
            .collect();
        Ok(ExplainResponse {
            dropped: total - kept.len(),
            sentences: kept,
            model: req.model,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifyRequest {
    base_url: String,
    model: String,
    question_id: String,
    subject: String,
    options: Vec<String>,
    files: Vec<Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifyResponse {
    question_id: String,
    tier: String,
    reason: String,
    model: String,
}

#[tauri::command]
pub async fn slm_classify(req: ClassifyRequest) -> Result<ClassifyResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let schema = json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["tier", "reason"],
            "properties": {
                "tier": {"type": "string", "enum": req.options},
                "reason": {"type": "string", "maxLength": 240}
            }
        });
        let system = "You classify parts of a software repository into architecture tiers. \
            Frontend = UI code, Gateway = middleware/auth/proxy, Backend = request handlers and business logic, \
            Storage = database models and data access, Workers = background jobs and schedulers, \
            Shared = utilities and types used by several tiers. Answer from the given facts only.";
        let user = format!(
            "Which tier is `{}`?\nFiles (imports, functions, routes):\n{}",
            req.subject,
            serde_json::to_string(&req.files).unwrap_or_default()
        );
        let v = chat(&req.base_url, &req.model, system, &user, schema)?;
        let tier = v["tier"].as_str().unwrap_or_default().to_owned();
        if !req.options.contains(&tier) {
            return Err("model answered outside the allowed options".to_owned());
        }
        Ok(ClassifyResponse {
            question_id: req.question_id,
            tier,
            reason: v["reason"].as_str().unwrap_or_default().to_owned(),
            model: req.model,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
