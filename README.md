# peabrain-rust

**Local, private, persistent memory for AI agents** — with semantic search, CRDT sync, real embeddings, and WASM plugin extensibility.

> A single binary (49 MB) that gives any MCP-compatible agent: long-term fact storage, vector search, knowledge graphs, task management, multi-agent synchronization, and a sandboxed plugin system.

---

### 🙏 Attribution

This project is a **Rust rewrite and evolution** of the original [PeaBrain](https://github.com/akadatalimited/PeaBrain) by [AkaData Limited](https://www.linkedin.com/in/akadatalimited/) — extending it with:
- Real local embeddings (Candle + all-MiniLM-L6-v2)
- Vector search (sqlite-vec HNSW)
- CRDT-based multi-agent sync (VectorClock)
- WASM/native plugin marketplace (wasmtime + libloading)
- MCP server with 50 tools
- Cross-platform releases (Linux/macOS/Windows × x86_64/ARM64)

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| **Persistent Facts** | Versioned key-value storage with SQLite |
| **Semantic Search** | HNSW vector search via `sqlite-vec` (384-dim embeddings) |
| **Real Embeddings** | Local Candle + `all-MiniLM-L6-v2` (~90 MB, downloads once) |
| **Knowledge Graph** | Entities, observations, relations with graph traversal |
| **Task Management** | Structured tasks/notes with queries and updates |
| **CRDT Sync** | Vector clocks + last-write-wins for multi-agent collaboration |
| **WASM Plugins** | Sandboxed plugins (wasmtime) + native plugins (libloading) |
| **Plugin Marketplace** | Install/verify plugins with SHA256 signatures |
| **MCP Server** | 50 tools via stdio — works with Claude, Hermes, any MCP client |

---

## 🏛️ Architecture Overview

```mermaid
flowchart TB
    subgraph Agent["AI Agent (Claude, Hermes, Custom)"]
        direction TB
        MCP_CLIENT[MCP Client]
    end

    subgraph Server["peabrain-rust MCP Server (stdio)"]
        direction TB
        MCP_HANDLER[MCP Handler 50 tools]
        
        subgraph Core["Core Services"]
            FACTS[Fact Store<br/>SQLite + versions]
            ENTITIES[Entity Graph<br/>Observations + Relations]
            TASKS[Task Manager<br/>CRUD + Queries]
            NEURAL[Neural Model<br/>Online Learning]
        end
        
        subgraph Vector["Vector Engine"]
            SQLITE_VEC[(sqlite-vec HNSW)]
            EMBEDDINGS[Candle + all-MiniLM-L6-v2]
        end
        
        subgraph Sync["CRDT Sync"]
            VECTOR_CLOCK[VectorClock<br/>Last-Write-Wins]
            PEER_SYNC[Peer Sync Protocol]
        end
        
        subgraph Plugins["Plugin System"]
            WASM_RUNTIME[wasmtime<br/>WASM Sandbox]
            NATIVE_LOADER[libloading<br/>Native Plugins]
            MARKETPLACE[Plugin Registry<br/>SHA256 Verify]
        end
    end

    subgraph Storage["Local Storage (~/.peabrain)"]
        PEABRAIN_DB[(peabrain.db)]
        MODEL_DIR[model/]
        PLUGIN_DIR[plugins/<br/>wasm/ + native/]
        REGISTRY[registry.json]
    end

    MCP_CLIENT <-->|JSON-RPC 2.0<br/>stdio| MCP_HANDLER
    MCP_HANDLER --> FACTS
    MCP_HANDLER --> ENTITIES
    MCP_HANDLER --> TASKS
    MCP_HANDLER --> NEURAL
    MCP_HANDLER --> Vector
    MCP_HANDLER --> Sync
    MCP_HANDLER --> Plugins
    
    FACTS --> PEABRAIN_DB
    ENTITIES --> PEABRAIN_DB
    TASKS --> PEABRAIN_DB
    NEURAL --> MODEL_DIR
    Vector --> SQLITE_VEC
    Vector --> EMBEDDINGS
    Sync --> VECTOR_CLOCK
    Sync --> PEER_SYNC
    Plugins --> WASM_RUNTIME
    Plugins --> NATIVE_LOADER
    Plugins --> MARKETPLACE
    Plugins --> PLUGIN_DIR
    MARKETPLACE --> REGISTRY
    
    SQLITE_VEC --> PEABRAIN_DB
    EMBEDDINGS -->|~90 MB| MODEL_DIR

    style Agent fill:#e3f2fd
    style Server fill:#fff3e0
    style Storage fill:#e8f5e9
    style Vector fill:#fce4ec
    style Sync fill:#f3e5f5
    style Plugins fill:#fff8e1
```

---

## 🚀 Quick Start

### Install (Binary)

```bash
# Linux/macOS/Windows — download from releases
curl -L https://github.com/dbillion/peabrain-rust/releases/latest/download/peabrain-rust-$(uname -s)-$(uname -m) -o peabrain-rust
chmod +x peabrain-rust

# Or via cargo
cargo install --git https://github.com/dbillion/peabrain-rust
```

### Run MCP Server

```bash
# Set state directory (where SQLite + embeddings live)
export PEABRAIN_STATE_DIR=~/.peabrain

# Start MCP server (stdio)
./peabrain-rust --serve
```

### Use with Any MCP Client

**Claude Desktop** (`claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "peabrain": {
      "command": "/full/path/to/peabrain-rust",
      "args": ["--serve"],
      "env": { "PEABRAIN_STATE_DIR": "/home/you/.peabrain" }
    }
  }
}
```

**Hermes** (`~/.hermes/config.yaml`):
```yaml
mcp_servers:
  peabrain-rust:
    command: "/full/path/to/peabrain-rust"
    args: ["--serve"]
    env:
      PEABRAIN_STATE_DIR: "/home/you/.peabrain"
    enabled: true
```

**Custom Client** — connect via stdio JSON-RPC 2.0.

---

## 🎯 10 Key Business Use Cases

| # | Use Case | Value |
|---|----------|-------|
| **1** | **Institutional Knowledge Retention** | Never lose technical decisions, debugging context, or tribal knowledge when team members leave. Semantic search finds "why we chose X" instantly. |
| **2** | **Multi-Agent Collaboration** | Research, coding, and review agents share a synchronized fact base via CRDT — no conflicting states, no manual handoffs. |
| **3** | **Compliance & Audit Trail** | Versioned facts + timestamps = immutable record of decisions. Export for SOC2, GDPR, or internal audits. |
| **4** | **Cross-Session Continuity** | Agents resume exactly where they left off — context, tasks, learned patterns persist across restarts, days, machines. |
| **5** | **Private AI Memory (Air-Gapped)** | 100% local, no API keys, no cloud. Runs in secure environments, regulated industries, offline deployments. |
| **6** | **Plugin-Driven Workflow Automation** | Install WASM plugins for custom transformers (hash, JSON, text, domain-specific). Extend without recompiling. |
| **7** | **Semantic Search over Conversations** | "What did we decide about auth last month?" → vector search returns relevant turns, not keyword matches. |
| **8** | **Knowledge Graph for Domain Modeling** | Entities + observations + relations = queryable domain model. "Show all services depending on PostgreSQL." |
| **9** | **Task Handoff Between Agents** | Structured tasks with state, priority, assignee. Agent A creates, Agent B picks up, Agent C reviews — all tracked. |
| **10** | **Embedding-Powered RAG Without Cloud** | Local embeddings + vector search = full RAG pipeline on-device. Zero marginal cost per query. |

---

## 🛠️ CLI Usage

```bash
# Health check
peabrain-rust health

# Facts
peabrain-rust fact-write --name "project:config" --text "Uses port 8080"
peabrain-rust fact-read --name "project:config"
peabrain-rust fact-list
peabrain-rust fact-versions --name "project:config" --limit 5

# Entities & Knowledge Graph
peabrain-rust entity-create --name "User" --type "person" --observations "Prefers TypeScript"
peabrain-rust entity-search --query "User"
peabrain-rust relation-create --from "User" --to "Project" --type "owns"

# Tasks
peabrain-rust task-create --step "Review PR #42" --thought "Check vector search logic"
peabrain-rust task-query --status pending

# Embeddings & Vector Search
peabrain-rust embed-text --text "How to configure vector search"
peabrain-rust vector-upsert-fact --fact-id 1 --embedding-file embedding.json
peabrain-rust vector-search-facts --query-embedding-file query.json --k 5

# Sync (Multi-agent)
peabrain-rust sync-write-fact --name "shared:decision" --text "Use HNSW"
peabrain-rust sync-get-changes --peer agent-2
peabrain-rust sync-merge-changes --changes-file changes.json

# Plugins
peabrain-rust plugin-list
peabrain-rust plugin-search --query "hash"
peabrain-rust plugin-install --name hash-tools
peabrain-rust plugin-call --name hash-tools --tool sha256 --input "hello"
```

---

## 🔌 MCP Tools (50 Total)

| Category | Tools |
|----------|-------|
| **Health** | `health` |
| **Facts** | `write_fact`, `read_fact`, `list_facts`, `read_fact_versions`, `delete_fact`, `delete_fact_prefix` |
| **Entities** | `create_entities`, `add_observations`, `open_nodes`, `search_nodes`, `create_relations` |
| **Tasks** | `create_task_or_note`, `query_tasks`, `update_task` |
| **Neural Model** | `model_init`, `model_train`, `model_predict`, `model_status`, `feedback`, `explain_rank`, `sequential_thinking`, `conclusion`, `thought_history`, `thought_reset`, `model_save`, `model_load` |
| **Vector Search** | `vector_upsert_fact`, `vector_upsert_entity`, `vector_search_facts`, `vector_search_entities`, `vector_delete_fact`, `vector_delete_entity` |
| **Sync (CRDT)** | `sync_write_fact`, `sync_read_fact`, `sync_list_facts`, `sync_create_entity`, `sync_get_changes`, `sync_merge_changes` |
| **Embeddings** | `embed_text`, `embed_batch` |
| **Plugins** | `plugin_load`, `plugin_call`, `plugin_list`, `plugin_discover` |
| **Marketplace** | `plugin_search`, `plugin_install`, `plugin_uninstall`, `plugin_registry_stats`, `plugin_update_registry` |

---

## 📚 peabrain-mcp Skill (Usage Guide)

A dedicated skill teaches agents how to use peabrain-rust effectively via MCP. **Included in this repo at `skills/peabrain-mcp/`.**

### Install Skill

```bash
# Hermes (from local repo)
hermes skill install skills/peabrain-mcp

# Or clone skill dir directly
cp -r skills/peabrain-mcp ~/.hermes/skills/peabrain-mcp
```

### Skill Provides

| Capability | Description |
|------------|-------------|
| **Tool Reference** | All 50 MCP tools with parameters, examples, error handling |
| **Workflow Patterns** | Common patterns: fact extraction → embed → upsert → search |
| **Best Practices** | When to use facts vs entities vs tasks, naming conventions |
| **Integration Recipes** | Claude Desktop, Hermes, custom MCP clients |
| **Troubleshooting** | Common errors, performance tuning, debugging tips |

### Example Skill Usage (Agent)

```
# Agent reads skill, then:
peabrain_recall("authentication architecture decision")
# → Returns relevant facts with scores

peabrain_write_fact("api:rate_limit", "100 req/min per IP")
# → Stores with auto-embedding for future recall
```

---

## 🧠 Auto-Conversation Memory (Hermes)

The **peabrain-memory** Hermes plugin captures every conversation turn, extracts facts, generates embeddings, and enables semantic recall.

### Install Plugin

```bash
# Copy to Hermes user plugins dir
cp -r peabrain-memory ~/.hermes/plugins/peabrain-memory
```

### Enable in Config

```yaml
# ~/.hermes/config.yaml
plugins:
  enabled:
    - peabrain-memory
    # ... other plugins

memory:
  provider: peabrain   # <-- Critical: only ONE external provider allowed
```

### Restart Gateway

```bash
systemctl --user restart hermes-gateway
```

### Agent Tools Available

After restart, agents get 3 tools:

| Tool | Description |
|------|-------------|
| `peabrain_recall` | Search conversation memory by semantic similarity |
| `peabrain_write_fact` | Manually write a fact to persistent memory |
| `peabrain_search` | Alias for recall |

**How it works:**
1. `on_turn_start` → captures user messages
2. `on_session_end` → extracts facts, embeds via `embed_text`, stores via `write_fact`, upserts vectors via `vector_upsert_fact`
3. Background thread → non-blocking async processing
4. `peabrain_recall("vector search config")` → returns relevant past conversations

---

## 🏗️ Build from Source

### Prerequisites

- Rust 1.78+ (MSRV)
- `sqlite3` dev headers
- `pkg-config`

### Build

```bash
git clone https://github.com/dbillion/peabrain-rust
cd peabrain-rust

# Type-check only (fast)
cargo check

# Lint
cargo clippy

# Release build (~8-10 min first time)
cargo build --release

# Binary at target/release/peabrain-rust (49 MB)
```

### Cross-Platform Builds (Local)

```bash
# Install cross
cargo install cross

# Linux x86_64
cross build --release --target x86_64-unknown-linux-gnu

# Linux ARM64
cross build --release --target aarch64-unknown-linux-gnu

# macOS x86_64
cross build --release --target x86_64-apple-darwin

# macOS ARM64 (Apple Silicon)
cross build --release --target aarch64-apple-darwin

# Windows x86_64
cargo build --release --target x86_64-pc-windows-msvc
```

### GitHub Actions (Auto on Tag)

The repo includes **two workflows** in `.github/workflows/`:

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `ci.yml` | Push/PR to master | Format, clippy, type-check, test, cross-platform check (5 targets) |
| `release.yml` | Tag push (`v*`) | **Build 6 platform binaries + create GitHub Release** |

**Release on tag push:**
```bash
git tag v0.3.0
git push origin v0.3.0
# → CI passes → release.yml builds all 6 targets → creates GitHub Release with binaries
```

**Artifacts produced per release:**
| Platform | Artifact Name |
|----------|---------------|
| Linux x86_64 | `peabrain-rust-x86_64-unknown-linux-gnu` |
| Linux ARM64 | `peabrain-rust-aarch64-unknown-linux-gnu` |
| macOS x86_64 (Intel) | `peabrain-rust-x86_64-apple-darwin` |
| macOS ARM64 (Apple Silicon) | `peabrain-rust-aarch64-apple-darwin` |
| Windows x86_64 | `peabrain-rust-x86_64-pc-windows-msvc.exe` |

Each release includes `SHA256SUMS.txt` for verification.

---

## 📦 Plugin Development

### WASM Plugin (Recommended)

Create a WAT file with required exports:

```wat
(module
  (memory 1 10)  ;; 10 pages = 640 KB
  (export "memory" (memory 0))

  ;; Allocator for host→WASM data transfer
  (func (export "allocate") (param $size i32) (result i32)
    ;; Return pointer to fresh memory
  )
  (func (export "deallocate") (param $ptr i32) (param $size i32)
    ;; Free memory
  )

  ;; Manifest & tools (called by host after allocate+write)
  (func (export "set_manifest") (param $ptr i32) (param $len i32))
  (func (export "set_tools") (param $ptr i32) (param $len i32))
  (func (export "get_manifest") (result i32))
  (func (export "get_tools") (result i32))

  ;; Tool execution
  (func (export "call_tool") (param $name_ptr i32) (param $name_len i32)
                    (param $input_ptr i32) (param $input_len i32)
                    (result i32))
)
```

**Host calls:** `allocate()` → writes JSON → `set_manifest(ptr, len)` / `set_tools(ptr, len)` → `get_manifest()` / `get_tools()` → `call_tool()`.

See `plugins/wasm/*.wat` for examples: `hash-tools`, `text-transform`, `json-tools`.

### Native Plugin

Shared library with C ABI:

```c
// plugin.h
typedef struct { const char* name; const char* version; } PluginManifest;
typedef struct { const char* name; const char* description; const char* input_schema; } ToolDef;

PluginManifest* get_manifest();
ToolDef* get_tools(int* count);
char* call_tool(const char* tool_name, const char* input_json);
void free_string(char* s);
```

Compile: `rustc --crate-type=cdylib plugin.rs -o libplugin.so`

Install: `peabrain-rust plugin-install --path ./libplugin.so --type native`

---

## 🔒 Privacy & Security

- **100% local** — no network calls except initial model download (~90 MB from HuggingFace)
- **WASM sandbox** — memory-limited, no host access
- **Native plugins** — only from trusted sources (you verify)
- **SQLite** — single file, easy to backup/encrypt
- **No telemetry** — zero data leaves your machine

---

## 📊 Performance

| Operation | Latency (typical) |
|-----------|-------------------|
| Fact write/read | < 1 ms |
| Embedding (384-dim) | ~50 ms (CPU) |
| Vector upsert | < 5 ms |
| Vector search (k=10) | < 10 ms |
| Sync merge (1000 ops) | ~100 ms |
| Plugin call (WASM) | ~1 ms overhead |

---

## 🗺️ Roadmap

| Version | Focus |
|---------|-------|
| **v0.3** | Conversation auto-logging hook, better fact extraction (LLM-based) |
| **v0.4** | Remote plugin registry, plugin signing/verification |
| **v0.5** | Distributed sync (WebRTC/WebSocket), conflict resolution UI |
| **v1.0** | Production hardening, benchmarks, migration tools |

---

## 🤝 Contributing

```bash
# 1. Fork & clone
# 2. Create branch
# 3. Make changes + tests
# 4. cargo check && cargo clippy && cargo test
# 5. PR with conventional commit: "feat: add X" / "fix: resolve Y"
```

---

## 📄 License

MIT — see [LICENSE](LICENSE)

---

## 🔗 Related

- **Hermes Plugin**: `peabrain-memory` (in `~/.hermes/plugins/peabrain-memory/`)
- **peabrain-mcp Skill**: `skills/peabrain-mcp/` in this repo
- **MCP Spec**: https://modelcontextprotocol.io
- **sqlite-vec**: https://github.com/asg017/sqlite-vec
- **Candle**: https://github.com/huggingface/candle
- **wasmtime**: https://github.com/bytecodealliance/wasmtime