import pytesseract
from pypdf import PdfReader

from .config import CHUNK_CHARS
from .embed_client import embed
from .pg_store import insert_chunks


def read_book(path):
    reader = PdfReader(path)
    return [p.extract_text() or pytesseract.image_to_string(p) for p in reader.pages]


def chunk_pages(pages, book_id):
    out = []
    for page_no, text in enumerate(pages):
        for i in range(0, len(text), CHUNK_CHARS):
            out.append({"book_id": book_id, "page_no": page_no, "text": text[i : i + CHUNK_CHARS]})
    return out


def ingest(path, book_id):
    chunks = chunk_pages(read_book(path), book_id)
    insert_chunks(chunks, embed([c["text"] for c in chunks]))
