"""An agent loop: the model decides to retrieve more context or to answer."""
from langgraph.graph import StateGraph, END
from langchain_core.tools import tool
from langchain_core.prompts import ChatPromptTemplate
from langchain_ollama import ChatOllama

from .retrieve import hybrid_search

llm = ChatOllama(model="qwen3:4b", temperature=0, num_ctx=8192, keep_alive="10m")

prompt = ChatPromptTemplate.from_messages([
    ("system", "Answer only from the context. Say 'not in the book' otherwise."),
    ("human", "Context:\n{context}\n\nQuestion: {question}"),
])


@tool
def search_book(question: str) -> str:
    """Search the book for passages about the question."""
    docs = hybrid_search(STORE, KEYWORDS, question)
    return "\n\n".join(d.page_content for d in docs)


def call_model(state: dict) -> dict:
    model = llm.bind_tools([search_book])
    reply = model.invoke(prompt.format_messages(context=state.get("context", ""), question=state["question"]))
    return {"messages": [reply], "steps": state.get("steps", 0) + 1}


def should_continue(state: dict) -> str:
    last = state["messages"][-1]
    if state.get("steps", 0) >= 4:
        return END
    if last.tool_calls:
        return "tools"
    return END


graph = StateGraph(dict)
graph.add_node("agent", call_model)
graph.add_node("tools", search_book)
graph.add_conditional_edges("agent", should_continue)
graph.add_edge("tools", "agent")
app = graph.compile()
