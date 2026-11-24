"""Thin method-compatible adapters for LangChain-style memory and AutoGen tools.

No LangChain or AutoGen dependency is imported; applications can pass the adapter
methods into their framework's memory/tool registration APIs.
"""
from __future__ import annotations

import uuid
from typing import Any, Mapping

from .client import KoalaMCPClient


def _first_text(values: Mapping[str, Any]) -> str:
    for value in values.values():
        if isinstance(value, str):
            return value
    return str(dict(values))


class LangChainMemoryAdapter:
    """Implements the common BaseMemory method shape without a hard dependency."""

    def __init__(self, client: KoalaMCPClient, *, session_id: str = "default", k: int = 5, memory_key: str = "history") -> None:
        self.client = client
        self.session_id = session_id
        self.k = k
        self.memory_key = memory_key

    @property
    def memory_variables(self) -> list[str]:
        return [self.memory_key]

    def save_context(self, inputs: Mapping[str, Any], outputs: Mapping[str, Any]) -> None:
        question = _first_text(inputs)
        answer = _first_text(outputs)
        name = f"langchain/{self.session_id}/{uuid.uuid4().hex}"
        self.client.remember(name, f"User: {question}\nAssistant: {answer}")

    def load_memory_variables(self, inputs: Mapping[str, Any]) -> dict[str, str]:
        query = _first_text(inputs)
        hits = self.client.recall(query, k=self.k)
        history = "\n\n".join(hit.get("text", "") for hit in hits)
        return {self.memory_key: history}

    def clear(self) -> None:
        self.client.delete_prefix(f"langchain/{self.session_id}/")


class AutoGenMemoryAdapter:
    """Small add/query adapter for AutoGen conversation callbacks and tools."""

    def __init__(self, client: KoalaMCPClient, *, default_conversation: str = "default", k: int = 5) -> None:
        self.client = client
        self.default_conversation = default_conversation
        self.k = k

    def add(self, message: str, *, sender: str = "user", conversation_id: str | None = None) -> dict[str, Any]:
        conversation = conversation_id or self.default_conversation
        name = f"autogen/{conversation}/{uuid.uuid4().hex}"
        return self.client.remember(name, f"{sender}: {message}")

    def query(self, query: str, *, conversation_id: str | None = None, k: int | None = None) -> list[dict[str, Any]]:
        hits = self.client.recall(query, k=k or self.k)
        if conversation_id is None:
            return hits
        prefix = f"autogen/{conversation_id}/"
        return [hit for hit in hits if hit.get("name", "").startswith(prefix)]

    def clear(self, conversation_id: str | None = None) -> dict[str, Any]:
        conversation = conversation_id or self.default_conversation
        return self.client.delete_prefix(f"autogen/{conversation}/")
