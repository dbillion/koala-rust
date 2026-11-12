use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqliteConnectOptions, Column, Row, SqlitePool};
use std::path::Path;
use std::str::FromStr;

// ===== MODULE-LEVEL STRUCTS (not inside impl blocks) =====

#[derive(Debug, serde::Serialize)]
pub struct EntityNode {
    pub name: String,
    pub type_: Option<String>,
    pub project: Option<String>,
    pub observations: Vec<String>,
    pub relations: Vec<Relation>,
}

#[derive(Debug, serde::Serialize)]
pub struct Relation {
    pub from: String,
    pub to: String,
    pub rel_type: String,
}

#[derive(Debug, serde::Serialize)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub status: String,
    pub project: String,
    pub priority: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, serde::Serialize)]
pub struct RankExplanation {
    pub name: String,
    pub signal: String,
    pub score: f64,
}

#[derive(Debug, serde::Serialize)]
pub struct HealthInfo {
    pub tables: i64,
    pub pages: i64,
    pub journal: bool,
    pub cache_used: i64,
    pub usage_tracked: i64,
    pub pending_flush: bool,
}

// ===== DATABASE STRUCT =====

#[derive(Clone)]
pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    pub async fn new(path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Get absolute path manually
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };

        // Create pool with sqlite-vec extension loaded on every connection
        let db_url = format!("sqlite://{}?mode=rwc", abs_path.display());
        let connect_opts =
            SqliteConnectOptions::from_str(&db_url)?.extension("/usr/local/lib/vec0.so");
        let pool = SqlitePool::connect_with(connect_opts).await?;

        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }

    async fn migrate(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS facts (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                text TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                is_current BOOLEAN DEFAULT 1
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS facts_tombstone (
                name TEXT PRIMARY KEY,
                deleted_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS entities (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL,
                type TEXT,
                project TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS observations (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                entity_name TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (entity_name) REFERENCES entities(name) ON DELETE CASCADE
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS relations (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                from_entity TEXT NOT NULL,
                to_entity TEXT NOT NULL,
                rel_type TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (from_entity) REFERENCES entities(name) ON DELETE CASCADE,
                FOREIGN KEY (to_entity) REFERENCES entities(name) ON DELETE CASCADE
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                body TEXT DEFAULT '',
                status TEXT DEFAULT 'not_started',
                project TEXT DEFAULT '',
                priority TEXT DEFAULT 'medium',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS model_weights (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                step INTEGER NOT NULL,
                weights_json TEXT NOT NULL,
                biases_json TEXT NOT NULL,
                optimizer_json TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS thoughts (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                step INTEGER NOT NULL,
                thought TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS conclusions (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                summary TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS rank_signals (
                name TEXT PRIMARY KEY,
                signal_type TEXT NOT NULL,
                value REAL NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        // v0.2: Vector embedding tables (managed by sqlite-vec extension)
        // These are virtual tables created by vector engine

        // v0.3: Sync metadata tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sync_peers (
                peer_id TEXT PRIMARY KEY,
                last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
                doc_type TEXT NOT NULL,
                heads_json TEXT NOT NULL
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sync_changes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                peer_id TEXT NOT NULL,
                doc_type TEXT NOT NULL,
                change_bytes BLOB NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        // Small key/value store: embedding model identity and the reindex-pending marker.
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS brain_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        // v0.4: Embedding metadata
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS embedding_metadata (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_type TEXT NOT NULL, -- 'fact' | 'entity'
                source_id INTEGER NOT NULL,
                model_name TEXT NOT NULL,
                dimension INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        // v0.5: Plugin metadata
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS plugin_registry (
                name TEXT PRIMARY KEY,
                version TEXT NOT NULL,
                manifest_json TEXT NOT NULL,
                plugin_type TEXT NOT NULL, -- 'wasm' | 'native'
                path TEXT NOT NULL,
                enabled BOOLEAN DEFAULT 1,
                loaded_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(r#"
            CREATE INDEX IF NOT EXISTS idx_facts_name ON facts(name);
            CREATE INDEX IF NOT EXISTS idx_facts_current ON facts(is_current);
            CREATE INDEX IF NOT EXISTS idx_observations_entity ON observations(entity_name);
            CREATE INDEX IF NOT EXISTS idx_relations_from ON relations(from_entity);
            CREATE INDEX IF NOT EXISTS idx_relations_to ON relations(to_entity);
            CREATE INDEX IF NOT EXISTS idx_sync_changes_peer ON sync_changes(peer_id, doc_type);
            CREATE INDEX IF NOT EXISTS idx_embedding_metadata_source ON embedding_metadata(source_type, source_id);
        "#).execute(&self.pool).await?;

        Ok(())
    }

    // ===== FACTS =====
    pub async fn write_fact(&self, name: &str, text: &str) -> Result<i64> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("UPDATE facts SET is_current = 0 WHERE name = ? AND is_current = 1")
            .bind(name)
            .execute(&mut *tx)
            .await?;

        let row = sqlx::query(
            "INSERT INTO facts (name, text, is_current) VALUES (?, ?, 1) RETURNING rowid",
        )
        .bind(name)
        .bind(text)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(row.get("rowid"))
    }

    pub async fn read_fact(&self, name: &str) -> Result<Option<(i64, String)>> {
        let row = sqlx::query("SELECT rowid, text FROM facts WHERE name = ? AND is_current = 1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| (r.get("rowid"), r.get("text"))))
    }

    /// Fetch a fact by rowid. Returns (name, text, is_current).
    pub async fn read_fact_by_rowid(&self, rowid: i64) -> Result<Option<(String, String, bool)>> {
        let row = sqlx::query("SELECT name, text, is_current FROM facts WHERE rowid = ?")
            .bind(rowid)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| (r.get("name"), r.get("text"), r.get::<bool, _>("is_current"))))
    }

    /// All current facts as (rowid, name, text).
    pub async fn current_facts(&self) -> Result<Vec<(i64, String, String)>> {
        let rows = sqlx::query("SELECT rowid, name, text FROM facts WHERE is_current = 1")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.get("rowid"), r.get("name"), r.get("text")))
            .collect())
    }

    pub async fn meta_get(&self, key: &str) -> Result<Option<String>> {
        let row = sqlx::query("SELECT value FROM brain_meta WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.get("value")))
    }

    pub async fn meta_set(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO brain_meta(key, value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Rowids of current facts whose name or text contains any of the tokens.
    /// Independent of the vector index, so exact names outside the vector
    /// candidate pool are still found.
    pub async fn keyword_fact_candidates(&self, tokens: &[String], limit: i64) -> Result<Vec<i64>> {
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        let clause = tokens
            .iter()
            .map(|_| "(instr(lower(replace(name, '_', ' ')), ?) > 0 OR instr(lower(text), ?) > 0)")
            .collect::<Vec<_>>()
            .join(" OR ");
        let sql = format!("SELECT rowid FROM facts WHERE is_current = 1 AND ({clause}) LIMIT ?");
        let mut q = sqlx::query(&sql);
        for t in tokens {
            q = q.bind(t.clone()).bind(t.clone());
        }
        let rows = q.bind(limit).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| r.get("rowid")).collect())
    }

    /// Entities as (rowid, text to embed): name, type and observations.
    pub async fn entity_texts(&self) -> Result<Vec<(i64, String)>> {
        let rows = sqlx::query(
            "SELECT e.rowid AS rowid, e.name AS name, e.type AS type, \
             group_concat(o.content, '; ') AS obs \
             FROM entities e LEFT JOIN observations o ON o.entity_name = e.name \
             GROUP BY e.rowid",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let name: String = r.get("name");
                let ty: Option<String> = r.get("type");
                let obs: Option<String> = r.get("obs");
                let text = format!(
                    "{} ({}): {}",
                    name,
                    ty.unwrap_or_default(),
                    obs.unwrap_or_default()
                );
                (r.get("rowid"), text)
            })
            .collect())
    }

    /// Rowids of the current version(s) of a fact name.
    pub async fn current_fact_rowids(&self, name: &str) -> Result<Vec<i64>> {
        let rows = sqlx::query("SELECT rowid FROM facts WHERE name = ? AND is_current = 1")
            .bind(name)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.get("rowid")).collect())
    }

    pub async fn read_fact_versions(
        &self,
        name: &str,
        limit: i64,
    ) -> Result<Vec<(i64, String, DateTime<Utc>, bool)>> {
        let rows = sqlx::query("SELECT rowid, text, created_at, is_current FROM facts WHERE name = ? ORDER BY created_at DESC LIMIT ?")
            .bind(name)
            .bind(limit)
            .fetch_all(&self.pool).await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.get("rowid"),
                    r.get("text"),
                    r.get("created_at"),
                    r.get("is_current"),
                )
            })
            .collect())
    }

    pub async fn list_facts(&self) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT name FROM facts WHERE is_current = 1 ORDER BY name")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.get("name")).collect())
    }

    pub async fn delete_fact(&self, name: &str) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let deleted = sqlx::query("DELETE FROM facts WHERE name = ? AND is_current = 1")
            .bind(name)
            .execute(&mut *tx)
            .await?
            .rows_affected();

        if deleted > 0 {
            sqlx::query("INSERT OR IGNORE INTO facts_tombstone (name) VALUES (?)")
                .bind(name)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(deleted > 0)
    }

    pub async fn delete_fact_prefix(&self, prefix: &str) -> Result<i64> {
        let mut tx = self.pool.begin().await?;
        let names: Vec<String> =
            sqlx::query("SELECT name FROM facts WHERE name LIKE ? AND is_current = 1")
                .bind(format!("{}%", prefix))
                .fetch_all(&mut *tx)
                .await?
                .into_iter()
                .map(|r| r.get("name"))
                .collect();

        let count = names.len() as i64;
        for name in names {
            sqlx::query("UPDATE facts SET is_current = 0 WHERE name = ? AND is_current = 1")
                .bind(&name)
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT OR IGNORE INTO facts_tombstone (name) VALUES (?)")
                .bind(&name)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(count)
    }

    // ===== ENTITIES =====
    pub async fn create_entity(
        &self,
        name: &str,
        type_: Option<&str>,
        project: Option<&str>,
        observations: &[String],
    ) -> Result<i64> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "INSERT INTO entities (name, type, project) VALUES (?, ?, ?) RETURNING rowid",
        )
        .bind(name)
        .bind(type_)
        .bind(project)
        .fetch_one(&mut *tx)
        .await?;

        let rowid: i64 = row.get("rowid");
        for obs in observations {
            sqlx::query("INSERT INTO observations (entity_name, content) VALUES (?, ?)")
                .bind(name)
                .bind(obs)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(rowid)
    }

    pub async fn add_observations(&self, name: &str, observations: &[String]) -> Result<i64> {
        let mut tx = self.pool.begin().await?;
        let mut added = 0;
        for obs in observations {
            let result =
                sqlx::query("INSERT INTO observations (entity_name, content) VALUES (?, ?)")
                    .bind(name)
                    .bind(obs)
                    .execute(&mut *tx)
                    .await?;
            added += result.rows_affected() as i64;
        }
        tx.commit().await?;
        Ok(added)
    }

    pub async fn open_nodes(&self, names: &[String]) -> Result<Vec<EntityNode>> {
        let mut results = Vec::new();
        for name in names {
            if let Some(entity) =
                sqlx::query("SELECT name, type, project FROM entities WHERE name = ?")
                    .bind(name)
                    .fetch_optional(&self.pool)
                    .await?
            {
                let observations: Vec<String> =
                    sqlx::query("SELECT content FROM observations WHERE entity_name = ?")
                        .bind(name)
                        .fetch_all(&self.pool)
                        .await?
                        .into_iter()
                        .map(|r| r.get("content"))
                        .collect();

                let relations: Vec<Relation> = sqlx::query("SELECT from_entity, to_entity, rel_type FROM relations WHERE from_entity = ? OR to_entity = ?")
                    .bind(name)
                    .bind(name)
                    .fetch_all(&self.pool).await?
                    .into_iter()
                    .map(|r| Relation {
                        from: r.get("from_entity"),
                        to: r.get("to_entity"),
                        rel_type: r.get("rel_type"),
                    })
                    .collect();

                results.push(EntityNode {
                    name: entity.get("name"),
                    type_: entity.get("type"),
                    project: entity.get("project"),
                    observations,
                    relations,
                });
            }
        }
        Ok(results)
    }

    pub async fn create_relation(&self, from: &str, to: &str, rel_type: &str) -> Result<i64> {
        let row = sqlx::query("INSERT INTO relations (from_entity, to_entity, rel_type) VALUES (?, ?, ?) RETURNING rowid")
            .bind(from)
            .bind(to)
            .bind(rel_type)
            .fetch_one(&self.pool).await?;
        Ok(row.get("rowid"))
    }

    pub async fn search_nodes(&self, query: &str, limit: i64) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT DISTINCT name FROM (
                SELECT name FROM entities WHERE name LIKE ?
                UNION
                SELECT entity_name as name FROM observations WHERE content LIKE ?
                UNION
                SELECT name FROM facts WHERE name LIKE ? AND is_current = 1
            ) LIMIT ?",
        )
        .bind(format!("%{}%", query))
        .bind(format!("%{}%", query))
        .bind(format!("%{}%", query))
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.get("name")).collect())
    }

    // ===== TASKS =====
    pub async fn create_task(
        &self,
        title: &str,
        body: &str,
        status: &str,
        project: &str,
        priority: &str,
    ) -> Result<i64> {
        let row = sqlx::query(
            "INSERT INTO tasks (title, body, status, project, priority) VALUES (?, ?, ?, ?, ?) RETURNING rowid"
        )
        .bind(title)
        .bind(body)
        .bind(status)
        .bind(project)
        .bind(priority)
        .fetch_one(&self.pool).await?;
        Ok(row.get("rowid"))
    }

    pub async fn query_tasks(
        &self,
        status: Option<&str>,
        project: Option<&str>,
        search: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Task>> {
        let mut query = String::from("SELECT rowid, title, body, status, project, priority, created_at, updated_at FROM tasks WHERE 1=1");
        if status.is_some() {
            query.push_str(" AND status = ?");
        }
        if project.is_some() {
            query.push_str(" AND project = ?");
        }
        if search.is_some() {
            query.push_str(" AND (title LIKE ? OR body LIKE ?)");
        }
        query.push_str(" ORDER BY created_at DESC LIMIT ?");

        let mut q = sqlx::query(&query);
        if let Some(s) = status {
            q = q.bind(s);
        }
        if let Some(p) = project {
            q = q.bind(p);
        }
        if let Some(s) = search {
            q = q.bind(format!("%{}%", s));
            q = q.bind(format!("%{}%", s));
        }
        q = q.bind(limit);

        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|r| Task {
                id: r.get("rowid"),
                title: r.get("title"),
                body: r.get("body"),
                status: r.get("status"),
                project: r.get("project"),
                priority: r.get("priority"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    pub async fn update_task(
        &self,
        id: i64,
        title: Option<&str>,
        body: Option<&str>,
        status: Option<&str>,
        project: Option<&str>,
        priority: Option<&str>,
    ) -> Result<bool> {
        let mut updates = Vec::new();

        if let Some(_t) = title {
            updates.push("title = ?");
        }
        if let Some(_b) = body {
            updates.push("body = ?");
        }
        if let Some(_s) = status {
            updates.push("status = ?");
        }
        if let Some(_p) = project {
            updates.push("project = ?");
        }
        if let Some(_p) = priority {
            updates.push("priority = ?");
        }

        if updates.is_empty() {
            return Ok(false);
        }

        updates.push("updated_at = CURRENT_TIMESTAMP");
        let query = format!("UPDATE tasks SET {} WHERE rowid = ?", updates.join(", "));

        let mut q = sqlx::query(&query);
        if let Some(t) = title {
            q = q.bind(t);
        }
        if let Some(b) = body {
            q = q.bind(b);
        }
        if let Some(s) = status {
            q = q.bind(s);
        }
        if let Some(p) = project {
            q = q.bind(p);
        }
        if let Some(p) = priority {
            q = q.bind(p);
        }
        q = q.bind(id);

        let result = q.execute(&self.pool).await?;
        Ok(result.rows_affected() > 0)
    }

    // ===== MODEL PERSISTENCE =====
    pub async fn save_model(
        &self,
        step: i64,
        weights: &[f32],
        biases: &[f32],
        optimizer: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO model_weights (step, weights_json, biases_json, optimizer_json) VALUES (?, ?, ?, ?)"
        )
        .bind(step)
        .bind(serde_json::to_string(weights)?)
        .bind(serde_json::to_string(biases)?)
        .bind(optimizer)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn load_model(&self, step: i64) -> Result<Option<(Vec<f32>, Vec<f32>)>> {
        let row = sqlx::query("SELECT weights_json, biases_json FROM model_weights WHERE step = ? ORDER BY created_at DESC LIMIT 1")
            .bind(step)
            .fetch_optional(&self.pool).await?;

        Ok(row.map(|r| {
            (
                serde_json::from_str(r.get("weights_json")).unwrap(),
                serde_json::from_str(r.get("biases_json")).unwrap(),
            )
        }))
    }

    // Full model state save/load
    pub async fn save_model_full(&self, step: i64, state_json: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO model_weights (step, weights_json, biases_json, optimizer_json) VALUES (?, ?, ?, ?)"
        )
        .bind(step)
        .bind(state_json)
        .bind(state_json)
        .bind(None::<String>)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn load_model_full(&self, step: i64) -> Result<Option<String>> {
        let row = sqlx::query("SELECT weights_json FROM model_weights WHERE step = ? ORDER BY created_at DESC LIMIT 1")
            .bind(step)
            .fetch_optional(&self.pool).await?;

        Ok(row.map(|r| r.get("weights_json")))
    }

    // ===== RANKING =====
    pub async fn feedback(&self, name: &str, signal: &str, kind: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO rank_signals (name, signal_type, value) VALUES (?, ?, 1.0) 
             ON CONFLICT(name) DO UPDATE SET signal_type = ?, value = rank_signals.value + 1.0, updated_at = CURRENT_TIMESTAMP"
        )
        .bind(name)
        .bind(format!("{}:{}", kind, signal))
        .bind(format!("{}:{}", kind, signal))
        .execute(&self.pool).await?;
        Ok(())
    }

    // ===== THOUGHTS =====
    pub async fn add_thought(&self, step: i64, thought: &str) -> Result<i64> {
        let row = sqlx::query("INSERT INTO thoughts (step, thought) VALUES (?, ?) RETURNING rowid")
            .bind(step)
            .bind(thought)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("rowid"))
    }

    pub async fn get_thoughts(&self, limit: i64) -> Result<Vec<(i64, i64, String, DateTime<Utc>)>> {
        let rows = sqlx::query(
            "SELECT rowid, step, thought, created_at FROM thoughts ORDER BY step ASC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.get("rowid"),
                    r.get("step"),
                    r.get("thought"),
                    r.get("created_at"),
                )
            })
            .collect())
    }

    pub async fn clear_thoughts(&self) -> Result<i64> {
        let result = sqlx::query("DELETE FROM thoughts")
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() as i64)
    }

    // ===== CONCLUSIONS =====
    pub async fn add_conclusion(&self, summary: &str) -> Result<i64> {
        let row = sqlx::query("INSERT INTO conclusions (summary) VALUES (?) RETURNING rowid")
            .bind(summary)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("rowid"))
    }

    pub async fn get_conclusions(&self) -> Result<Vec<(i64, String, DateTime<Utc>)>> {
        let rows = sqlx::query(
            "SELECT rowid, summary, created_at FROM conclusions ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.get("rowid"), r.get("summary"), r.get("created_at")))
            .collect())
    }

    pub async fn explain_rank(&self, name: &str) -> Result<Option<RankExplanation>> {
        let row = sqlx::query("SELECT name, signal_type, value FROM rank_signals WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| RankExplanation {
            name: r.get("name"),
            signal: r.get("signal_type"),
            score: r.get("value"),
        }))
    }

    // ===== HEALTH =====
    pub async fn health(&self) -> Result<HealthInfo> {
        let tables: i64 = sqlx::query("SELECT COUNT(*) as c FROM sqlite_master WHERE type='table'")
            .fetch_one(&self.pool)
            .await?
            .get("c");

        let pages: i64 = sqlx::query("PRAGMA page_count")
            .fetch_one(&self.pool)
            .await?
            .get(0);
        let journal_mode: String = sqlx::query("PRAGMA journal_mode")
            .fetch_one(&self.pool)
            .await?
            .get(0);
        let journal = journal_mode != "off";

        Ok(HealthInfo {
            tables,
            pages,
            journal,
            cache_used: 0,
            usage_tracked: 0,
            pending_flush: false,
        })
    }

    /// Execute raw SQL query
    pub async fn raw_sql(&self, query: &str) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query(query).fetch_all(&self.pool).await?;
        let mut results = Vec::new();
        for row in rows {
            let mut obj = serde_json::Map::new();
            let columns = row.columns();
            for (i, col) in columns.iter().enumerate() {
                let col_name = col.name();
                let value: serde_json::Value = row.try_get(i).unwrap_or(serde_json::Value::Null);
                obj.insert(col_name.to_string(), value);
            }
            results.push(serde_json::Value::Object(obj));
        }
        Ok(results)
    }
}
