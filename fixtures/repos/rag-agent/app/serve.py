"""Serve the model with vLLM. These settings decide how GPU memory is used."""
from vllm import LLM, SamplingParams

engine = LLM(
    model="Qwen/Qwen3-4B",
    max_model_len=8192,
    gpu_memory_utilization=0.85,
    enable_prefix_caching=True,
    kv_cache_dtype="fp8",
)


def answer(prompts: list[str]):
    params = SamplingParams(temperature=0.0, max_tokens=512)
    return engine.generate(prompts, params)
