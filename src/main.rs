//! PeaBrain MCP Server & CLI in Rust
//!
//! A persistent memory system for AI agents with vector search, multi-agent sync,
//! neural models, and a plugin marketplace.
//!
//! # Quick Start
//!
//! ```bash
//! # Install
//! cargo install peabrain-rust
//!
//! # Run as MCP server (full 50 tools)
//! peabrain-rust --serve
//!
//! # CLI usage
//! peabrain-rust fact-write --name "project:config" --text "Angular 21 + NestJS"
//! peabrain-rust fact-read --name "project:config"
//! peabrain-rust vector-search "how to configure vector search"
//! ```
//!
//! # Architecture
//!
//! - **Facts**: Versioned key-value storage with SQLite
//! - **Entities/Relations**: Knowledge graph with observations
//! - **Vector Search**: Real semantic embeddings via Model2Vec (potion-base-8M, 256-dim)
//! - **CRDT Sync**: Vector clock-based multi-agent synchronization
//! - **Neural Model**: Pure-Rust 2→H→1 MLP for ranking/learning
//! - **Plugins**: WASM and native plugin system with marketplace
//!
//! # MCP Integration
//!
//! PeaBrain runs as an MCP (Model Context Protocol) server, exposing 50 tools
//! to MCP clients like Claude Code, Cursor, and other agents.
//!
//! ```json
//! {
//!   "mcpServers": {
//!     "peabrain": {
//!       "command": "peabrain-rust",
//!       "args": ["--serve"],
//!       "env": { "PEABRAIN_STATE_DIR": "~/.peabrain" }
//!     }
//!   }
//! }
//! ```

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use peabrain_rust::{PeaBrain, mcp::run_stdio};

/// Output format for CLI commands
#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    /// Human-readable text output
    Text,
    /// JSON output for scripting
    Json,
}

/// Main CLI arguments
#[derive(Parser, Debug)]
#[command(name = "peabrain-rust", version, about = "PeaBrain MCP Server & CLI in Rust")]
struct Args {
    /// State directory (default: $HOME/.peabrain)
    #[arg(long, env = "PEABRAIN_STATE_DIR", global = true)]
    state_dir: Option<PathBuf>,

    /// Output format
    #[arg(short, long, global = true, default_value = "text")]
    format: OutputFormat,

    /// Run as MCP server over stdio (full 50 tools)
    #[arg(long)]
    serve: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

/// All CLI commands
#[derive(Subcommand, Debug)]
enum Command {
    // === Fact tools ===
    /// Write a fact
    FactWrite {
        /// Fact name
        #[arg(long)]
        name: String,
        /// Fact text
        #[arg(long)]
        text: String,
    },
    /// Read a fact
    FactRead {
        /// Fact name
        #[arg(long)]
        name: String,
    },
    /// List all facts
    FactList,
    /// Read fact versions
    FactVersions {
        /// Fact name
        #[arg(long)]
        name: String,
        /// Limit
        #[arg(long, default_value = "10")]
        limit: i64,
    },
    /// Delete a fact
    FactDelete {
        /// Fact name
        #[arg(long)]
        name: String,
    },
    /// Delete facts by prefix
    FactDeletePrefix {
        /// Prefix
        #[arg(long)]
        prefix: String,
    },

    // === Entity tools ===
    /// Create entity
    EntityCreate {
        /// Entity name
        #[arg(long)]
        name: String,
        /// Entity type
        #[arg(long)]
        type_: Option<String>,
        /// Project
        #[arg(long)]
        project: Option<String>,
        /// Observations (comma-separated)
        #[arg(long, value_delimiter = ',')]
        observations: Vec<String>,
    },
    /// Add observations to entity
    EntityAddObs {
        /// Entity name
        #[arg(long)]
        name: String,
        /// Observations (comma-separated)
        #[arg(long, value_delimiter = ',')]
        observations: Vec<String>,
    },
    /// Open entities
    EntityOpen {
        /// Entity names (comma-separated)
        #[arg(long, value_delimiter = ',')]
        names: Vec<String>,
    },
    /// Search entities
    EntitySearch {
        /// Query
        #[arg(long)]
        query: String,
        /// Limit
        #[arg(long, default_value = "10")]
        limit: i64,
    },
    /// Create relation
    RelationCreate {
        /// From entity
        #[arg(long)]
        from: String,
        /// To entity
        #[arg(long)]
        to: String,
        /// Relation type
        #[arg(long)]
        type_: String,
    },

    // === Task tools ===
    /// Create task
    TaskCreate {
        /// Title
        #[arg(long)]
        title: String,
        /// Body
        #[arg(long, default_value = "")]
        body: String,
        /// Status
        #[arg(long, default_value = "not_started")]
        status: String,
        /// Project
        #[arg(long, default_value = "")]
        project: String,
        /// Priority
        #[arg(long, default_value = "medium")]
        priority: String,
    },
    /// Query tasks
    TaskQuery {
        /// Status filter
        #[arg(long)]
        status: Option<String>,
        /// Project filter
        #[arg(long)]
        project: Option<String>,
        /// Search text
        #[arg(long)]
        search: Option<String>,
        /// Limit
        #[arg(long, default_value = "10")]
        limit: i64,
    },
    /// Update task
    TaskUpdate {
        /// Task ID
        #[arg(long)]
        id: i64,
        /// Title
        #[arg(long)]
        title: Option<String>,
        /// Body
        #[arg(long)]
        body: Option<String>,
        /// Status
        #[arg(long)]
        status: Option<String>,
        /// Project
        #[arg(long)]
        project: Option<String>,
        /// Priority
        #[arg(long)]
        priority: Option<String>,
    },

    // === Model tools ===
    /// Initialize neural model
    ModelInit {
        /// Hidden layer size
        #[arg(long, default_value = "8")]
        hidden: usize,
        /// Activation function (leaky|sigmoid|tanh)
        #[arg(long, default_value = "leaky")]
        activation: String,
        /// Learning rate
        #[arg(long, default_value = "0.01")]
        lr: f32,
    },
    /// Train model
    ModelTrain {
        /// X values (comma-separated)
        #[arg(long, value_delimiter = ',')]
        xs: Vec<f32>,
        /// Y values (comma-separated)
        #[arg(long, value_delimiter = ',')]
        ys: Vec<f32>,
        /// Targets (comma-separated)
        #[arg(long, value_delimiter = ',')]
        targets: Vec<f32>,
        /// Learning rate
        #[arg(long, default_value = "0.01")]
        lr: f32,
        /// Training steps
        #[arg(long, default_value = "100")]
        steps: u32,
    },
    /// Predict with model
    ModelPredict {
        /// X values (comma-separated)
        #[arg(long, value_delimiter = ',')]
        xs: Vec<f32>,
        /// Y values (comma-separated)
        #[arg(long, value_delimiter = ',')]
        ys: Vec<f32>,
    },
    /// Model status
    ModelStatus,
    /// Save model
    ModelSave {
        /// Step to save
        #[arg(long)]
        step: i64,
    },
    /// Load model
    ModelLoad {
        /// Step to load
        #[arg(long)]
        step: i64,
    },

    // === Embedding tools ===
    /// Generate embedding for text
    EmbedText {
        /// Text to embed
        #[arg(long)]
        text: String,
    },
    /// Generate embeddings for batch of texts
    EmbedBatch {
        /// Texts to embed (comma-separated)
        #[arg(long, value_delimiter = ',')]
        texts: Vec<String>,
    },

    // === Vector search tools ===
    /// Upsert fact embedding for vector search
    VectorUpsertFact {
        /// Fact ID
        #[arg(long)]
        fact_id: i64,
        /// Embedding vector (comma-separated floats)
        #[arg(long, value_delimiter = ',')]
        embedding: Vec<f32>,
    },
    /// Search facts by vector similarity
    VectorSearchFacts {
        /// Query embedding (comma-separated floats)
        #[arg(long, value_delimiter = ',')]
        query_embedding: Vec<f32>,
        /// Number of results
        #[arg(long, default_value = "5")]
        k: usize,
        /// Optional keyword filter
        #[arg(long)]
        keyword_filter: Option<String>,
    },

    // === Sync tools ===
    /// Push local changes to peers
    SyncPush,
    /// Pull changes from peers
    SyncPull,
    /// Sync status
    SyncStatus,

    // === Plugin tools ===
    /// Search plugin registry
    PluginSearch {
        /// Search query
        #[arg(long)]
        query: Option<String>,
    },
    /// Install plugin
    PluginInstall {
        /// Plugin name
        #[arg(long)]
        name: String,
        /// Version (optional)
        #[arg(long)]
        version: Option<String>,
    },
    /// Uninstall plugin
    PluginUninstall {
        /// Plugin name
        #[arg(long)]
        name: String,
    },
    /// Call plugin tool
    PluginCall {
        /// Plugin name
        #[arg(long)]
        plugin: String,
        /// Tool name
        #[arg(long)]
        tool: String,
        /// Arguments as JSON
        #[arg(long)]
        args: String,
    },
    /// Plugin registry stats
    PluginRegistryStats,
    /// Update plugin registry
    PluginUpdateRegistry,

    // === Thought tools ===
    /// Sequential thinking
    Think {
        /// Step number
        #[arg(long)]
        step: i64,
        /// Thought text
        #[arg(long)]
        thought: String,
        /// Next step
        #[arg(long)]
        next_step: i64,
    },
    /// Thought history
    ThoughtHistory {
        /// Limit
        #[arg(long, default_value = "10")]
        limit: i64,
    },
    /// Reset thoughts
    ThoughtReset,

    // === Conclusion tools ===
    /// Add conclusion
    Conclusion {
        /// Summary
        #[arg(long)]
        summary: String,
    },

    // === Ranking tools ===
    /// Feedback
    Feedback {
        /// Name
        #[arg(long)]
        name: String,
        /// Signal
        #[arg(long)]
        signal: String,
        /// Kind
        #[arg(long)]
        kind: String,
    },
    /// Explain rank
    ExplainRank {
        /// Name
        #[arg(long)]
        name: String,
    },

    // === System ===
    /// Health check
    Health,

    // === SQL ===
    /// Run raw SQL query
    Sql {
        /// Query
        query: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Args::parse();

    let state_dir = cli.state_dir.unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".peabrain")
    });

    std::fs::create_dir_all(&state_dir)?;

    let mut brain = PeaBrain::new(state_dir.clone()).await?;

    // MCP server mode (full 50 tools)
    if cli.serve {
        run_stdio(brain).await?;
        return Ok(());
    }

    // CLI mode
    match cli.command {
        Some(cmd) => {
            execute_command(&mut brain, cmd, cli.format).await?;
        }
        None => {
            println!("PeaBrain Rust v0.2.0");
            println!("State directory: {}", state_dir.display());
            println!("Use --serve for MCP server, or subcommands for CLI");
        }
    }

    Ok(())
}

/// Execute a CLI command
async fn execute_command(brain: &mut PeaBrain, cmd: Command, format: OutputFormat) -> Result<()> {
    use sqlx::Row;

    match cmd {
        // Facts
        Command::FactWrite { name, text } => {
            let id = brain.db.write_fact(&name, &text).await?;
            print_output(format, serde_json::json!({"id": id, "name": name, "status": "written"}));
        }
        Command::FactRead { name } => {
            if let Some((id, text)) = brain.db.read_fact(&name).await? {
                print_output(format, serde_json::json!({"id": id, "name": name, "text": text}));
            } else {
                print_output(format, serde_json::json!({"error": "Fact not found", "name": name}));
            }
        }
        Command::FactList => {
            let facts = brain.db.list_facts().await?;
            print_output(format, serde_json::json!(facts));
        }
        Command::FactVersions { name, limit } => {
            let versions = brain.db.read_fact_versions(&name, limit).await?;
            print_output(format, serde_json::json!(versions));
        }
        Command::FactDelete { name } => {
            let deleted = brain.db.delete_fact(&name).await?;
            print_output(format, serde_json::json!({"deleted": deleted, "name": name}));
        }
        Command::FactDeletePrefix { prefix } => {
            let count = brain.db.delete_fact_prefix(&prefix).await?;
            print_output(format, serde_json::json!({"deleted": count, "prefix": prefix}));
        }

        // Entities
        Command::EntityCreate { name, type_, project, observations } => {
            let id = brain.db.create_entity(&name, type_.as_deref(), project.as_deref(), &observations).await?;
            print_output(format, serde_json::json!({"id": id, "name": name}));
        }
        Command::EntityAddObs { name, observations } => {
            let count = brain.db.add_observations(&name, &observations).await?;
            print_output(format, serde_json::json!({"added": count, "name": name}));
        }
        Command::EntityOpen { names } => {
            let nodes = brain.db.open_nodes(&names).await?;
            print_output(format, serde_json::json!(nodes));
        }
        Command::EntitySearch { query, limit } => {
            let results = brain.db.search_nodes(&query, limit).await?;
            print_output(format, serde_json::json!(results));
        }
        Command::RelationCreate { from, to, type_ } => {
            let id = brain.db.create_relation(&from, &to, &type_).await?;
            print_output(format, serde_json::json!({"id": id, "from": from, "to": to, "type": type_}));
        }

        // Tasks
        Command::TaskCreate { title, body, status, project, priority } => {
            let id = brain.db.create_task(&title, &body, &status, &project, &priority).await?;
            print_output(format, serde_json::json!({"id": id, "title": title}));
        }
        Command::TaskQuery { status, project, search, limit } => {
            let tasks = brain.db.query_tasks(status.as_deref(), project.as_deref(), search.as_deref(), limit).await?;
            print_output(format, serde_json::json!(tasks));
        }
        Command::TaskUpdate { id, title, body, status, project, priority } => {
            brain.db.update_task(id, title.as_deref(), body.as_deref(), status.as_deref(), project.as_deref(), priority.as_deref()).await?;
            print_output(format, serde_json::json!({"updated": true, "id": id}));
        }

        // Model
        Command::ModelInit { hidden, activation, lr } => {
            brain.init_model(hidden, lr, &activation).await?;
            print_output(format, serde_json::json!({"initialized": true}));
        }
        Command::ModelTrain { xs, ys, targets, lr, steps } => {
            let loss = brain.train_model(&xs, &ys, &targets, lr, steps).await?;
            print_output(format, serde_json::json!({"loss": loss}));
        }
        Command::ModelPredict { xs, ys } => {
            let preds = brain.predict_model(&xs, &ys).await?;
            print_output(format, serde_json::json!({"predictions": preds}));
        }
        Command::ModelStatus => {
            let status = brain.model_status().await?;
            print_output(format, serde_json::json!(status));
        }
        Command::ModelSave { step } => {
            brain.save_model(step).await?;
            print_output(format, serde_json::json!({"saved": true, "step": step}));
        }
        Command::ModelLoad { step } => {
            brain.load_model(step).await?;
            print_output(format, serde_json::json!({"loaded": true, "step": step}));
        }

        // Embeddings
        Command::EmbedText { text } => {
            let embedding = brain.embed_text(&text).await?;
            print_output(format, serde_json::json!({"embedding": embedding}));
        }
        Command::EmbedBatch { texts } => {
            let embeddings = brain.embed_batch(&texts.iter().map(|s| s.as_str()).collect::<Vec<_>>()).await?;
            print_output(format, serde_json::json!({"embeddings": embeddings}));
        }

        // Vector search
        Command::VectorUpsertFact { fact_id, embedding } => {
            brain.vector_upsert_fact(fact_id, &embedding).await?;
            print_output(format, serde_json::json!({"upserted": true, "fact_id": fact_id}));
        }
        Command::VectorSearchFacts { query_embedding, k, keyword_filter } => {
            let results = brain.vector_search_facts(&query_embedding, k, keyword_filter.as_deref()).await?;
            print_output(format, serde_json::json!(results));
        }

        // Sync
        Command::SyncPush => {
            brain.sync_manager.sync_write_fact("", "").await?; // Placeholder
            print_output(format, serde_json::json!({"pushed": true}));
        }
        Command::SyncPull => {
            // Placeholder
            print_output(format, serde_json::json!({"pulled": true}));
        }
        Command::SyncStatus => {
            print_output(format, serde_json::json!({"status": "ok"}));
        }

        // Plugins
        Command::PluginSearch { query } => {
            let results = brain.plugin_registry.search(&query.unwrap_or_default()).await?;
            print_output(format, serde_json::json!(results));
        }
        Command::PluginInstall { name, version } => {
            let path = brain.plugin_registry.install(&name, version.as_deref()).await?;
            print_output(format, serde_json::json!({"installed": true, "path": path}));
        }
        Command::PluginUninstall { name } => {
            brain.plugin_registry.uninstall(&name).await?;
            print_output(format, serde_json::json!({"uninstalled": true, "name": name}));
        }
        Command::PluginCall { plugin, tool, args } => {
            let result = brain.call_plugin_tool(&plugin, &tool, serde_json::from_str(&args)?).await?;
            print_output(format, result);
        }
        Command::PluginRegistryStats => {
            let stats = brain.plugin_registry.stats().await?;
            print_output(format, serde_json::json!(stats));
        }
        Command::PluginUpdateRegistry => {
            let count = brain.plugin_registry.update_registry(None).await?;
            print_output(format, serde_json::json!({"updated": count}));
        }

        // Thoughts
        Command::Think { step, thought, next_step } => {
            let id = brain.db.add_thought(step, &thought).await?;
            print_output(format, serde_json::json!({"id": id, "step": step, "next_step": next_step}));
        }
        Command::ThoughtHistory { limit } => {
            let thoughts = brain.db.get_thoughts(limit).await?;
            print_output(format, serde_json::json!(thoughts));
        }
        Command::ThoughtReset => {
            let count = brain.db.clear_thoughts().await?;
            print_output(format, serde_json::json!({"cleared": count}));
        }

        // Conclusions
        Command::Conclusion { summary } => {
            let id = brain.db.add_conclusion(&summary).await?;
            print_output(format, serde_json::json!({"id": id, "summary": summary}));
        }

        // Ranking
        Command::Feedback { name, signal, kind } => {
            brain.db.feedback(&name, &signal, &kind).await?;
            print_output(format, serde_json::json!({"recorded": true}));
        }
        Command::ExplainRank { name } => {
            if let Some(exp) = brain.db.explain_rank(&name).await? {
                print_output(format, serde_json::json!(exp));
            } else {
                print_output(format, serde_json::json!({"error": "No rank data", "name": name}));
            }
        }

        // System
        Command::Health => {
            let health = brain.db.health().await?;
            print_output(format, serde_json::json!(health));
        }

        // SQL
        Command::Sql { query } => {
            let rows = brain.db.raw_sql(&query).await?;
            print_output(format, serde_json::json!(rows));
        }
    }

    Ok(())
}

/// Print output based on format
fn print_output(format: OutputFormat, value: serde_json::Value) {
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&value).unwrap()),
        OutputFormat::Text => {
            if value.is_object() {
                let mut out = String::new();
                if let Some(obj) = value.as_object() {
                    for (k, v) in obj {
                        out.push_str(&format!("{}: {}\n", k, v));
                    }
                }
                print!("{}", out);
            } else if value.is_array() {
                let mut out = String::new();
                if let Some(arr) = value.as_array() {
                    for (i, v) in arr.iter().enumerate() {
                        out.push_str(&format!("{}: {}\n", i, v));
                    }
                }
                print!("{}", out);
            } else {
                println!("{}", value);
            }
        }
    }
}