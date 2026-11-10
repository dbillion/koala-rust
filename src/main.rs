#![allow(clippy::all)]

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use peabrain_rust::{mcp::run_stdio, PeaBrain};

#[derive(Parser, Debug)]
#[command(
    name = "peabrain-rust",
    version,
    about = "PeaBrain MCP Server & CLI in Rust"
)]
struct Args {
    /// State directory (default: $HOME/.peabrain)
    #[arg(long, env = "PEABRAIN_STATE_DIR", global = true)]
    state_dir: Option<PathBuf>,

    /// Run as MCP server over stdio
    #[arg(long)]
    serve: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

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
    /// Add observations
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
    /// Search nodes
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
    /// Initialize model
    ModelInit {
        /// Hidden size
        #[arg(long, default_value = "8")]
        hidden: usize,
        /// Activation (leaky|sigmoid|tanh)
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
        /// Steps
        #[arg(long, default_value = "100")]
        steps: u32,
    },
    /// Predict
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
        /// Step
        #[arg(long)]
        step: i64,
    },
    /// Load model
    ModelLoad {
        /// Step
        #[arg(long)]
        step: i64,
    },

    // === Thought tools ===
    /// Sequential thinking
    Think {
        /// Step
        #[arg(long)]
        step: i64,
        /// Thought
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
    let args = Args::parse();

    let state_dir = args.state_dir.unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".peabrain")
    });

    std::fs::create_dir_all(&state_dir)?;

    let brain = PeaBrain::new(state_dir.clone()).await?;

    if args.serve {
        run_stdio(brain).await?;
    } else {
        match args.command {
            Some(cmd) => {
                handle_cli_command(brain, cmd).await?;
            }
            None => {
                println!("PeaBrain Rust v0.2.0");
                println!("State directory: {}", state_dir.display());
                println!("Use --serve for MCP server, or subcommands for CLI");
            }
        }
    }

    Ok(())
}

async fn handle_cli_command(mut brain: PeaBrain, cmd: Command) -> Result<()> {
    use sqlx::Row;

    match cmd {
        // Facts
        Command::FactWrite { name, text } => {
            let id = brain.db.write_fact(&name, &text).await?;
            println!("Fact written: {} (id: {})", name, id);
        }
        Command::FactRead { name } => {
            if let Some((id, text)) = brain.db.read_fact(&name).await? {
                println!("{} (id: {}): {}", name, id, text);
            } else {
                println!("Fact not found: {}", name);
            }
        }
        Command::FactList => {
            let facts = brain.db.list_facts().await?;
            for f in facts {
                println!("{}", f);
            }
        }
        Command::FactVersions { name, limit } => {
            let versions = brain.db.read_fact_versions(&name, limit).await?;
            for (id, text, created, current) in versions {
                println!("id:{} current:{} at:{}\n  {}", id, current, created, text);
            }
        }
        Command::FactDelete { name } => {
            let deleted = brain.db.delete_fact(&name).await?;
            println!("Deleted: {}", deleted);
        }
        Command::FactDeletePrefix { prefix } => {
            let count = brain.db.delete_fact_prefix(&prefix).await?;
            println!("Deleted {} facts", count);
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
            println!("Entity created: {} (id: {})", name, id);
        }
        Command::EntityAddObs { name, observations } => {
            let count = brain.db.add_observations(&name, &observations).await?;
            println!("Added {} observations to {}", count, name);
        }
        Command::EntityOpen { names } => {
            let nodes = brain.db.open_nodes(&names).await?;
            for node in nodes {
                println!(
                    "Entity: {} (type: {:?}, project: {:?})",
                    node.name, node.type_, node.project
                );
                for obs in node.observations {
                    println!("  - {}", obs);
                }
                for rel in node.relations {
                    println!("  {} -> {} ({})", rel.from, rel.to, rel.rel_type);
                }
            }
        }
        Command::EntitySearch { query, limit } => {
            let results = brain.db.search_nodes(&query, limit).await?;
            for r in results {
                println!("{}", r);
            }
        }
        Command::RelationCreate { from, to, type_ } => {
            let id = brain.db.create_relation(&from, &to, &type_).await?;
            println!(
                "Relation created: {} -> {} ({}) (id: {})",
                from, to, type_, id
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
            println!("Task created: {} (id: {})", title, id);
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
            for t in tasks {
                println!(
                    "{} [{}] {} - {} (priority: {})",
                    t.id, t.status, t.title, t.project, t.priority
                );
            }
        }
        Command::TaskUpdate {
            id,
            title,
            body,
            status,
            project,
            priority,
        } => {
            let updated = brain
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
            println!("Updated: {}", updated);
        }

        // Model
        Command::ModelInit {
            hidden,
            activation,
            lr,
        } => {
            brain.init_model(hidden, lr, &activation).await?;
            println!(
                "Model initialized: hidden={}, activation={}, lr={}",
                hidden, activation, lr
            );
        }
        Command::ModelTrain {
            xs,
            ys,
            targets,
            lr,
            steps,
        } => {
            let loss = brain.train_model(&xs, &ys, &targets, lr, steps).await?;
            println!("Trained: steps={}, loss={}", steps, loss);
        }
        Command::ModelPredict { xs, ys } => {
            let preds = brain.predict_model(&xs, &ys).await?;
            for (i, p) in preds.iter().enumerate() {
                println!("  [{}] {:.6}", i, p);
            }
        }
        Command::ModelStatus => {
            let status = brain.model_status().await?;
            println!("{:?}", status);
        }
        Command::ModelSave { step } => {
            brain.save_model(step).await?;
            println!("Model saved at step {}", step);
        }
        Command::ModelLoad { step } => {
            if let Some(_state) = brain.load_model(step).await? {
                println!("Model loaded at step {}", step);
            } else {
                println!("No model at step {}", step);
            }
        }

        // Thoughts
        Command::Think {
            step,
            thought,
            next_step,
        } => {
            let id = brain.db.add_thought(step, &thought).await?;
            println!(
                "Thought added (id: {}): step={}, next={}",
                id, step, next_step
            );
        }
        Command::ThoughtHistory { limit } => {
            let thoughts = brain.db.get_thoughts(limit).await?;
            for (id, step, thought, _created) in thoughts {
                println!("{} [{}] {}", id, step, thought);
            }
        }
        Command::ThoughtReset => {
            let count = brain.db.clear_thoughts().await?;
            println!("Cleared {} thoughts", count);
        }

        // Conclusion
        Command::Conclusion { summary } => {
            let id = brain.db.add_conclusion(&summary).await?;
            println!("Conclusion added (id: {}): {}", id, summary);
        }

        // Ranking
        Command::Feedback { name, signal, kind } => {
            brain.db.feedback(&name, &signal, &kind).await?;
            println!("Feedback recorded: {} - {} ({})", name, signal, kind);
        }
        Command::ExplainRank { name } => {
            if let Some(exp) = brain.db.explain_rank(&name).await? {
                println!("{}: {} = {:.2}", exp.name, exp.signal, exp.score);
            } else {
                println!("No rank data for {}", name);
            }
        }

        // System
        Command::Health => {
            let health = brain.db.health().await?;
            println!(
                "Tables: {}, Pages: {}, Journal: {}, Cache: {}",
                health.tables, health.pages, health.journal, health.cache_used
            );
        }

        // SQL
        Command::Sql { query } => {
            let db = &brain.db;
            let rows = sqlx::query(&query).fetch_all(&db.pool).await?;
            for row in rows {
                let mut cols = Vec::new();
                for i in 0..row.len() {
                    cols.push(
                        row.try_get::<String, _>(i)
                            .unwrap_or_else(|_| "NULL".to_string()),
                    );
                }
                println!("{}", cols.join(" | "));
            }
        }
    }
    Ok(())
}
