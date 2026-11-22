#!/usr/bin/env python3
"""Try koala-rust over MCP (stdio). No third-party packages needed.

Usage: python3 examples/demo.py ./koala-rust
Uses a throwaway state dir, so your real ~/.koala is not touched.
First run downloads the embedding model (~30 MB) once; later runs work offline.
"""
import json, os, subprocess, sys, tempfile, time

binary = sys.argv[1] if len(sys.argv) > 1 else "koala-rust"
state = tempfile.mkdtemp(prefix="koala-demo-")
proc = subprocess.Popen([binary, "--serve"], stdin=subprocess.PIPE,
                        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                        text=True, env=dict(os.environ, KOALA_STATE_DIR=state))
_id = 0

def rpc(method, params=None):
    global _id
    _id += 1
    msg = {"jsonrpc": "2.0", "id": _id, "method": method, "params": params or {}}
    proc.stdin.write(json.dumps(msg) + "\n")
    proc.stdin.flush()
    return json.loads(proc.stdout.readline())

def tool(tool_name, **args):
    out = rpc("tools/call", {"name": tool_name, "arguments": args})
    return json.loads(out["result"]["content"][0]["text"])

def rss_mb():
    with open(f"/proc/{proc.pid}/status") as f:
        return int(f.read().split("VmRSS:")[1].split()[0]) // 1024

rpc("initialize", {"protocolVersion": "2024-11-05", "capabilities": {},
                   "clientInfo": {"name": "demo", "version": "0"}})
print("MCP tools:", len(rpc("tools/list")["result"]["tools"]))

SAMPLE = {  # made-up data, one example per sector
    "health/cardio": "Patient on warfarin, avoid NSAIDs due to bleeding risk",
    "health/ortho": "Knee replacement scheduled, physio twice weekly",
    "finance/desk": "Equity desk restricted list includes ACME Corp until merger closes",
}
for name, text in SAMPLE.items():
    t = time.time()
    tool("remember_semantic", name=name, text=text)
    print(f"stored {name:14s} {time.time() - t:.2f}s")

for query in ("which medicines are unsafe with blood thinners",
              "which companies can traders not touch"):
    hit = tool("recall", query=query, k=1)["results"][0]
    print(f"\nQ: {query}\n -> {hit['name']}: {hit['text']}")

print(f"\nProcess memory: {rss_mb()} MB   state dir: {state}")
proc.terminate()
