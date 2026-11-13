# koala-mcp Quick Reference Card

## 🚀 Most Used Tools (Start Here)

### Store & Recall Facts
```bash
# Write
write_fact("project:config:port", "8080")

# Read
read_fact("project:config:port")

# Semantic search (auto-embeds query)
vector_search_facts(
  query_embedding=[...],  # from embed_text("port config")
  k=5
)
```

### Entities & Relations
```bash
# Create
create_entities([{"name": "AuthAPI", "type": "service", "observations": ["JWT", "Port 8080"]}])

# Relate
create_relations([{"from": "AuthAPI", "to": "UserDB", "type": "reads"}])

# Query
search_nodes("AuthAPI")
```

### Tasks
```bash
create_task_or_note("Review PR #42", "Check vector search logic", "pending")
query_tasks("pending")
update_task("task-123", "completed")
```

---

## 🔄 Multi-Agent Sync (CRDT)

```bash
# Agent A: write with clock
sync_write_fact("shared:decision", "Use HNSW", {"agent-a": 1})

# Agent B: get changes since last sync
changes = sync_get_changes("agent-a", {"agent-a": 0})
sync_merge_changes(changes, "agent-a")
```

---

## 🧠 Embeddings

```bash
# Single
emb = embed_text("How to configure HNSW")
# → emb.embedding[384]

# Batch (10x faster for multiple)
embs = embed_batch(["text1", "text2", "text3"])
```

---

## 🔌 Plugins

```bash
# Discover
plugin_discover()

# Install from marketplace
plugin_install("hash-tools")

# Use
plugin_call("hash-tools", "sha256", "data")

# List loaded
plugin_list()
```

---

## ⚡ Performance Cheatsheet

| Operation | Typical Latency | Batch? |
|-----------|----------------|--------|
| Fact write/read | < 1 ms | No |
| Embed (384-dim) | ~50 ms | Yes (embed_batch) |
| Vector upsert | < 5 ms | No |
| Vector search (k=10) | < 10 ms | No |
| Sync merge (1000 ops) | ~100 ms | No |
| Plugin call (WASM) | ~1 ms | No |

---

## 🐛 Common Errors

| Error | Fix |
|-------|-----|
| `sqlite-vec not loaded` | Install `vec0.so` or rebuild with `sqlite-vec` feature |
| `dimension mismatch` | Use 384-dim (all-MiniLM-L6-v2) |
| `vector_clock conflict` | Merge changes, increment clock, retry |
| `plugin not found` | `plugin_install("name")` first |
| `MCP connection failed` | Check binary path, `chmod +x`, `KOALA_STATE_DIR` |

---

## 📝 Naming Quick Ref

```
Facts:      domain:category:key          → project:config:port
Entities:   PascalCase                   → AuthService
Relations:  from → type → to             → AuthService → reads → UserDB
Tasks:      verb + object                → "Review PR #42"
Plugins:    kebab-case                   → hash-tools
```

---

## 🔗 Full Docs

- **All 50 tools**: See SKILL.md
- **Workflows**: SKILL.md → "Common Workflow Patterns"
- **Integration**: SKILL.md → "Integration Recipes"
- **Architecture**: README.md → "Architecture Diagram"