use ark_link::ParsedFile;
use ark_scan::{Language, ScannedFile};
use ark_schema::{AiRole, AiSystemKind, DsaKind};

fn pf(path: &str, lang: Language, src: &str) -> ParsedFile {
    ParsedFile {
        scan: ScannedFile {
            path: path.to_owned(),
            size: src.len() as u64,
            hash: "0".into(),
            language: Some(lang),
            other: None,
        },
        facts: ark_parse::parse_file(lang, path, src),
    }
}

#[test]
fn catalog_loads() {
    let c = &*crate::catalog::CATALOG;
    assert!(c.components.len() > 30);
    assert!(c.libraries.iter().any(|l| l.name.contains("OKF")));
}

#[test]
fn detects_stack_queue_heap_recursion_graph() {
    let src = r#"
import heapq
from collections import deque

def balanced(s):
    stack = []
    for ch in s:
        if ch == "(":
            stack.append(ch)
        else:
            stack.pop()
    return not stack

def bfs(graph, start):
    visited = {start}
    q = deque([start])
    while q:
        u = q.popleft()
        for v in graph[u]:
            if v not in visited:
                visited.add(v)
                q.append(v)

def k_smallest(nums, k):
    h = []
    for n in nums:
        heapq.heappush(h, n)
    return [heapq.heappop(h) for _ in range(k)]

def fib(n):
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)
"#;
    let files = [pf("algo.py", Language::Python, src)];
    let f = crate::dsa::analyze(&files);
    let kinds: Vec<DsaKind> = f.iter().map(|x| x.kind).collect();
    for k in [
        DsaKind::Stack,
        DsaKind::Queue,
        DsaKind::Heap,
        DsaKind::Recursion,
        DsaKind::Graph,
    ] {
        assert!(kinds.contains(&k), "missing {k:?}: {f:#?}");
    }
    let stack = f.iter().find(|x| x.kind == DsaKind::Stack).unwrap();
    assert_eq!(stack.subject.as_deref(), Some("stack"));
    assert_eq!(
        stack.ops.iter().map(|o| o.op.as_str()).collect::<Vec<_>>(),
        ["push", "pop"]
    );
    let heap = f.iter().find(|x| x.kind == DsaKind::Heap).unwrap();
    assert_eq!(heap.subject.as_deref(), Some("h"));
    let graph = f.iter().find(|x| x.kind == DsaKind::Graph).unwrap();
    assert!(
        graph.evidence[0].contains("breadth-first"),
        "{:?}",
        graph.evidence
    );
    // Deterministic.
    assert_eq!(f, crate::dsa::analyze(&files));
}

#[test]
fn detects_class_stack_across_methods_and_tree() {
    let src = r#"
class Stack:
    def __init__(self):
        self.items = []
    def push(self, x):
        self.items.append(x)
    def pop(self):
        return self.items.pop()

def height(node):
    if node is None:
        return 0
    return 1 + max(height(node.left), height(node.right))
"#;
    let f = crate::dsa::analyze(&[pf("s.py", Language::Python, src)]);
    assert!(
        f.iter()
            .any(|x| x.kind == DsaKind::Stack && x.subject.as_deref() == Some("items")),
        "{f:#?}"
    );
    assert!(f.iter().any(|x| x.kind == DsaKind::BinaryTree));
}

#[test]
fn recognizes_hybrid_rag_with_reranker() {
    let src = r#"
from langchain_community.document_loaders import PyPDFLoader
from langchain_text_splitters import RecursiveCharacterTextSplitter
from langchain_huggingface import HuggingFaceEmbeddings
from langchain_chroma import Chroma
from langchain_community.retrievers import BM25Retriever
from langchain.retrievers import EnsembleRetriever
from sentence_transformers import CrossEncoder
from langchain_ollama import ChatOllama

docs = PyPDFLoader("book.pdf").load()
splitter = RecursiveCharacterTextSplitter(chunk_size=800, chunk_overlap=120)
chunks = splitter.split_documents(docs)
emb = HuggingFaceEmbeddings(model_name="BAAI/bge-small-en-v1.5")
vs = Chroma.from_documents(chunks, emb)
dense = vs.as_retriever(search_kwargs={"k": 8})
sparse = BM25Retriever.from_documents(chunks, k=8)
hybrid = EnsembleRetriever(retrievers=[dense, sparse], weights=[0.5, 0.5])
reranker = CrossEncoder("cross-encoder/ms-marco-MiniLM-L-6-v2")
llm = ChatOllama(model="qwen3:4b", temperature=0, num_ctx=8192)
"#;
    let files = [pf("rag.py", Language::Python, src)];
    let out = crate::ai::analyze(&files, &[]);
    let ai = out.ai.expect("ai architecture");
    assert_eq!(ai.kind, AiSystemKind::HybridRag);
    let get = |r: AiRole| {
        ai.stages
            .iter()
            .find(|s| s.role == r)
            .unwrap_or_else(|| panic!("no {r:?}: {:#?}", ai.stages))
    };
    assert_eq!(get(AiRole::Splitter).params["chunk_size"], "800");
    assert_eq!(get(AiRole::Splitter).params["chunk_overlap"], "120");
    assert_eq!(
        get(AiRole::Embedder).params["model_name"],
        "BAAI/bge-small-en-v1.5"
    );
    assert_eq!(get(AiRole::Retriever).params["k"], "8");
    assert_eq!(
        get(AiRole::Reranker).params["model"],
        "cross-encoder/ms-marco-MiniLM-L-6-v2"
    );
    assert_eq!(get(AiRole::Llm).params["num_ctx"], "8192");
    assert!(ai.edges.iter().any(|e| e.label == "keyword hits"));
    assert!(out.libraries.iter().any(|l| l.name == "LangChain"));
    assert!(
        out.memory
            .iter()
            .any(|m| m.name.contains("keep_alive") || m.name.contains("Model residency"))
    );
}

#[test]
fn recognizes_langgraph_agent_loop_and_prompt_cache() {
    let src = r#"
from langgraph.graph import StateGraph, END
from langchain_core.tools import tool
from anthropic import Anthropic

@tool
def search_docs(q: str) -> str:
    """Search the docs."""
    return q

def call_model(state):
    client = Anthropic()
    return client.messages.create(model="claude-x", max_tokens=1024, system=[{"type": "text", "text": "...", "cache_control": {"type": "ephemeral"}}])

g = StateGraph(dict)
g.add_node("agent", call_model)
g.add_node("tools", search_docs)
g.add_conditional_edges("agent", lambda s: "tools")
"#;
    let out = crate::ai::analyze(&[pf("agent.py", Language::Python, src)], &[]);
    let ai = out.ai.unwrap();
    assert_eq!(ai.kind, AiSystemKind::Agent, "{:#?}", ai.stages);
    assert!(ai.edges.iter().any(|e| e.loop_back), "{:#?}", ai.edges);
    let router = ai.stages.iter().find(|s| s.role == AiRole::Router).unwrap();
    assert_eq!(router.params["nodes"], "agent, tools");
    assert!(
        out.memory.iter().any(|m| m.engine == "Anthropic API"),
        "{:#?}",
        out.memory
    );
}

#[test]
fn recognizes_vllm_prefix_caching_and_torch_attention() {
    let src = r#"
import torch
from torch import nn
from vllm import LLM

class CausalSelfAttention(nn.Module):
    def __init__(self, d, h):
        super().__init__()
        self.attn = nn.MultiheadAttention(embed_dim=d, num_heads=h, batch_first=True)

llm = LLM(model="Qwen/Qwen3-4B", enable_prefix_caching=True, gpu_memory_utilization=0.85, kv_cache_dtype="fp8")
"#;
    let out = crate::ai::analyze(&[pf("serve.py", Language::Python, src)], &[]);
    let names: Vec<&str> = out.memory.iter().map(|m| m.name.as_str()).collect();
    assert!(names.contains(&"Automatic prefix caching"), "{names:?}");
    assert!(names.contains(&"PagedAttention KV cache"));
    assert!(names.contains(&"KV cache quantization"));
    let ai = out.ai.unwrap();
    assert!(
        ai.stages
            .iter()
            .any(|s| s.name == "CausalSelfAttention" && s.explain.contains("KV cache"))
    );
    let attn = ai
        .stages
        .iter()
        .find(|s| s.name == "MultiheadAttention")
        .unwrap();
    assert_eq!(attn.params["num_heads"], "h");
}

#[test]
fn js_vector_search_top_k() {
    let src = r#"
import { MemoryVectorStore } from "langchain/vectorstores/memory";
import { OpenAIEmbeddings } from "@langchain/openai";
export async function ask(q) {
  const store = await MemoryVectorStore.fromTexts(texts, meta, new OpenAIEmbeddings());
  const hits = await store.similaritySearch(q, 4, { k: 4 });
  return hits;
}
"#;
    let out = crate::ai::analyze(&[pf("a.ts", Language::TypeScript, src)], &[]);
    let ai = out.ai.unwrap();
    assert_eq!(ai.kind, AiSystemKind::Rag);
    let r = ai
        .stages
        .iter()
        .find(|s| s.role == AiRole::Retriever)
        .unwrap();
    assert_eq!(r.params["k"], "4");
}

mod deployment {
    use crate::deployment::{CodeFile, InfraFile, analyze, parse_bytes};
    use ark_schema::{LinkVia, ServiceKind, TierLabel};

    const COMPOSE: &str = r#"
services:
  web:
    build: ./web
    environment:
      API_URL: http://api:8000
  api:
    build:
      context: ./backend
    command: uvicorn app.main:app --port 8000
    environment:
      - QDRANT_URL=http://vectors:6333
      - REDIS_URL=redis://cache:6379
      - LLM_URL=${LLM_URL:-http://llm:8000/v1}
    deploy:
      resources:
        limits: { memory: 1.5g, cpus: "1" }
  vectors:
    image: qdrant/qdrant:v1.15.0
    mem_limit: 2048m
  llm:
    image: vllm/vllm-openai:latest
    command: --model Qwen/Qwen3-4B --gpu-memory-utilization 0.9
    gpus: all
  cache:
    image: docker.io/library/redis:7
    command: ["redis-server", "--maxmemory", "128mb"]
"#;

    fn run() -> ark_schema::Deployment {
        let infra = [
            InfraFile {
                path: "docker-compose.yml",
                text: COMPOSE,
            },
            InfraFile {
                path: "web/Dockerfile",
                text: "FROM node:22\nCMD [\"npm\",\"start\"]",
            },
            InfraFile {
                path: "backend/Dockerfile",
                text: "FROM python:3.12\nEXPOSE 8000",
            },
        ];
        let code = [
            CodeFile {
                path: "web/src/App.tsx",
                tier: TierLabel::Frontend,
            },
            CodeFile {
                path: "backend/app/main.py",
                tier: TierLabel::Backend,
            },
            CodeFile {
                path: "backend/app/memory.py",
                tier: TierLabel::Backend,
            },
        ];
        analyze(&infra, &code).expect("deployment")
    }

    #[test]
    fn sizes_parse() {
        assert_eq!(parse_bytes("1.5g"), Some(1_610_612_736));
        assert_eq!(parse_bytes("2048m"), Some(2_147_483_648));
        assert_eq!(parse_bytes("512k"), Some(524_288));
        assert_eq!(parse_bytes("lots"), None);
    }

    #[test]
    fn services_kinds_limits_and_entry() {
        let d = run();
        let by = |n: &str| d.services.iter().find(|s| s.name == n).unwrap();
        assert_eq!(by("web").kind, ServiceKind::Frontend);
        assert_eq!(by("web").code_files, 1);
        let api = by("api");
        assert_eq!(api.kind, ServiceKind::Api);
        assert_eq!(api.code_files, 2);
        assert_eq!(api.memory_limit, Some(1_610_612_736));
        assert_eq!(api.entry_file.as_deref(), Some("file:backend/app/main.py"));
        assert_eq!(by("vectors").kind, ServiceKind::VectorStore);
        assert_eq!(by("vectors").memory_limit, Some(2_147_483_648));
        let llm = by("llm");
        assert_eq!(llm.kind, ServiceKind::ModelServer);
        assert!(llm.gpu);
        assert!(llm.explain.contains("0.9 of the GPU"));
        let cache = by("cache");
        assert_eq!(cache.kind, ServiceKind::Cache);
        assert!(cache.explain.contains("128mb"));
    }

    #[test]
    fn links_only_from_proven_hosts() {
        let d = run();
        let has = |a: &str, b: &str| {
            d.links.iter().any(|l| {
                l.from == format!("svc:{a}") && l.to == format!("svc:{b}") && l.via == LinkVia::Env
            })
        };
        assert!(has("web", "api"));
        assert!(has("api", "vectors"));
        assert!(has("api", "cache"));
        assert!(has("api", "llm"), "default inside ${{VAR:-default}} counts");
        assert!(!d.links.iter().any(|l| l.from == "svc:vectors"));
    }

    #[test]
    fn deterministic() {
        assert_eq!(run(), run());
    }
}

#[test]
fn recognizes_qdrant_sparse_dense_hybrid() {
    let src = r#"
from qdrant_client import QdrantClient, models
from fastembed import SparseTextEmbedding, TextEmbedding

client = QdrantClient(url="http://qdrant:6333")
dense = TextEmbedding("BAAI/bge-small-en-v1.5")
sparse = SparseTextEmbedding(model_name="prithivida/Splade_PP_en_v1")

def search(q):
    return client.query_points(
        "docs",
        prefetch=[models.Prefetch(query=dv, using="dense", limit=20), models.Prefetch(query=sv, using="sparse", limit=20)],
        query=models.FusionQuery(fusion=models.Fusion.RRF),
        limit=5,
    )
"#;
    let out = crate::ai::analyze(&[pf("search.py", Language::Python, src)], &[]);
    let ai = out.ai.expect("ai");
    let roles: Vec<AiRole> = ai.stages.iter().map(|s| s.role).collect();
    assert!(roles.contains(&AiRole::KeywordIndex), "{roles:?}");
    assert!(roles.contains(&AiRole::Fusion), "{roles:?}");
    assert!(roles.contains(&AiRole::Embedder), "{roles:?}");
}

#[test]
fn recognizes_custom_bm25_and_bge_m3() {
    let src = r#"
from FlagEmbedding import BGEM3FlagModel

model = BGEM3FlagModel("BAAI/bge-m3", use_fp16=True)

class BM25Index:
    def __init__(self, docs, k1=1.5, b=0.75):
        self.k1 = k1

def build(docs):
    return BM25Index(docs)

def dense_search(q):
    out = model.encode([q], return_dense=True, return_sparse=True)
    return out

def sparse_search(index, q):
    return index.bm25_score(q)
"#;
    let out = crate::ai::analyze(&[pf("search.py", Language::Python, src)], &[]);
    let ai = out.ai.expect("ai");
    let roles: Vec<AiRole> = ai.stages.iter().map(|s| s.role).collect();
    assert!(roles.contains(&AiRole::KeywordIndex), "{roles:?}");
    let emb = ai
        .stages
        .iter()
        .find(|s| s.role == AiRole::Embedder)
        .expect("embedder");
    assert_eq!(emb.params["model"], "BAAI/bge-m3");
}

#[test]
fn re_search_and_game_embed_are_not_ai() {
    let src = r#"
import re
from game import fs

def find(t):
    return re.search(r"a+", t)

def draw():
    fs.embed("shader")
"#;
    let out = crate::ai::analyze(&[pf("a.py", Language::Python, src)], &[]);
    assert!(out.ai.is_none(), "{:#?}", out.ai);
}

#[test]
fn recognizes_whisper_gemini_and_local_http() {
    let src = r#"
import requests
from faster_whisper import WhisperModel
import google.generativeai as genai

asr = WhisperModel("small")
g = genai.GenerativeModel("gemini-2.0-flash")

def chat(p):
    return requests.post("http://localhost:11434/api/chat", json={"model": "qwen3"})
"#;
    let out = crate::ai::analyze(&[pf("a.py", Language::Python, src)], &[]);
    let ai = out.ai.expect("ai");
    let libs: Vec<&str> = ai.stages.iter().map(|s| s.library.as_str()).collect();
    assert!(libs.iter().any(|l| l.contains("Whisper")), "{libs:?}");
    assert!(libs.iter().any(|l| l.contains("Gemini")), "{libs:?}");
    assert!(
        libs.iter().any(|l| l.contains("local model server")),
        "{libs:?}"
    );
}

#[test]
fn model_ids_and_fts_in_config_files() {
    use crate::InfraFile;
    let texts = [
        InfraFile {
            path: "app/config.py",
            text: "embed_model: str = \"BAAI/bge-m3\"\nrerank = 'BAAI/bge-reranker-v2-m3'\n",
        },
        InfraFile {
            path: "app/pg_store.py",
            text: "SELECT ts_rank(tsv, q) FROM chunks, to_tsvector('simple', body) ...",
        },
    ];
    let ai = crate::ai::analyze_texts(&[], &[], &texts).ai.expect("ai");
    let roles: Vec<AiRole> = ai.stages.iter().map(|s| s.role).collect();
    assert!(roles.contains(&AiRole::Embedder), "{roles:?}");
    assert!(roles.contains(&AiRole::Reranker), "{roles:?}");
    assert!(roles.contains(&AiRole::KeywordIndex), "{roles:?}");
    // plain full-text SQL alone is not an AI system
    let only = [InfraFile {
        path: "db/search.sql",
        text: "select to_tsvector('x')",
    }];
    assert!(crate::ai::analyze_texts(&[], &[], &only).ai.is_none());
}

#[test]
fn recognizes_pgvector_sql_rrf_and_fts_with_bm25_comment() {
    use crate::InfraFile;
    let texts = [
        InfraFile {
            path: "corpus.yaml",
            text: "embedding:\n  model: BAAI/bge-m3\n",
        },
        InfraFile {
            path: "pg_store.py",
            text: "# BM25-style idf weight\nq = \"SELECT ts_rank(tsv, plainto_tsquery('simple', %s))\"\nv = \"ORDER BY embedding <=> %s::vector\"\nrrf_k = 60\n",
        },
    ];
    let ai = crate::ai::analyze_texts(&[], &[], &texts).ai.expect("ai");
    let has = |r: ark_schema::AiRole, n: &str| ai.stages.iter().any(|s| s.role == r && s.name == n);
    assert!(has(ark_schema::AiRole::VectorStore, "pgvector"));
    assert!(has(ark_schema::AiRole::Fusion, "RRF"));
    assert!(has(ark_schema::AiRole::KeywordIndex, "Postgres full-text"));
    assert!(has(ark_schema::AiRole::Embedder, "BAAI/bge-m3"));
}
