#!/usr/bin/env python3
"""Capture and verify the v0.3.3 CLI, MCP tool, and SQLite schema contracts."""
import argparse
import difflib
import json
import os
from pathlib import Path
import re
import sqlite3
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
SNAPSHOTS = ROOT / "tests" / "snapshots"
BINARY = Path(os.environ.get("KOALA_BIN", ROOT / "target" / "debug" / "koala-rust")).resolve()
VEC0 = Path(os.environ.get("KOALA_VEC0_PATH", "/usr/local/lib/vec0.so")).resolve()
VECTOR = ",".join(["0"] * 256)


def normalize(value, state_dir):
    if isinstance(value, float):
        return "<NUMBER>"
    if isinstance(value, str):
        value = value.replace(str(state_dir), "<STATE_DIR>")
        value = re.sub(r"\d{4}-\d\d-\d\d[T ][^\s\"]+", "<TIMESTAMP>", value)
        return value
    if isinstance(value, list):
        result = [normalize(item, state_dir) for item in value]
        if result and all(isinstance(item, dict) and "name" in item for item in result):
            result.sort(key=lambda item: item["name"])
        return result
    if isinstance(value, dict):
        return {
            key: "<TIMESTAMP>" if key in {"created_at", "updated_at", "deleted_at", "loaded_at", "timestamp"}
            else "<UUID>" if key in {"peer_id", "from_peer"}
            else "<NUMBER>" if key == "pages"
            else normalize(item, state_dir)
            for key, item in sorted(value.items())
        }
    return value


def environment(state_dir):
    env = os.environ.copy()
    env["KOALA_STATE_DIR"] = str(state_dir)
    env["KOALA_VEC0_PATH"] = str(VEC0)
    hf_home = Path(os.environ.get("KOALA_SNAPSHOT_HF_HOME", state_dir / "hf"))
    env["HF_HOME"] = str(hf_home)
    env["HF_HUB_CACHE"] = str(hf_home / "hub")
    env["HUGGINGFACE_HUB_CACHE"] = str(hf_home / "hub")
    env["TRANSFORMERS_CACHE"] = str(hf_home / "transformers")
    env["HF_HUB_DISABLE_TELEMETRY"] = "1"
    env["HF_HUB_OFFLINE"] = "1"
    env["TRANSFORMERS_OFFLINE"] = "1"
    env["KOALA_EMBED_MODEL"] = "minishlab/potion-base-8M"
    return env


def json_text(value):
    return json.dumps(value, indent=2, sort_keys=True) + "\n"


def capture_tools():
    state = Path(tempfile.mkdtemp(prefix="koala-tools-snapshot-"))
    env = environment(state)
    proc = subprocess.Popen(
        [str(BINARY), "--serve"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, text=True, env=env, cwd=ROOT,
    )
    request_id = 0

    def rpc(method, params=None):
        nonlocal request_id
        request_id += 1
        request = {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params or {}}
        proc.stdin.write(json.dumps(request) + "\n")
        proc.stdin.flush()
        while True:
            line = proc.stdout.readline()
            if not line:
                raise RuntimeError(f"MCP server exited while handling {method}")
            response = json.loads(line)
            if response.get("id") == request_id:
                if "error" in response:
                    raise RuntimeError(response["error"])
                return response["result"]

    try:
        rpc("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "koala-snapshot", "version": "1"},
        })
        tools = rpc("tools/list")["tools"]
        tools.sort(key=lambda tool: tool["name"])
        if len(tools) < 53:
            raise RuntimeError(f"Expected at least 53 MCP tools, got {len(tools)}")
        return tools
    finally:
        proc.stdin.close()
        proc.wait(timeout=30)


def command_help():
    def names(text):
        section = text.split("Commands:", 1)[-1]
        found = []
        for line in section.splitlines():
            match = re.match(r"^\s{2}([a-z][a-z0-9-]*)\s{2,}", line)
            if match:
                found.append(match.group(1))
        return found

    result = {}
    root_help = subprocess.run([str(BINARY), "--help"], check=True, capture_output=True, text=True)
    for command in names(root_help.stdout):
        if command == "help":
            continue
        help_result = subprocess.run(
            [str(BINARY), command, "--help"], check=True, capture_output=True, text=True
        )
        result[command] = help_result.stdout
        if command == "context":
            for action in names(help_result.stdout):
                if action == "help":
                    continue
                nested = subprocess.run(
                    [str(BINARY), "context", action, "--help"],
                    check=True, capture_output=True, text=True,
                )
                result[f"context {action}"] = nested.stdout
    result["<root>"] = root_help.stdout
    return result


def run_cli(state, *args):
    result = subprocess.run(
        [str(BINARY), *args, "--format", "json"],
        capture_output=True, text=True, timeout=180,
        env=environment(state), cwd=ROOT,
    )
    try:
        output = json.loads(result.stdout) if result.stdout.strip() else None
    except json.JSONDecodeError:
        output = result.stdout.strip()
    if result.returncode:
        error_lines = [
            line for line in result.stderr.splitlines()
            if line.startswith(("Error:", "Caused by:"))
        ]
        if not error_lines:
            error_lines = result.stderr.strip().splitlines()[-5:]
        return {
            "exit_code": result.returncode,
            "stdout": normalize(output, state),
            "stderr": normalize("\n".join(error_lines), state),
        }
    return {"exit_code": 0, "output": normalize(output, state)}


def capture_cli():
    state = Path(tempfile.mkdtemp(prefix="koala-cli-snapshot-"))
    results = {}

    def record(name, *args):
        results[name] = run_cli(state, *args)
        return results[name].get("output")

    fact = record("fact-write", "fact-write", "--name", "snapshot-fact", "--text", "baseline fact")
    fact_id = fact["id"]
    record("fact-read", "fact-read", "--name", "snapshot-fact")
    record("fact-versions-initial", "fact-versions", "--name", "snapshot-fact")
    time.sleep(1.1)  # SQLite CURRENT_TIMESTAMP has one-second resolution.
    record("fact-write-version", "fact-write", "--name", "snapshot-fact", "--text", "updated baseline fact")
    record("fact-versions-updated", "fact-versions", "--name", "snapshot-fact")
    record("fact-list", "fact-list")
    for suffix in ("a", "b"):
        record(f"fact-prefix-write-{suffix}", "fact-write", "--name", f"snapshot-prefix-{suffix}", "--text", f"prefix {suffix}")
    record("fact-delete-prefix", "fact-delete-prefix", "--prefix", "snapshot-prefix-")

    record("entity-create", "entity-create", "--name", "snapshot-entity", "--type", "note", "--project", "snapshot", "--observations", "alpha,beta")
    record("entity-create-related", "entity-create", "--name", "snapshot-related", "--type", "note", "--project", "snapshot")
    record("entity-add-obs", "entity-add-obs", "--name", "snapshot-entity", "--observations", "gamma,delta")
    record("entity-open", "entity-open", "--names", "snapshot-entity,snapshot-related")
    record("entity-search", "entity-search", "--query", "snapshot-entity")
    record("relation-create", "relation-create", "--from", "snapshot-entity", "--to", "snapshot-related", "--type", "related")

    task = record("task-create", "task-create", "--title", "snapshot task", "--body", "baseline task", "--status", "not_started", "--project", "snapshot", "--priority", "medium")
    task_id = task["id"]
    record("task-query", "task-query", "--project", "snapshot")
    record("task-update", "task-update", "--id", str(task_id), "--status", "done")

    record("model-init", "model-init", "--hidden", "4", "--activation", "tanh", "--lr", "0.01")
    record("model-train", "model-train", "--xs", "0,1", "--ys", "0,1", "--targets", "0,1", "--steps", "2")
    record("model-predict", "model-predict", "--xs", "0,1", "--ys", "0,1")
    record("model-status", "model-status")
    record("model-save", "model-save", "--step", "2")
    record("model-load", "model-load", "--step", "2")
    record("embed-text", "embed-text", "--text", "snapshot embedding")
    record("embed-batch", "embed-batch", "--texts", "alpha,beta")
    record("vector-upsert-fact", "vector-upsert-fact", "--fact-id", str(fact_id), "--embedding", VECTOR)
    record("vector-search-facts", "vector-search-facts", "--query-embedding", VECTOR, "--k", "3")
    record("sync-push", "sync-push")
    record("sync-pull", "sync-pull")
    record("sync-status", "sync-status")

    record("plugin-search", "plugin-search", "--query", "hash")
    record("plugin-install", "plugin-install", "--name", "hash-tools")
    record("plugin-call", "plugin-call", "--plugin", "hash-tools", "--tool", "hash", "--args", '{"algorithm":"sha256","text":"snapshot"}')
    record("plugin-registry-stats", "plugin-registry-stats")
    record("plugin-uninstall", "plugin-uninstall", "--name", "hash-tools")

    record("think", "think", "--step", "1", "--thought", "snapshot thought", "--next-step", "2")
    record("thought-history", "thought-history")
    record("thought-reset", "thought-reset")
    record("conclusion", "conclusion", "--summary", "snapshot conclusion")
    record("feedback", "feedback", "--name", "snapshot-entity", "--signal", "positive", "--kind", "entity")
    record("explain-rank", "explain-rank", "--name", "snapshot-entity")
    record("health", "health")
    record("sql", "sql", "SELECT 1 AS value, 'snapshot' AS label")
    record("context-bootstrap", "context", "bootstrap", "--project", "snapshot")
    record("context-checkpoint", "context", "checkpoint", "--project", "snapshot", "--summary", "baseline", "--next", "continue")
    record("context-decision", "context", "decision", "--project", "snapshot", "--decision", "keep behavior", "--rationale", "baseline")
    record("context-status", "context", "status", "--project", "snapshot", "--status", "active", "--blockers", "none", "--next", "continue")
    record("context-get-user", "context", "get-user")
    record("context-get-project", "context", "get-project", "--project", "snapshot")
    record("fact-delete", "fact-delete", "--name", "snapshot-fact")

    # This command contacts the public registry; its CLI contract is covered by help.
    skipped = {"plugin-update-registry": "contacts the public registry"}
    db = state / "koala.db"
    connection = sqlite3.connect(db)
    rows = connection.execute(
        "SELECT type, name, tbl_name, sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY type, name"
    ).fetchall()
    connection.close()
    schema = "\n\n".join(
        f"-- {kind}: {name} ({table})\n{statement};" for kind, name, table, statement in rows
    ) + "\n"
    return results, skipped, schema


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--update", action="store_true", help="write current outputs as snapshots")
    args = parser.parse_args()
    if not BINARY.is_file():
        raise FileNotFoundError(f"KOALA_BIN not found: {BINARY}")
    if not VEC0.is_file():
        raise FileNotFoundError(f"KOALA_VEC0_PATH not found: {VEC0}")
    SNAPSHOTS.mkdir(parents=True, exist_ok=True)

    captured = {
        "mcp-tools.json": json_text(capture_tools()),
        "cli-help.json": json_text(command_help()),
    }
    cli, skipped, schema = capture_cli()
    captured["cli-commands.json"] = json_text({"commands": cli, "not_run": skipped})
    captured["schema.sql"] = schema

    failed = False
    for filename, content in captured.items():
        path = SNAPSHOTS / filename
        if args.update:
            path.write_text(content)
            print(f"UPDATED {path.relative_to(ROOT)}")
        elif not path.exists():
            print(f"MISSING snapshot: {path.relative_to(ROOT)}")
            failed = True
        else:
            expected = path.read_text()
            if expected != content:
                print(f"DIFF {path.relative_to(ROOT)}")
                print("".join(difflib.unified_diff(
                    expected.splitlines(True), content.splitlines(True),
                    fromfile="expected", tofile="actual",
                ))[:12000])
                failed = True
            else:
                print(f"PASS {path.relative_to(ROOT)}")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
