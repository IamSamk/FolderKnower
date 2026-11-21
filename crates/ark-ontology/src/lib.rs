//! `ark-ontology`: the Systems Ontology. YAML rules tag functions with OS, network,
//! cloud and data concepts based on the calls they make. Rules are embedded at
//! build time, so tagging is deterministic and offline.

use ark_schema::{OntologyLayer, OntologyTag};
use regex::Regex;
use serde::{Deserialize, Serialize};

pub const RULES_YAML: &str = include_str!("../../../ontology/rules.yaml");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub layer: OntologyLayer,
    pub concept: String,
    pub explain: String,
    pub calls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleFile {
    pub version: String,
    pub rules: Vec<Rule>,
}

pub struct Ontology {
    pub version: String,
    pub rules: Vec<Rule>,
    compiled: Vec<Vec<Regex>>,
}

impl Ontology {
    pub fn load() -> Self {
        Self::from_yaml(RULES_YAML).expect("embedded ontology is valid")
    }

    pub fn from_yaml(yaml: &str) -> Result<Self, String> {
        let file: RuleFile = serde_yaml_ng::from_str(yaml).map_err(|e| e.to_string())?;
        let mut compiled = Vec::with_capacity(file.rules.len());
        for r in &file.rules {
            let mut v = Vec::new();
            for p in &r.calls {
                // `pattern:hint` suffixes are documentation only.
                let pat = p
                    .rsplit_once("$:")
                    .map(|(a, _)| format!("{a}$"))
                    .unwrap_or(p.clone());
                v.push(Regex::new(&pat).map_err(|e| format!("{}: {e}", r.id))?);
            }
            compiled.push(v);
        }
        Ok(Self {
            version: file.version,
            rules: file.rules,
            compiled,
        })
    }

    /// Tags for a function given its calls as `(receiver, callee)` pairs. Sorted, unique.
    pub fn tag<'a>(
        &self,
        calls: impl IntoIterator<Item = (Option<&'a str>, &'a str)>,
    ) -> Vec<OntologyTag> {
        let keys: Vec<String> = calls
            .into_iter()
            .map(|(r, c)| match r {
                Some(r) => format!("{r}.{c}"),
                None => c.to_owned(),
            })
            .collect();
        let mut out = Vec::new();
        for (rule, pats) in self.rules.iter().zip(&self.compiled) {
            if keys.iter().any(|k| pats.iter().any(|p| p.is_match(k))) {
                out.push(OntologyTag {
                    concept: rule.concept.clone(),
                    layer: rule.layer,
                    rule_id: rule.id.clone(),
                });
            }
        }
        out.sort_by(|a, b| a.rule_id.cmp(&b.rule_id));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_load_and_cover_all_layers() {
        let o = Ontology::load();
        assert!(o.rules.len() >= 60, "only {} rules", o.rules.len());
        for layer in [
            OntologyLayer::Os,
            OntologyLayer::Network,
            OntologyLayer::Cloud,
            OntologyLayer::Data,
        ] {
            assert!(o.rules.iter().any(|r| r.layer == layer));
        }
        let mut ids: Vec<_> = o.rules.iter().map(|r| &r.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), o.rules.len(), "duplicate rule ids");
    }

    /// One positive example per rule, so every rule is exercised.
    #[test]
    fn every_rule_has_a_matching_example() {
        let examples: &[(&str, Option<&str>, &str)] = &[
            ("os.fs.read", Some("fs"), "readFile"),
            ("os.fs.write", Some("fs"), "writeFileSync"),
            ("os.fs.watch", Some("chokidar"), "watch"),
            ("os.process.spawn", Some("subprocess"), "run"),
            ("os.thread.spawn", Some("threading"), "Thread"),
            ("os.async.task", Some("asyncio"), "gather"),
            ("os.timer", None, "setTimeout"),
            ("os.env.read", Some("os"), "getenv"),
            ("os.signal", Some("signal"), "signal"),
            ("os.memory.buffer", Some("Buffer"), "alloc"),
            ("os.lock", Some("threading"), "Lock"),
            ("os.channel", Some("mpsc::"), "channel"),
            ("os.crypto.random", Some("crypto"), "randomUUID"),
            ("os.crypto.hash", Some("hashlib"), "sha256"),
            ("os.compress", Some("zlib"), "gzip"),
            ("os.log", Some("console"), "log"),
            ("os.stdio", None, "print"),
            ("os.gpu", Some("torch"), "tensor"),
            ("net.http.client", None, "fetch"),
            ("net.http.server", Some("uvicorn"), "run"),
            ("net.http.route", Some("router"), "get"),
            ("net.middleware", Some("app"), "use"),
            ("net.websocket", Some("io"), "emit"),
            ("net.socket.tcp", Some("socket"), "socket"),
            ("net.dns", Some("dns"), "lookup"),
            ("net.grpc", Some("grpc"), "insecure_channel"),
            ("net.auth.jwt", Some("jwt"), "verify"),
            ("net.auth.password", Some("bcrypt"), "hashpw"),
            ("net.cors", None, "cors"),
            ("net.rate-limit", None, "rateLimit"),
            ("net.sse", None, "StreamingResponse"),
            ("net.url.parse", None, "URL"),
            ("net.http.proxy", None, "createProxyMiddleware"),
            ("data.sql.query", Some("cursor"), "execute"),
            ("data.orm.read", Some("prisma.todo"), "findMany"),
            ("data.orm.write", Some("db.session"), "commit"),
            ("data.transaction", Some("prisma"), "$transaction"),
            ("data.migration", Some("op"), "create_table"),
            ("data.cache.redis", Some("redis"), "get"),
            ("data.cache.memo", None, "useMemo"),
            ("data.document", Some("collection"), "insertOne"),
            ("data.search", Some("meilisearch"), "search"),
            ("data.serialize.json", Some("JSON"), "stringify"),
            ("data.validation", Some("schema"), "safeParse"),
            ("data.state.client", None, "useState"),
            ("data.state.server-cache", None, "useQuery"),
            ("data.storage.browser", Some("localStorage"), "setItem"),
            ("data.file.upload", None, "multer"),
            ("data.pagination", Some("query"), "offset"),
            ("cloud.aws.s3", Some("s3"), "put_object"),
            ("cloud.queue", Some("sqs"), "send_message"),
            ("cloud.worker.task", Some("task"), "delay"),
            ("cloud.pubsub", Some("client"), "publish"),
            ("cloud.secrets", Some("vault"), "read"),
            ("cloud.email", Some("smtplib"), "SMTP"),
            ("cloud.payments", Some("stripe.checkout.sessions"), "create"),
            ("cloud.llm", Some("openai.chat.completions"), "create"),
            ("cloud.observability", Some("sentry"), "init"),
            ("cloud.container", Some("docker"), "from_env"),
            ("cloud.serverless", None, "lambda_handler"),
        ];
        let o = Ontology::load();
        for (id, recv, callee) in examples {
            let tags = o.tag([(*recv, *callee)]);
            assert!(
                tags.iter().any(|t| t.rule_id == *id),
                "{id} did not match {recv:?}.{callee}; got {:?}",
                tags.iter().map(|t| &t.rule_id).collect::<Vec<_>>()
            );
        }
        let covered: std::collections::BTreeSet<_> = examples.iter().map(|e| e.0).collect();
        for r in &o.rules {
            assert!(covered.contains(r.id.as_str()), "no example for {}", r.id);
        }
    }
}
