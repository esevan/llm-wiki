"""Optional semantic reranking; importing it never loads an ONNX model."""

from __future__ import annotations

import math
from collections.abc import Sequence


class SemanticUnavailable(RuntimeError):
    pass


class SemanticEmbedder:
    def __init__(self) -> None:
        try:
            from fastembed import TextEmbedding
        except ImportError as error:
            raise SemanticUnavailable("Install the semantic optional dependency") from error
        self._model = TextEmbedding(model_name="sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2")

    def embed(self, texts: Sequence[str]) -> list[list[float]]:
        return [list(vector) for vector in self._model.embed(list(texts))]


def cosine(left: Sequence[float], right: Sequence[float]) -> float:
    denominator = math.sqrt(sum(item * item for item in left)) * math.sqrt(sum(item * item for item in right))
    return sum(a * b for a, b in zip(left, right)) / denominator if denominator else 0.0
