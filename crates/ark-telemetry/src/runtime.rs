//! Parsers for local model-server introspection endpoints. All pure functions over
//! response bodies, so they are testable without a server.
//!
//! | Server | Endpoint | What we read |
//! |---|---|---|
//! | Ollama (11434) | `GET /api/ps` | loaded models, total size, size in VRAM, context length, expiry |
//! | llama-server (8080) | `GET /metrics` (needs `--metrics`), `GET /slots` | tokens/s, busy slots, per-slot context size |
//! | vLLM (8000) | `GET /metrics` | `vllm:kv_cache_usage_perc`, prefix-cache hits/queries, running/waiting requests |

use std::collections::BTreeMap;

use serde::Serialize;

/// Prometheus text exposition format → `name{labels}` → value. Comments skipped.
pub fn parse_prometheus(text: &str) -> BTreeMap<String, f64> {
    let mut m = BTreeMap::new();
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        // Value is the first token after the series (labels may contain spaces).
        let split_at = match l.find('}') {
            Some(i) => i + 1,
            None => l.find(char::is_whitespace).unwrap_or(l.len()),
        };
        let (series, rest) = l.split_at(split_at);
        if let Some(v) = rest
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<f64>().ok())
        {
            m.insert(series.trim().to_owned(), v);
        }
    }
    m
}

/// Sum every series of a metric family (ignoring labels).
pub fn family(m: &BTreeMap<String, f64>, name: &str) -> Option<f64> {
    let mut found = false;
    let mut sum = 0.0;
    for (k, v) in m {
        let base = k.split('{').next().unwrap_or(k);
        if base == name {
            found = true;
            sum += v;
        }
    }
    found.then_some(sum)
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LoadedModel {
    pub name: String,
    pub size_bytes: u64,
    pub vram_bytes: u64,
    pub context_length: Option<u64>,
    pub quantization: Option<String>,
    pub parameter_size: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    /// `ollama`, `llama.cpp`, `vllm`.
    pub server: String,
    pub endpoint: String,
    pub models: Vec<LoadedModel>,
    /// 0-1 fraction of the KV cache in use, when the server reports it.
    pub kv_cache_usage: Option<f64>,
    /// Prefix-cache hit rate 0-1 (vLLM).
    pub prefix_cache_hit_rate: Option<f64>,
    pub requests_running: Option<f64>,
    pub requests_waiting: Option<f64>,
    pub prompt_tokens_total: Option<f64>,
    pub generated_tokens_total: Option<f64>,
    /// Per-slot context sizes (llama-server).
    pub slot_contexts: Vec<u64>,
    pub busy_slots: Option<u64>,
    /// Plain-language notes, e.g. "start llama-server with --metrics".
    pub notes: Vec<String>,
}

/// Ollama `GET /api/ps`.
pub fn parse_ollama_ps(json: &str) -> Option<RuntimeStatus> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let models = v.get("models")?.as_array()?;
    let models = models
        .iter()
        .map(|m| LoadedModel {
            name: m
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_owned(),
            size_bytes: m.get("size").and_then(|x| x.as_u64()).unwrap_or(0),
            vram_bytes: m.get("size_vram").and_then(|x| x.as_u64()).unwrap_or(0),
            context_length: m.get("context_length").and_then(|x| x.as_u64()),
            quantization: m
                .pointer("/details/quantization_level")
                .and_then(|x| x.as_str())
                .map(str::to_owned),
            parameter_size: m
                .pointer("/details/parameter_size")
                .and_then(|x| x.as_str())
                .map(str::to_owned),
            expires_at: m
                .get("expires_at")
                .and_then(|x| x.as_str())
                .map(str::to_owned),
        })
        .collect();
    Some(RuntimeStatus {
        server: "ollama".into(),
        models,
        ..Default::default()
    })
}

/// vLLM `GET /metrics`.
pub fn parse_vllm_metrics(text: &str) -> RuntimeStatus {
    let m = parse_prometheus(text);
    let hits =
        family(&m, "vllm:prefix_cache_hits").or_else(|| family(&m, "vllm:prefix_cache_hits_total"));
    let queries = family(&m, "vllm:prefix_cache_queries")
        .or_else(|| family(&m, "vllm:prefix_cache_queries_total"));
    RuntimeStatus {
        server: "vllm".into(),
        kv_cache_usage: family(&m, "vllm:kv_cache_usage_perc")
            .or_else(|| family(&m, "vllm:gpu_cache_usage_perc")),
        prefix_cache_hit_rate: match (hits, queries) {
            (Some(h), Some(q)) if q > 0.0 => Some(h / q),
            _ => None,
        },
        requests_running: family(&m, "vllm:num_requests_running"),
        requests_waiting: family(&m, "vllm:num_requests_waiting"),
        prompt_tokens_total: family(&m, "vllm:prompt_tokens_total")
            .or_else(|| family(&m, "vllm:prompt_tokens")),
        generated_tokens_total: family(&m, "vllm:generation_tokens_total")
            .or_else(|| family(&m, "vllm:generation_tokens")),
        ..Default::default()
    }
}

/// llama-server `GET /metrics` (optional) and `GET /slots`.
pub fn parse_llama_server(metrics: Option<&str>, slots: Option<&str>) -> RuntimeStatus {
    let mut st = RuntimeStatus {
        server: "llama.cpp".into(),
        ..Default::default()
    };
    match metrics {
        Some(t) => {
            let m = parse_prometheus(t);
            st.prompt_tokens_total = family(&m, "llamacpp:prompt_tokens_total");
            st.generated_tokens_total = family(&m, "llamacpp:tokens_predicted_total");
            st.requests_running = family(&m, "llamacpp:requests_processing");
            st.requests_waiting = family(&m, "llamacpp:requests_deferred");
            st.kv_cache_usage = family(&m, "llamacpp:kv_cache_usage_ratio");
        }
        None => st
            .notes
            .push("Start llama-server with --metrics to see token counters.".into()),
    }
    if let Some(s) = slots
        && let Ok(serde_json::Value::Array(a)) = serde_json::from_str::<serde_json::Value>(s)
    {
        st.slot_contexts = a
            .iter()
            .filter_map(|x| x.get("n_ctx").and_then(|v| v.as_u64()))
            .collect();
        st.busy_slots = Some(
            a.iter()
                .filter(|x| {
                    x.get("is_processing")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                })
                .count() as u64,
        );
    }
    st
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prometheus_and_vllm() {
        let t = "# HELP x\nvllm:kv_cache_usage_perc{model_name=\"q\"} 0.42\nvllm:prefix_cache_hits_total{model_name=\"q\"} 30\nvllm:prefix_cache_queries_total{model_name=\"q\"} 120\nvllm:num_requests_running{model_name=\"q\"} 3\n";
        let s = parse_vllm_metrics(t);
        assert_eq!(s.kv_cache_usage, Some(0.42));
        assert_eq!(s.prefix_cache_hit_rate, Some(0.25));
        assert_eq!(s.requests_running, Some(3.0));
    }

    #[test]
    fn ollama_ps() {
        let j = r#"{"models":[{"name":"qwen3:4b","size":5137025024,"size_vram":5137025024,"context_length":8192,"expires_at":"2026-11-25T18:00:00Z","details":{"parameter_size":"4.0B","quantization_level":"Q4_K_M"}}]}"#;
        let s = parse_ollama_ps(j).unwrap();
        assert_eq!(s.models[0].vram_bytes, 5137025024);
        assert_eq!(s.models[0].context_length, Some(8192));
        assert_eq!(s.models[0].quantization.as_deref(), Some("Q4_K_M"));
    }

    #[test]
    fn llama_slots() {
        let s = parse_llama_server(
            None,
            Some(
                r#"[{"id":0,"n_ctx":4096,"is_processing":true},{"id":1,"n_ctx":4096,"is_processing":false}]"#,
            ),
        );
        assert_eq!(s.slot_contexts, [4096, 4096]);
        assert_eq!(s.busy_slots, Some(1));
        assert_eq!(s.notes.len(), 1);
    }
}
