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
