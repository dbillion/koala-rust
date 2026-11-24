#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! Koala MCP Server & CLI in Rust
//!
//! A persistent memory system for AI agents with vector search, multi-agent sync,
//! neural models, and a plugin marketplace.
//!
//! # Quick Start
//!
//! ```bash
//! # Install
//! cargo install koala-rust
//!
//! # Run as MCP server (full 53 tools)
//! koala-rust --serve
//!
//! # CLI usage
//! koala-rust fact-write --name "project:config" --text "Angular 21 + NestJS"
//! koala-rust fact-read --name "project:config"
//! koala-rust vector-search "how to configure vector search"
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
//! Koala runs as an MCP (Model Context Protocol) server, exposing 53 tools
//! to MCP clients like Claude Code, Cursor, and other agents.
//!
//! ```json
//! {
//!   "mcpServers": {
//!     "koala": {
//!       "command": "koala-rust",
//!       "args": ["--serve"],
//!       "env": { "KOALA_STATE_DIR": "~/.koala" }
//!     }
//!   }
//! }
//! ```

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::PathBuf;

use koala_rust::{mcp::run_stdio, Koala};

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
#[command(name = "koala-rust", version, about = "Koala MCP Server & CLI in Rust")]
struct Args {
    /// State directory (default: $HOME/.koala)
    #[arg(long, env = "KOALA_STATE_DIR", global = true)]
    state_dir: Option<PathBuf>,

    /// Output format
    #[arg(short, long, global = true, default_value = "text")]
    format: OutputFormat,

    /// Run as MCP server over stdio (full 53 tools)
    #[arg(long)]
    serve: bool,

    /// Disable all runtime network access; requires a local model directory
    #[arg(long, global = true)]
    offline: bool,

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
        /// Read the latest version that existed at this RFC3339/SQLite timestamp
        #[arg(long)]
        as_of: Option<String>,
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

    // === Semantic memory tools ===
    /// Remember a fact and create its semantic embedding
    Remember {
        /// Fact name
        #[arg(long)]
        name: String,
        /// Fact text
        #[arg(long)]
        text: String,
        /// Optional time-to-live in seconds (overrides per-label defaults)
        #[arg(long)]
        ttl_seconds: Option<i64>,
    },
    /// Purge expired facts and their vectors immediately
    SweepExpired,
    /// Recall facts by semantic query without supplying raw vectors
    Recall {
        /// Natural-language search query
        #[arg(long)]
        query: String,
        /// Number of results
        #[arg(long, default_value = "5")]
        k: usize,
        /// Keyword overlap weight
        #[arg(long, default_value = "0.35")]
        keyword_weight: f32,
        /// Restrict recall to the newest fact versions known at this timestamp
        #[arg(long)]
        as_of: Option<String>,
    },

    // === Vector search tools ===
    /// Upsert fact embedding for vector search
    VectorUpsertFact {
        /// Fact ID
        #[arg(long)]
        fact_id: i64,
        /// Embedding vector (comma-separated floats)
        #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
        embedding: Vec<f32>,
    },
    /// Search facts by vector similarity
    VectorSearchFacts {
        /// Query embedding (comma-separated floats)
        #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
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

    // === Database maintenance ===
    /// Create a consistent SQLite backup
    DbBackup {
        /// Destination path (must not exist)
        #[arg(long)]
        path: PathBuf,
    },
    /// Restore the database from a verified SQLite backup
    DbRestore {
        /// Backup file path
        #[arg(long)]
        path: PathBuf,
    },

    /// Verify or export the tamper-evident audit chain
    Audit {
        #[command(subcommand)]
        action: AuditAction,
    },

    // === SQL ===
    /// Run raw SQL query
    Sql {
        /// Query
        query: String,
    },

    // === Model provisioning ===
    /// Download a Model2Vec model for later offline use
    Model {
        #[command(subcommand)]
        action: ModelAction,
    },

    // === Context & Continuity ===
    /// AI Continuity and context lifecycle commands
    Context {
        #[command(subcommand)]
        action: ContextAction,
    },
}

/// Audit log operations
#[derive(Subcommand, Debug)]
enum AuditAction {
    /// Verify the hash chain and signed head
    Verify,
    /// Export all audit records as JSON
    Export,
}

/// Model provisioning actions
#[derive(Subcommand, Debug)]
enum ModelAction {
    /// Fetch model files into a directory for offline runtime use
    Fetch {
        /// Destination directory for the local Model2Vec files
        #[arg(long)]
        to: PathBuf,
        /// Hugging Face model repository
        #[arg(long, default_value = "minishlab/potion-base-8M")]
        model: String,
    },
}

/// Context lifecycle actions
#[derive(Subcommand, Debug)]
enum ContextAction {
    /// Retrieve full bootstrap context (user profile + project architecture, decisions, handoff)
    Bootstrap {
        /// Project name
        #[arg(long, short)]
        project: String,
    },
    /// Save a handoff checkpoint when work pauses or concludes
    Checkpoint {
        /// Project name
        #[arg(long, short)]
        project: String,
        /// Handoff summary of what was accomplished
        #[arg(long, short)]
        summary: String,
        /// Next steps for the incoming agent/session
        #[arg(long, short)]
        next: String,
        /// Comma-separated list of blockers (optional)
        #[arg(long, short)]
        blockers: Option<String>,
    },
    /// Record an architectural or technical decision
    Decision {
        /// Project name
        #[arg(long, short)]
        project: String,
        /// The decision that was made
        #[arg(long, short)]
        decision: String,
        /// Rationale behind the decision (optional)
        #[arg(long, short)]
        rationale: Option<String>,
    },
    /// Update the current project status and blockers
    Status {
        /// Project name
        #[arg(long, short)]
        project: String,
        /// Current status summary
        #[arg(long, short)]
        status: String,
        /// Comma-separated list of blockers (optional)
        #[arg(long, short)]
        blockers: Option<String>,
        /// Next immediate step (optional)
        #[arg(long, short)]
        next: Option<String>,
    },
    /// Get current user working profile and preferences
    GetUser,
    /// Get project context
    GetProject {
        /// Project name
        #[arg(long, short)]
        project: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Args::parse();
    if cli.offline {
        std::env::set_var("KOALA_OFFLINE", "1");
        std::env::set_var("HF_HUB_OFFLINE", "1");
        std::env::set_var("TRANSFORMERS_OFFLINE", "1");
    }

    // Model provisioning does not need to open the database or load sqlite-vec.
    if let Some(Command::Model {
        action: ModelAction::Fetch { to, model },
    }) = cli.command.as_ref()
    {
        koala_rust::embeddings::fetch_model_to(model, to)?;
        print_output(
            cli.format,
            serde_json::json!({
                "model": model,
                "directory": to.display().to_string(),
                "offline_env": "KOALA_MODEL_DIR",
            }),
        );
        return Ok(());
    }

    let state_dir = cli.state_dir.unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".koala")
    });

    std::fs::create_dir_all(&state_dir)?;

    // Restore must run before Koala opens the destination database.
    if let Some(Command::DbRestore { path }) = cli.command.as_ref() {
        restore_database(&state_dir, path).await?;
        print_output(
            cli.format,
            serde_json::json!({"restored": true, "from": path}),
        );
        return Ok(());
    }

    let mut brain = Koala::new(state_dir.clone()).await?;

    // MCP server mode (full 53 tools)
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
            println!("Koala Rust v{}", env!("CARGO_PKG_VERSION"));
            println!("State directory: {}", state_dir.display());
            println!("Use --serve for MCP server, or subcommands for CLI");
        }
    }

    Ok(())
}

/// Validate a backup before atomically replacing the active database file.
async fn restore_database(state_dir: &std::path::Path, backup: &std::path::Path) -> Result<()> {
    if !backup.is_file() {
        anyhow::bail!("backup file does not exist: {}", backup.display());
    }
    let mut options = SqliteConnectOptions::new().filename(backup).read_only(true);
    if cfg!(feature = "sqlcipher") {
        let key = std::env::var("KOALA_DB_KEY").map_err(|_| {
            anyhow::anyhow!("KOALA_DB_KEY must be set when using the sqlcipher feature")
        })?;
        if key.is_empty() {
            anyhow::bail!("KOALA_DB_KEY must not be empty");
        }
        options = options.pragma("key", format!("'{}'", key.replace('\'', "''")));
    }
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&pool)
        .await?;
    if integrity != "ok" {
        pool.close().await;
        anyhow::bail!("backup failed SQLite integrity_check: {}", integrity);
    }
    pool.close().await;

    let destination = state_dir.join("koala.db");
    let temporary = state_dir.join("koala.db.restore.tmp");
    if temporary.exists() {
        std::fs::remove_file(&temporary)?;
    }
    std::fs::copy(backup, &temporary)?;
    std::fs::File::open(&temporary)?.sync_all()?;
    if destination.exists() {
        let stamp = chrono::Utc::now().timestamp_millis();
        let previous = state_dir.join(format!("koala.db.pre-restore-{}.bak", stamp));
        std::fs::rename(&destination, previous)?;
    }
    std::fs::rename(&temporary, &destination)?;
    Ok(())
}

/// Execute a CLI command
async fn execute_command(brain: &mut Koala, cmd: Command, format: OutputFormat) -> Result<()> {
    match cmd {
        // Facts
        Command::FactWrite { name, text } => {
            let id = brain.db.write_fact(&name, &text).await?;
            print_output(
                format,
                serde_json::json!({"id": id, "name": name, "status": "written"}),
            );
        }
        Command::FactRead { name, as_of } => {
            let result = if let Some(timestamp) = as_of {
                brain.db.read_fact_as_of(&name, &timestamp).await?
                    .map(|(id, text, created_at)| serde_json::json!({"id": id, "name": name, "text": text, "created_at": created_at, "as_of": timestamp}))
            } else {
                brain
                    .db
                    .read_fact(&name)
                    .await?
                    .map(|(id, text)| serde_json::json!({"id": id, "name": name, "text": text}))
            };
            print_output(
                format,
                result.unwrap_or_else(
                    || serde_json::json!({"error": "Fact not found", "name": name}),
                ),
            );
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
            let deleted = brain.delete_fact_and_vector(&name).await?;
            print_output(
                format,
                serde_json::json!({"deleted": deleted, "name": name}),
            );
        }
        Command::FactDeletePrefix { prefix } => {
            let count = brain.delete_fact_prefix_and_vectors(&prefix).await?;
            print_output(
                format,
                serde_json::json!({"deleted": count, "prefix": prefix}),
            );
        }
        Command::Remember {
            name,
            text,
            ttl_seconds,
        } => {
            let rowid = if let Some(ttl) = ttl_seconds {
                brain
                    .remember_semantic_with_ttl(&name, &text, Some(ttl))
                    .await?
            } else {
                brain.remember_semantic(&name, &text).await?
            };
            print_output(
                format,
                serde_json::json!({"rowid": rowid, "name": name, "status": "remembered", "ttl_seconds": ttl_seconds}),
            );
        }
        Command::SweepExpired => {
            let purged = brain.sweep_expired_facts().await?;
            print_output(format, serde_json::json!({"purged_rows": purged}));
        }
        Command::Recall {
            query,
            k,
            keyword_weight,
            as_of,
        } => {
            let results = if let Some(timestamp) = as_of {
                brain
                    .recall_as_of(&query, k, keyword_weight, &timestamp)
                    .await?
            } else {
                brain.recall(&query, k, keyword_weight).await?
            };
            print_output(
                format,
                serde_json::json!({"query": query, "results": results}),
            );
        }

        // Entities
        Command::EntityCreate {
            name,
            type_,
            project,
            observations,
        } => {
            let id = brain
                .db
                .create_entity(&name, type_.as_deref(), project.as_deref(), &observations)
                .await?;
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
            print_output(
                format,
                serde_json::json!({"id": id, "from": from, "to": to, "type": type_}),
            );
        }

        // Tasks
        Command::TaskCreate {
            title,
            body,
            status,
            project,
            priority,
        } => {
            let id = brain
                .db
                .create_task(&title, &body, &status, &project, &priority)
                .await?;
            print_output(format, serde_json::json!({"id": id, "title": title}));
        }
        Command::TaskQuery {
            status,
            project,
            search,
            limit,
        } => {
            let tasks = brain
                .db
                .query_tasks(
                    status.as_deref(),
                    project.as_deref(),
                    search.as_deref(),
                    limit,
                )
                .await?;
            print_output(format, serde_json::json!(tasks));
        }
        Command::TaskUpdate {
            id,
            title,
            body,
            status,
            project,
            priority,
        } => {
            brain
                .db
                .update_task(
                    id,
                    title.as_deref(),
                    body.as_deref(),
                    status.as_deref(),
                    project.as_deref(),
                    priority.as_deref(),
                )
                .await?;
            print_output(format, serde_json::json!({"updated": true, "id": id}));
        }

        // Model
        Command::ModelInit {
            hidden,
            activation,
            lr,
        } => {
            brain.init_model(hidden, lr, &activation).await?;
            print_output(format, serde_json::json!({"initialized": true}));
        }
        Command::ModelTrain {
            xs,
            ys,
            targets,
            lr,
            steps,
        } => {
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
            let embeddings = brain
                .embed_batch(&texts.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                .await?;
            print_output(format, serde_json::json!({"embeddings": embeddings}));
        }

        // Vector search
        Command::VectorUpsertFact { fact_id, embedding } => {
            brain.vector_upsert_fact(fact_id, &embedding).await?;
            print_output(
                format,
                serde_json::json!({"upserted": true, "fact_id": fact_id}),
            );
        }
        Command::VectorSearchFacts {
            query_embedding,
            k,
            keyword_filter,
        } => {
            let results = brain
                .vector_search_facts(&query_embedding, k, keyword_filter.as_deref())
                .await?;
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
            let results = brain
                .plugin_registry
                .search(&query.unwrap_or_default())
                .await?;
            print_output(format, serde_json::json!(results));
        }
        Command::PluginInstall { name, version } => {
            let path = brain
                .plugin_registry
                .install(&name, version.as_deref())
                .await?;
            print_output(format, serde_json::json!({"installed": true, "path": path}));
        }
        Command::PluginUninstall { name } => {
            brain.plugin_registry.uninstall(&name).await?;
            print_output(
                format,
                serde_json::json!({"uninstalled": true, "name": name}),
            );
        }
        Command::PluginCall { plugin, tool, args } => {
            let result = brain
                .call_plugin_tool(&plugin, &tool, serde_json::from_str(&args)?)
                .await?;
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
        Command::Think {
            step,
            thought,
            next_step,
        } => {
            let id = brain.db.add_thought(step, &thought).await?;
            print_output(
                format,
                serde_json::json!({"id": id, "step": step, "next_step": next_step}),
            );
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
                print_output(
                    format,
                    serde_json::json!({"error": "No rank data", "name": name}),
                );
            }
        }

        // System
        Command::Health => {
            let health = brain.db.health().await?;
            print_output(format, serde_json::json!(health));
        }

        // Database maintenance
        Command::DbBackup { path } => {
            brain.db.backup_to(&path).await?;
            print_output(format, serde_json::json!({"backup": path, "created": true}));
        }
        Command::DbRestore { .. } => unreachable!("restore is handled before opening the database"),

        // Audit
        Command::Audit {
            action: AuditAction::Verify,
        } => {
            let result = brain.db.audit_verify().await?;
            print_output(format, serde_json::to_value(result)?);
        }
        Command::Audit {
            action: AuditAction::Export,
        } => {
            let verification = brain.db.audit_verify().await?;
            let records = brain.db.audit_export().await?;
            print_output(
                format,
                serde_json::json!({"verification": verification, "records": records}),
            );
        }

        // Model provisioning
        Command::Model {
            action: ModelAction::Fetch { to, model },
        } => {
            koala_rust::embeddings::fetch_model_to(&model, &to)?;
            print_output(
                format,
                serde_json::json!({
                    "model": model,
                    "directory": to.display().to_string(),
                    "offline_env": "KOALA_MODEL_DIR",
                }),
            );
        }

        // SQL
        Command::Sql { query } => {
            let rows = brain.db.raw_sql(&query).await?;
            print_output(format, serde_json::json!(rows));
        }

        // Context & Continuity
        Command::Context { action } => match action {
            ContextAction::Bootstrap { project } => {
                let ctx = brain.bootstrap_context(&project).await?;
                print_output(format, serde_json::to_value(&ctx)?);
            }
            ContextAction::Checkpoint {
                project,
                summary,
                next,
                blockers,
            } => {
                let blocker_list: Vec<String> = blockers
                    .map(|b| {
                        b.split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    })
                    .unwrap_or_default();
                let cp = brain
                    .create_handoff(&project, &summary, &next, &blocker_list)
                    .await?;
                print_output(format, serde_json::to_value(&cp)?);
            }
            ContextAction::Decision {
                project,
                decision,
                rationale,
            } => {
                let res = brain
                    .record_decision(&project, &decision, rationale.as_deref())
                    .await?;
                print_output(format, res);
            }
            ContextAction::Status {
                project,
                status,
                blockers,
                next,
            } => {
                let blocker_list: Vec<String> = blockers
                    .map(|b| {
                        b.split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    })
                    .unwrap_or_default();
                let res = brain
                    .update_project_status(&project, &status, &blocker_list, next.as_deref())
                    .await?;
                print_output(format, res);
            }
            ContextAction::GetUser => {
                let user_ctx = brain.get_user_context().await?;
                print_output(format, serde_json::to_value(&user_ctx)?);
            }
            ContextAction::GetProject { project } => {
                let proj_ctx = brain.get_project_context(&project).await?;
                print_output(format, serde_json::to_value(&proj_ctx)?);
            }
        },
    }

    Ok(())
}

/// Print output based on format
fn print_output(format: OutputFormat, value: serde_json::Value) {
    match format {
        OutputFormat::Json => match serde_json::to_string_pretty(&value) {
            Ok(json) => println!("{json}"),
            Err(error) => eprintln!("failed to serialize JSON output: {error}"),
        },
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

#[cfg(test)]
mod cli_tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn vector_search_accepts_negative_embedding_components() {
        let args = Args::try_parse_from([
            "koala-rust",
            "vector-search-facts",
            "--query-embedding",
            "-0.5,0.25",
        ])
        .expect("negative vector values are positional values, not flags");
        match args.command {
            Some(Command::VectorSearchFacts {
                query_embedding, ..
            }) => {
                assert_eq!(query_embedding, vec![-0.5, 0.25]);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[cfg(not(feature = "sqlcipher"))]
    #[tokio::test]
    async fn restore_replaces_database_only_after_integrity_check() {
        let root = std::env::temp_dir().join(format!("koala-restore-{}", uuid::Uuid::new_v4()));
        let state_dir = root.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let backup_path = root.join("backup.db");

        let backup = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&backup_path)
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        sqlx::query("CREATE TABLE facts (name TEXT, text TEXT)")
            .execute(&backup)
            .await
            .unwrap();
        sqlx::query("INSERT INTO facts VALUES ('restored','new data')")
            .execute(&backup)
            .await
            .unwrap();
        backup.close().await;

        let current_path = state_dir.join("koala.db");
        let current = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&current_path)
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        sqlx::query("CREATE TABLE old_data (value TEXT)")
            .execute(&current)
            .await
            .unwrap();
        current.close().await;

        restore_database(&state_dir, &backup_path).await.unwrap();
        let restored = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&current_path)
                    .read_only(true),
            )
            .await
            .unwrap();
        let text: String = sqlx::query_scalar("SELECT text FROM facts WHERE name='restored'")
            .fetch_one(&restored)
            .await
            .unwrap();
        assert_eq!(text, "new data");
        assert_eq!(std::fs::read_dir(&state_dir).unwrap().count(), 2);
        restored.close().await;
        let _ = std::fs::remove_dir_all(root);
    }
}
