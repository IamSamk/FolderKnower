"""A tiny GPT: token embedding -> N blocks (attention + MLP) -> logits."""
import torch
from torch import nn


class CausalSelfAttention(nn.Module):
    def __init__(self, d_model: int, n_heads: int):
        super().__init__()
        self.attn = nn.MultiheadAttention(embed_dim=d_model, num_heads=n_heads, batch_first=True)

    def forward(self, x, cache=None):
        if cache is not None:
            x = torch.cat([cache, x], dim=1)
        out, _ = self.attn(x, x, x, need_weights=False)
        return out, x


class MLP(nn.Module):
    def __init__(self, d_model: int):
        super().__init__()
        self.up = nn.Linear(d_model, 4 * d_model)
        self.down = nn.Linear(4 * d_model, d_model)

    def forward(self, x):
        return self.down(torch.nn.functional.gelu(self.up(x)))


class Block(nn.Module):
    def __init__(self, d_model: int, n_heads: int):
        super().__init__()
        self.norm1 = nn.LayerNorm(d_model)
        self.attn = CausalSelfAttention(d_model, n_heads)
        self.norm2 = nn.LayerNorm(d_model)
        self.mlp = MLP(d_model)

    def forward(self, x):
        a, _ = self.attn(self.norm1(x))
        x = x + a
        return x + self.mlp(self.norm2(x))


class TinyGPT(nn.Module):
    def __init__(self, vocab: int = 32000, d_model: int = 256, n_heads: int = 4, n_layers: int = 4):
        super().__init__()
        self.embed = nn.Embedding(num_embeddings=vocab, embedding_dim=d_model)
        self.blocks = nn.ModuleList([Block(d_model, n_heads) for _ in range(n_layers)])
        self.head = nn.Linear(d_model, vocab)

    def forward(self, ids):
        x = self.embed(ids)
        for b in self.blocks:
            x = b(x)
        return self.head(x)
