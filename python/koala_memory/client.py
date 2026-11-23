"""Small synchronous MCP stdio client for koala-rust. Standard library only."""
from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path
from typing import Any, Mapping


class KoalaError(RuntimeError):
    """Raised when the Koala process or an MCP tool returns an error."""


class KoalaMCPClient:
    def __init__(
        self,
        binary: str | os.PathLike[str] = "koala-rust",
        *,
        state_dir: str | os.PathLike[str],
        env: Mapping[str, str] | None = None,
    ) -> None:
        process_env = os.environ.copy()
        process_env["KOALA_STATE_DIR"] = str(state_dir)
        if env:
            process_env.update(env)
        self._process = subprocess.Popen(
            [str(binary), "--serve"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            encoding="utf-8",
            bufsize=1,
            env=process_env,
        )
        self._request_id = 0
        self._closed = False
        self._request(
            "initialize",
            {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "koala-python", "version": "0.1.0"},
            },
        )
        self._notify("notifications/initialized")

    def _send(self, message: dict[str, Any]) -> None:
        if self._closed or self._process.stdin is None:
            raise KoalaError("Koala client is closed")
        self._process.stdin.write(json.dumps(message, separators=(",", ":")) + "\n")
        self._process.stdin.flush()

    def _request(self, method: str, params: dict[str, Any] | None = None) -> dict[str, Any]:
        self._request_id += 1
        request_id = self._request_id
        self._send({"jsonrpc": "2.0", "id": request_id, "method": method, "params": params or {}})
        if self._process.stdout is None:
            raise KoalaError("Koala stdout is unavailable")
        while True:
            line = self._process.stdout.readline()
            if not line:
                code = self._process.poll()
                raise KoalaError(f"Koala server closed stdout (exit={code})")
            try:
                response = json.loads(line)
            except json.JSONDecodeError:
                continue
            if response.get("id") != request_id:
                continue
            if "error" in response:
                raise KoalaError(response["error"].get("message", str(response["error"])))
            return response.get("result", {})

    def _notify(self, method: str, params: dict[str, Any] | None = None) -> None:
        self._send({"jsonrpc": "2.0", "method": method, "params": params or {}})

    def call_tool(self, name: str, arguments: dict[str, Any] | None = None) -> Any:
        result = self._request("tools/call", {"name": name, "arguments": arguments or {}})
        if result.get("isError"):
            content = result.get("content", [])
            message = next((item.get("text", "tool failed") for item in content if item.get("type") == "text"), "tool failed")
            raise KoalaError(message)
        for item in result.get("content", []):
            if item.get("type") == "text":
                try:
                    return json.loads(item["text"])
                except (KeyError, json.JSONDecodeError):
                    return item.get("text")
        return result

    def remember(self, name: str, text: str, *, ttl_seconds: int | None = None) -> dict[str, Any]:
        arguments: dict[str, Any] = {"name": name, "text": text}
        if ttl_seconds is not None:
            arguments["ttl_seconds"] = ttl_seconds
        return self.call_tool("remember_semantic", arguments)

    def recall(self, query: str, *, k: int = 5, keyword_weight: float = 0.35) -> list[dict[str, Any]]:
        result = self.call_tool("recall", {"query": query, "k": k, "keyword_weight": keyword_weight})
        return result.get("results", []) if isinstance(result, dict) else []

    def delete(self, name: str) -> dict[str, Any]:
        return self.call_tool("delete_fact", {"name": name})

    def delete_prefix(self, prefix: str) -> dict[str, Any]:
        return self.call_tool("delete_fact_prefix", {"prefix": prefix})

    def close(self) -> None:
        if self._closed:
            return
        self._closed = True
        if self._process.stdin:
            self._process.stdin.close()
        try:
            self._process.wait(timeout=30)
        except subprocess.TimeoutExpired:
            self._process.terminate()
            try:
                self._process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self._process.kill()
                self._process.wait()
        if self._process.stdout:
            self._process.stdout.close()

    def __enter__(self) -> "KoalaMCPClient":
        return self

    def __exit__(self, *_: object) -> None:
        self.close()
