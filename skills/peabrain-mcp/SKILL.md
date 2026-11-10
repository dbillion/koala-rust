name: peabrain-mcp
version: 1.0.0
description: "Usage guide for peabrain-rust MCP server — tool reference, workflows, best practices, integration recipes"
category: integration
author: dbillion
tags:
  - mcp
  - memory
  - peabrain
  - vector-search
  - crdt
  - embeddings
  - plugins
  - hermes
  - claude

# Prerequisites
# - peabrain-rust binary installed and running as MCP server
# - MCP client (Claude Desktop, Hermes, custom)

---

# peabrain-mcp Skill

**Mastering peabrain-rust via MCP — the complete usage guide for agents.**

This skill teaches agents how to effectively use peabrain-rust's 50 MCP tools for persistent memory, semantic search, multi-agent sync, and plugin extensibility.

---

## 🎯 When to Use This Skill

- Agent needs to store/retrieve facts across sessions
- Agent needs semantic search over conversation history
- Multiple agents need to collaborate via CRDT sync
- Agent needs to install/use WASM plugins
- Setting up peabrain-rust with Claude, Hermes, or custom MCP client

---

## 📋 Tool Reference (All 50 Tools)

### Health
| Tool | Purpose |
|------|---------|
| `health` | Check server status |

### Facts (Versioned Key-Value)
| Tool | Purpose | Key Params |
|------|---------|------------|
| `write_fact` | Store a fact | `name`, `text` |
| `read_fact` | Read latest fact | `name` |
| `list_facts` | List all fact names | — |
| `read_fact_versions` | Read fact history | `name`, `limit` |
| `delete_fact` | Delete a fact | `name` |
| `delete_fact_prefix` | Delete by prefix | `prefix` |

### Entities & Knowledge Graph
| Tool | Purpose | Key Params |
|------|---------|------------|
| `create_entities` | Create entities with observations | `entities[]` (name, type, observations[]) |
| `add_observations` | Add observations to entities | `observations[]` (entity, contents[]) |
| `open_nodes` | Get entity details | `names[]` |
| `search_nodes` | Search entities | `query` |
| `create_relations` | Create relations | `relations[]` (from, to, type) |

### Tasks
| Tool | Purpose | Key Params |
|------|---------|------------|
| `create_task_or_note` | Create task/note | `step`, `thought`, `status?` |
| `query_tasks` | Query tasks | `status?`, `limit?` |
| `update_task` | Update task | `id`, `status?`, `step?`, `thought?` |

### Neural Model (Online Learning)
| Tool | Purpose |
|------|---------|
| `model_init` | Initialize model |
| `model_train` | Train on data |
| `model_predict` | Predict |
| `model_status` | Get status |
| `feedback` | Provide feedback |
| `explain_rank` | Explain ranking |
| `sequential_thinking` | Sequential reasoning |
| `conclusion` | Draw conclusion |
| `thought_history` | Get thought history |
| `thought_reset` | Reset thoughts |
| `model_save` | Save model |
| `model_load` | Load model |

### Vector Search (Semantic)
| Tool | Purpose | Key Params |
|------|---------|------------|
| `vector_upsert_fact` | Upsert fact vector | `fact_id`, `embedding[]` |
| `vector_upsert_entity` | Upsert entity vector | `entity_id`, `embedding[]` |
| `vector_search_facts` | Search facts by vector | `query_embedding[]`, `k?` |
| `vector_search_entities` | Search entities by vector | `query_embedding[]`, `k?` |
| `vector_delete_fact` | Delete fact vector | `fact_id` |
| `vector_delete_entity` | Delete entity vector | `entity_id` |

### CRDT Sync (Multi-Agent)
| Tool | Purpose | Key Params |
|------|---------|------------|
| `sync_write_fact` | Write fact for sync | `name`, `text`, `vector_clock?` |
| `sync_read_fact` | Read synced fact | `name` |
| `sync_list_facts` | List synced facts | — |
| `sync_create_entity` | Create entity for sync | `name`, `type`, `observations[]` |
| `sync_get_changes` | Get changes since clock | `peer`, `since_clock?` |
| `sync_merge_changes` | Merge peer changes | `changes[]`, `peer` |

### Embeddings
| Tool | Purpose | Key Params |
|------|---------|------------|
| `embed_text` | Generate embedding | `text` → `embedding[384]` |
| `embed_batch` | Batch embeddings | `texts[]` → `embeddings[][]` |

### Plugins
| Tool | Purpose | Key Params |
|------|---------|------------|
| `plugin_load` | Load plugin | `name`, `path?` |
| `plugin_call` | Call plugin tool | `name`, `tool`, `input` |
| `plugin_list` | List loaded plugins | — |
| `plugin_discover` | Discover available plugins | — |

### Marketplace
| Tool | Purpose | Key Params |
|------|---------|------------|
| `plugin_search` | Search registry | `query` |
| `plugin_install` | Install plugin | `name`, `version?` |
| `plugin_uninstall` | Uninstall plugin | `name` |
| `plugin_registry_stats` | Registry stats | — |
| `plugin_update_registry` | Update registry | — |

---

## 🔄 Common Workflow Patterns

### Pattern 1: Fact Extraction → Embed → Search
```python
# 1. Store fact
fact = write_fact("project:auth", "Using JWT with 15min expiry")

# 2. Generate embedding
emb = embed_text("project:auth uses JWT with 15min expiry")

# 3. Upsert vector for semantic search
vector_upsert_fact(fact_id=fact.rowid, embedding=emb.embedding)

# 4. Later: semantic recall
query_emb = embed_text("authentication token expiry").embedding
results = vector_search_facts(query_embedding=query_emb, k=5)
```

### Pattern 2: Entity Knowledge Graph
```python
create_entities([
    {"name": "AuthService", "type": "service", "observations": ["Handles JWT", "Port 8080"]},
    {"name": "UserDB", "type": "database", "observations": ["PostgreSQL", "Table: users"]}
])
create_relations([
    {"from": "AuthService", "to": "UserDB", "type": "reads"}
])
# Query: "What does AuthService connect to?"
search_nodes("AuthService")  # → Returns entity + relations
```

### Pattern 3: Multi-Agent Sync
```python
# Agent A writes
sync_write_fact("decision:vector_engine", "Use HNSW", vector_clock={"agent-a": 1})

# Agent B gets changes
changes = sync_get_changes("agent-a", since_clock={"agent-a": 0})
sync_merge_changes(changes, "agent-a")

# Both now have consistent state
```

### Pattern 4: Plugin Automation
```python
# Install plugin from marketplace
plugin_install("hash-tools")

# Use plugin tool
plugin_call("hash-tools", "sha256", "sensitive-data")
# → Returns hash without exposing data to agent context
```

---

## 🏗️ Integration Recipes

### Claude Desktop
```json
// ~/Library/Application Support/Claude/claude_desktop_config.json
{
  "mcpServers": {
    "peabrain": {
      "command": "/full/path/to/peabrain-rust",
      "args": ["--serve"],
      "env": { "PEABRAIN_STATE_DIR": "/Users/you/.peabrain" }
    }
  }
}
```

### Hermes
```yaml
# ~/.hermes/config.yaml
mcp_servers:
  peabrain-rust:
    command: "/full/path/to/peabrain-rust"
    args: ["--serve"]
    env:
      PEABRAIN_STATE_DIR: "/home/you/.peabrain"
    enabled: true

# For auto-conversation memory:
plugins:
  enabled:
    - peabrain-memory
memory:
  provider: peabrain
```

### Custom Python Client
```python
import json, subprocess, asyncio

async def call_peabrain(tool, args):
    proc = await asyncio.create_subprocess_exec(
        "/path/to/peabrain-rust", "--serve",
        stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE
    )
    req = json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": tool, "arguments": args}})
    proc.stdin.write((req + "\n").encode())
    await proc.stdin.drain()
    line = await proc.stdout.readline()
    return json.loads(line.decode())
```

---

## ✅ Best Practices

| Practice | Why |
|----------|-----|
| **Namespace facts** | Use `domain:key` (e.g., `project:config`, `team:decision`) to avoid collisions |
| **Embed everything searchable** | Call `embed_text` + `vector_upsert_fact` for any fact you'll need to find semantically |
| **Use entities for relationships** | Facts are key-value; entities + relations = queryable graph |
| **Sync with vector clocks** | Always pass `vector_clock` in multi-agent writes; merge on receipt |
| **Batch embeddings** | Use `embed_batch` for multiple texts — 10x faster than individual calls |
| **Plugin for sensitive ops** | Hashing, encryption, PII redaction → WASM plugin keeps data out of agent context |
| **Monitor vector clock growth** | Periodically compact: `sync_list_facts` → prune old versions |

---

## 🛠️ Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `sqlite-vec not loaded` | Extension missing | Rebuild with `sqlite-vec` or install `vec0.so` |
| `embedding dimension mismatch` | Wrong model | Ensure 384-dim (all-MiniLM-L6-v2) |
| `vector_clock conflict` | Concurrent writes | Merge changes, increment clock, retry |
| `plugin not found` | Not installed | `plugin_install("name")` or check `plugin_list` |
| `MCP connection failed` | Binary not executable | `chmod +x peabrain-rust`, check path |
| `Out of memory (WASM)` | Plugin too large | Increase wasmtime memory limit or use native plugin |

---

## 📁 Templates

### Fact Naming Convention
```
{domain}:{category}:{specific}
examples:
  - project:config:database_url
  - team:decision:auth_strategy
  - personal:preference:editor
  - debug:issue:memory_leak_fix
```

### Entity Types
```
service, database, api, library, model, person, team, document, issue, decision
```

### Relation Types
```
depends_on, reads, writes, owns, calls, implements, extends, uses, deploys, monitors
```

---

## 🔗 Related

- **peabrain-rust repo**: https://github.com/dbillion/peabrain-rust
- **MCP Spec**: https://modelcontextprotocol.io
- **sqlite-vec**: https://github.com/asg017/sqlite-vec
- **wasmtime**: https://github.com/bytecodealliance/wasmtime