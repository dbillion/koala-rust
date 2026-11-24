#!/usr/bin/env python3
"""End-to-end test: drives the MCP server over stdio (stdlib only).

Needs the sqlite3 CLI and the sqlite-vec extension (KOALA_VEC0_PATH).
Binary: KOALA_BIN, default target/debug/koala-rust.
Exit code 0 when every check passes.
"""
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.environ.get("KOALA_BIN", os.path.join(ROOT, "target", "debug", "koala-rust"))
VEC0 = os.environ.get("KOALA_VEC0_PATH", "/usr/local/lib/vec0.so")
TMP = tempfile.mkdtemp(prefix="kbe2e-")
results = []


def start(name, extra=None):
    state = os.path.join(TMP, name)
    env = dict(os.environ, KOALA_STATE_DIR=state)
    env.update(extra or {})
    # A deliberate remote-model override must supersede a shared local model
    # path so the degraded-start/reindex-retry scenario exercises failure.
    if extra and "KOALA_EMBED_MODEL" in extra:
        env.pop("KOALA_MODEL_DIR", None)
    p = subprocess.Popen([BIN, "--serve"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                         stderr=subprocess.DEVNULL, text=True, env=env)
    n = {"id": 0}

    def rpc(method, params=None):
        n["id"] += 1
        p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": n["id"], "method": method,
                                  "params": params or {}}) + "\n")
        p.stdin.flush()
        while True:
            line = p.stdout.readline()
            if not line:
                raise RuntimeError("server closed stdout")
            d = json.loads(line)
            if d.get("id") == n["id"]:
                return d

    def call(tool, args=None):
        d = rpc("tools/call", {"name": tool, "arguments": args or {}})
        if "error" in d:
            return {"ERR": d["error"]["message"]}
        return json.loads(d["result"]["content"][0]["text"])

    info = rpc("initialize", {"protocolVersion": "2024-11-05", "capabilities": {},
                              "clientInfo": {"name": "e2e", "version": "1"}})
    return {"p": p, "rpc": rpc, "call": call, "info": info, "db": os.path.join(state, "koala.db")}


def stop(s):
    s["p"].stdin.close()
    s["p"].wait(timeout=30)


def check(label, cond, extra=""):
    results.append(bool(cond))
    print(("PASS " if cond else "FAIL ") + label, extra)


def sql(db, query):
    r = subprocess.run(["sqlite3", db, query], capture_output=True, text=True)
    return r.stdout.strip()


def vcount(db, rowid):
    r = subprocess.run(["sqlite3", db, "-cmd", ".load " + re.sub(r"\.so$", "", VEC0),
                        f"select count(*) from fact_embeddings where rowid={rowid};"],
                       capture_output=True, text=True)
    out = r.stdout.strip().splitlines()
    return out[-1] if out else "ERR:" + r.stderr.strip()[:80]


def meta(db, key):
    return sql(db, f"select value from brain_meta where key='{key}'")


def main():
    cargo = open(os.path.join(ROOT, "Cargo.toml")).read()
    version = re.search(r'^version\s*=\s*"([^"]+)"', cargo, re.M).group(1)

    # ---- basics, semantic recall, versioning, reindex, sync doc_type
    s = start("main")
    call, rpc = s["call"], s["rpc"]
    check("serverInfo version matches Cargo.toml", s["info"]["result"]["serverInfo"]["version"] == version)
    tools = rpc("tools/list")["result"]["tools"]
    names = {t["name"] for t in tools}
    check("at least 53 tools", len(tools) >= 53, len(tools))
    check("semantic tools listed", {"remember_semantic", "recall", "reindex_facts"} <= names)
    sync = [t for t in tools if t["name"] == "sync_get_changes"][0]
    check("sync doc_type enum in schema", "enum" in sync["inputSchema"]["properties"]["doc_type"])

    facts = {
        "refund_policy": "Customers can get a full refund within 30 days of purchase if the product is unused.",
        "deploy_runbook": "To deploy, run the CI pipeline, then promote the Docker image to production Kubernetes with a canary rollout.",
        "vpn_setup": "Employees connect to the corporate VPN using WireGuard and their SSO login.",
        "pricing_tier": "The enterprise plan costs 49 dollars per seat per month with annual billing.",
        "oncall": "The on-call engineer rotates weekly and is paged through PagerDuty for severity one incidents.",
    }
    for k, v in facts.items():
        assert "rowid" in call("remember_semantic", {"name": k, "text": v})
    qs = [("how do I get my money back", "refund_policy"), ("ship new code to prod", "deploy_runbook"),
          ("remote access to company network", "vpn_setup"), ("how much does it cost", "pricing_tier"),
          ("who handles outages at night", "oncall"), ("WireGuard", "vpn_setup"), ("PagerDuty", "oncall")]
    top1 = 0
    for q, exp in qs:
        got = [h["name"] for h in call("recall", {"query": q, "k": 2}).get("results", [])]
        top1 += bool(got) and got[0] == exp
        print(f"  {q!r:40} -> {got}")
    check(f"recall top1 {top1}/{len(qs)} (>=6 expected)", top1 >= 6)

    call("remember_semantic", {"name": "pricing_tier", "text": "The enterprise plan now costs 79 dollars per seat per month."})
    r = call("recall", {"query": "enterprise plan price", "k": 5})["results"]
    texts = [h["text"] for h in r if h["name"] == "pricing_tier"]
    check("updated fact: only new version returned", len(texts) == 1 and "79" in texts[0], texts)
    check("reindex_facts", call("reindex_facts").get("reindexed") == 5)
    check("recall works after reindex",
          any(h["name"] == "pricing_tier" for h in call("recall", {"query": "enterprise plan price", "k": 5})["results"]))
    call("sync_write_fact", {"name": "shared", "text": "hello peer"})
    check("sync_get_changes doc_type facts",
          isinstance(call("sync_get_changes", {"doc_type": "facts"}).get("changes"), list))

    # ---- keyword path finds an exact token outside the vector pool
    for i in range(60):
        call("remember_semantic", {"name": f"filler_{i}",
                                   "text": f"Meeting notes number {i} about quarterly planning, budgets and office logistics."})
    call("remember_semantic", {"name": "ticket_zq7x", "text": "Reference zq7x covers the badge reader replacement."})
    r = call("recall", {"query": "zq7x", "k": 3, "keyword_weight": 1.0}).get("results", [])
    check("keyword path finds exact token outside vector pool", any(h["name"] == "ticket_zq7x" for h in r))

    # ---- delete removes the vector too
    rid = call("remember_semantic", {"name": "temp_del", "text": "Temporary fact about zebra crossings."})["rowid"]
    check("vector exists before delete", vcount(s["db"], rid) == "1", vcount(s["db"], rid))
    check("delete_fact deleted", call("delete_fact", {"name": "temp_del"}).get("deleted") is True)
    check("vector removed with fact", vcount(s["db"], rid) == "0", vcount(s["db"], rid))

    # Permanent deletion must remove every historical version, not just the
    # current row, and secure_delete + incremental vacuum must clear the text.
    old_marker = "purge-history-old-marker-9c71d2"
    new_marker = "purge-history-new-marker-3a4f8b"
    old_id = call("remember_semantic", {"name": "purge_history", "text": old_marker})["rowid"]
    new_id = call("remember_semantic", {"name": "purge_history", "text": new_marker})["rowid"]
    check("both versions have vectors", vcount(s["db"], old_id) == "0" and vcount(s["db"], new_id) == "1")
    check("purge removes all versions", call("delete_fact", {"name": "purge_history"}).get("deleted") is True
          and sql(s["db"], "select count(*) from facts where name='purge_history'") == "0")
    raw_db = open(s["db"], "rb").read()
    check("purge clears old plaintext from database file",
          old_marker.encode() not in raw_db and new_marker.encode() not in raw_db)
    check("purge leaves text-free tombstone",
          sql(s["db"], "select count(*) from facts_tombstone where name='purge_history'") == "1")

    a = call("remember_semantic", {"name": "tmpgrp_a", "text": "Group fact alpha about giraffes."})["rowid"]
    b = call("remember_semantic", {"name": "tmpgrp_b", "text": "Group fact beta about giraffes."})["rowid"]
    call("remember_semantic", {"name": "tmpgrp_a", "text": "Updated group fact alpha."})
    call("delete_fact_prefix", {"prefix": "tmpgrp_"})
    check("prefix vectors removed", vcount(s["db"], a) == "0" and vcount(s["db"], b) == "0")
    check("prefix purge removes historical rows",
          sql(s["db"], "select count(*) from facts where name like 'tmpgrp_%'") == "0")
    r = call("recall", {"query": "giraffes", "k": 5}).get("results", [])
    check("deleted facts not recalled",
          not any(h["name"].startswith("tmpgrp_") or h["name"] == "temp_del" for h in r))

    # ---- missing argument is an error, not a crash
    check("missing argument returns an error", "ERR" in call("write_fact", {"name": "x"}))
    check("server still alive after bad call", call("health") is not None)
    stop(s)

    # ---- model change with a failing model: degraded start, marker stays, then resumes
    s = start("main", {"KOALA_EMBED_MODEL": "nonexistent-org/no-such-model"})
    check("degraded start still serves", s["info"].get("result", {}).get("serverInfo", {}).get("name") is not None)
    stop(s)
    time.sleep(0.5)
    check("failed reindex leaves marker pending", meta(s["db"], "reindex_pending") == "1")
    s = start("main")
    r = s["call"]("recall", {"query": "WireGuard", "k": 2}).get("results", [])
    check("resumed reindex: recall works", any(h["name"] == "vpn_setup" for h in r))
    stop(s)
    time.sleep(0.5)
    check("resumed reindex clears marker", meta(s["db"], "reindex_pending") == "0")

    # ---- dimension migration: DB created at 384 dims, reopened at the default 256
    s = start("mig", {"KOALA_EMBED_DIM": "384"})
    s["call"]("write_fact", {"name": "refund_policy", "text": "Full refund within 30 days."})
    s["call"]("write_fact", {"name": "vpn_setup", "text": "Use WireGuard for the VPN."})
    stop(s)
    s = start("mig")
    r = s["call"]("recall", {"query": "money back", "k": 2}).get("results", [])
    check("384-dim DB auto-migrated and re-embedded", [h["name"] for h in r][:1] == ["refund_policy"])
    stop(s)
    time.sleep(0.5)
    check("migration: reindex marker cleared", meta(s["db"], "reindex_pending") == "0")
    check("migration: embed_model stored", meta(s["db"], "embed_model").endswith("|256"))

    # ---- two-peer CRDT sync via MCP: A -> B
    pa, pb = start("peerA"), start("peerB")
    pa["call"]("sync_write_fact", {"name": "shared:decision", "text": "Use exact KNN"})
    chg = pa["call"]("sync_get_changes", {"doc_type": "facts"}).get("changes", [])
    check("peer A has changes", len(chg) > 0)
    check("peer B merge ok", "ERR" not in pb["call"]("sync_merge_changes", {"doc_type": "facts", "changes": chg}))
    listed = str(pb["call"]("sync_list_facts"))
    check("peer B sees A fact after merge", "shared:decision" in listed)
    pb["call"]("sync_merge_changes", {"doc_type": "facts", "changes": chg})
    check("re-merge is idempotent",
          str(pb["call"]("sync_list_facts")).count("shared:decision") == listed.count("shared:decision"))
    stop(pa)
    stop(pb)

    # ---- raw sql CLI test
    out = subprocess.check_output([BIN, "--state-dir", os.path.join(TMP, "mig"), "sql",
                                  "SELECT 1 as num, 'test' as txt, 2.5 as flt", "-f", "json"], text=True)
    sql_rows = json.loads(out)
    check("sql CLI decodes primitives", sql_rows == [{"flt": 2.5, "num": 1, "txt": "test"}])

    # ---- independent processes write concurrently to one SQLite database
    concurrent_state = os.path.join(TMP, "multi-process")
    concurrent_env = dict(os.environ, KOALA_STATE_DIR=concurrent_state)
    seed = subprocess.run(
        [BIN, "--state-dir", concurrent_state, "fact-write", "--name", "multi_seed", "--text", "seed"],
        capture_output=True, text=True, env=concurrent_env, timeout=30,
    )
    check("multi-process database initialized", seed.returncode == 0, seed.stderr[:160])
    writers = [
        subprocess.Popen(
            [BIN, "--state-dir", concurrent_state, "fact-write", "--name", f"multi_writer_{i}",
             "--text", f"concurrent writer payload {i}"],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=concurrent_env,
        )
        for i in range(2)
    ]
    writer_results = [writer.communicate(timeout=30) + (writer.returncode,) for writer in writers]
    check("two concurrent CLI writers both succeed",
          all(result[2] == 0 for result in writer_results),
          [result[1][:120] for result in writer_results])
    concurrent_db = os.path.join(concurrent_state, "koala.db")
    check("both concurrent writes are durable",
          sql(concurrent_db, "select count(*) from facts where name like 'multi_writer_%'") == "2")

    # ---- AI continuity & context lifecycle test
    c_server = start("continuity")
    c_server["call"]("write_fact", {"name": "user_stack", "text": "Rust + TypeScript + MCP"})
    c_server["call"]("record_decision", {
        "project": "harvestconnect",
        "decision": "Use koala-rust for persistent memory",
        "rationale": "High-performance Rust SQLite + CRDT sync"
    })
    c_server["call"]("create_handoff", {
        "project": "harvestconnect",
        "summary": "Completed initial continuity implementation",
        "next_step": "Deploy and test CLI",
        "blockers": ["none"]
    })
    boot = c_server["call"]("bootstrap_context", {"project": "harvestconnect"})
    check("bootstrap has user stack", "Rust + TypeScript + MCP" in boot["user"]["stack"])
    check("bootstrap has latest handoff", boot["project"]["latest_handoff"]["summary"] == "Completed initial continuity implementation")
    check("bootstrap has recent decision", any("koala-rust" in str(d) for d in boot["project"]["recent_decisions"]))
    mem_res = c_server["call"]("search_memory", {"query": "initial continuity", "project": "harvestconnect"})
    check("search_memory returns relevant handoff", len(mem_res) > 0 and "harvestconnect" in mem_res[0]["name"])
    stop(c_server)

    # CLI context commands
    c_dir = os.path.join(TMP, "continuity")
    cp_out = subprocess.check_output([BIN, "--state-dir", c_dir, "context", "checkpoint",
                                      "--project", "harvestconnect",
                                      "--summary", "CLI checkpoint test",
                                      "--next", "Verify bootstrap",
                                      "-f", "json"], text=True)
    cp_data = json.loads(cp_out)
    check("cli context checkpoint", cp_data["summary"] == "CLI checkpoint test")

    boot_out = subprocess.check_output([BIN, "--state-dir", c_dir, "context", "bootstrap",
                                        "--project", "harvestconnect", "-f", "json"], text=True)
    boot_data = json.loads(boot_out)
    check("cli context bootstrap returns checkpoint",
          boot_data["project"]["latest_handoff"]["summary"] == "CLI checkpoint test")


if __name__ == "__main__":
    try:
        main()
    finally:
        shutil.rmtree(TMP, ignore_errors=True)
    ok = all(results)
    print("ALL PASS" if ok else "SOME FAIL")
    sys.exit(0 if ok else 1)
