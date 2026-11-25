"""Ingest a book: load -> chunk -> embed -> store (dense + keyword)."""
from langchain_community.document_loaders import PyPDFLoader
from langchain_text_splitters import RecursiveCharacterTextSplitter
from langchain_huggingface import HuggingFaceEmbeddings
from langchain_chroma import Chroma
from langchain_community.retrievers import BM25Retriever

CHUNK_SIZE = 800
CHUNK_OVERLAP = 120


def build_index(pdf_path: str, persist_dir: str = "./index"):
    docs = PyPDFLoader(pdf_path).load()
    if not docs:
        raise ValueError(f"{pdf_path} has no pages")
    splitter = RecursiveCharacterTextSplitter(chunk_size=800, chunk_overlap=120)
    chunks = splitter.split_documents(docs)
    embeddings = HuggingFaceEmbeddings(model_name="BAAI/bge-small-en-v1.5")
    store = Chroma.from_documents(chunks, embeddings, persist_directory=persist_dir, collection_name="book")
    keywords = BM25Retriever.from_documents(chunks, k=8)
    return store, keywords, len(chunks)
