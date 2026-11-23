use super::Database;
use anyhow::Result;
use chrono::{Duration, Utc};
use sqlx::Row;

impl Database {
    // ===== FACTS =====
    pub async fn write_fact(&self, name: &str, text: &str) -> Result<i64> {
        self.write_fact_with_ttl(name, text, None).await
    }

    /// Write a new fact version with an optional TTL in seconds.
    pub async fn write_fact_with_ttl(
        &self,
        name: &str,
        text: &str,
        ttl_seconds: Option<i64>,
    ) -> Result<i64> {
        if ttl_seconds.is_some_and(|ttl| ttl <= 0) {
            anyhow::bail!("ttl_seconds must be greater than zero");
        }
        let expires_at = ttl_seconds.map(|ttl| Utc::now() + Duration::seconds(ttl));
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM facts_tombstone WHERE name = ?")
            .bind(name)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE facts SET is_current = 0 WHERE name = ? AND is_current = 1")
            .bind(name)
            .execute(&mut *tx)
            .await?;

        let row = sqlx::query(
            "INSERT INTO facts (name, text, is_current, expires_at) VALUES (?, ?, 1, ?) RETURNING rowid",
        )
        .bind(name)
        .bind(text)
        .bind(expires_at)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(row.get("rowid"))
    }

    pub async fn read_fact(&self, name: &str) -> Result<Option<(i64, String)>> {
        let row = sqlx::query("SELECT rowid, text FROM facts WHERE name = ? AND is_current = 1 AND (expires_at IS NULL OR datetime(expires_at) > CURRENT_TIMESTAMP)")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| (r.get("rowid"), r.get("text"))))
    }

    /// Fetch a fact by rowid. Returns (name, text, is_current).
    pub async fn read_fact_by_rowid(&self, rowid: i64) -> Result<Option<(String, String, bool)>> {
        let row = sqlx::query("SELECT name, text, (is_current = 1 AND (expires_at IS NULL OR datetime(expires_at) > CURRENT_TIMESTAMP)) AS is_current FROM facts WHERE rowid = ?")
            .bind(rowid)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| (r.get("name"), r.get("text"), r.get::<bool, _>("is_current"))))
    }

    /// All current facts as (rowid, name, text).
    pub async fn current_facts(&self) -> Result<Vec<(i64, String, String)>> {
        let rows = sqlx::query("SELECT rowid, name, text FROM facts WHERE is_current = 1 AND (expires_at IS NULL OR datetime(expires_at) > CURRENT_TIMESTAMP)")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.get("rowid"), r.get("name"), r.get("text")))
            .collect())
    }

    pub async fn list_facts(&self) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT name FROM facts WHERE is_current = 1 AND (expires_at IS NULL OR datetime(expires_at) > CURRENT_TIMESTAMP) ORDER BY name")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.get("name")).collect())
    }

    /// Permanently remove expired rows. If a current version expires, all of
    /// that fact's retained versions are purged and a text-free tombstone is kept.
    /// Expired historical versions of still-live facts are removed individually.
    pub async fn purge_expired_facts(&self) -> Result<Vec<i64>> {
        let mut tx = self.pool.begin().await?;
        let expired_names: Vec<String> = sqlx::query(
            "SELECT DISTINCT name FROM facts WHERE is_current = 1 AND expires_at IS NOT NULL AND datetime(expires_at) <= CURRENT_TIMESTAMP",
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|row| row.get("name"))
        .collect();
        let rowids: Vec<i64> = sqlx::query(
            "SELECT rowid FROM facts WHERE (expires_at IS NOT NULL AND datetime(expires_at) <= CURRENT_TIMESTAMP) OR name IN (SELECT name FROM facts WHERE is_current = 1 AND expires_at IS NOT NULL AND datetime(expires_at) <= CURRENT_TIMESTAMP)",
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|row| row.get("rowid"))
        .collect();
        for rowid in &rowids {
            sqlx::query("DELETE FROM facts WHERE rowid = ?")
                .bind(rowid)
                .execute(&mut *tx)
                .await?;
        }
        for name in expired_names {
            sqlx::query("INSERT OR REPLACE INTO facts_tombstone (name, actor, reason) VALUES (?, 'local', 'ttl_expired')")
                .bind(name)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(rowids)
    }

    pub async fn delete_fact(&self, name: &str) -> Result<bool> {
        self.delete_fact_with_reason(name, "local", "user_requested")
            .await
    }

    pub async fn delete_fact_with_reason(
        &self,
        name: &str,
        actor: &str,
        reason: &str,
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let deleted = sqlx::query("DELETE FROM facts WHERE name = ?")
            .bind(name)
            .execute(&mut *tx)
            .await?
            .rows_affected();

        if deleted > 0 {
            sqlx::query(
                "INSERT OR REPLACE INTO facts_tombstone (name, actor, reason) VALUES (?, ?, ?)",
            )
            .bind(name)
            .bind(actor)
            .bind(reason)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(deleted > 0)
    }

    /// Reclaim pages released by a secure purge. This is effective when the
    /// database has incremental auto-vacuum enabled during initialization.
    pub async fn incremental_vacuum(&self) -> Result<()> {
        sqlx::query("PRAGMA incremental_vacuum")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn delete_fact_purges_all_versions_and_keeps_text_free_tombstone() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("CREATE TABLE facts (rowid INTEGER PRIMARY KEY, name TEXT, text TEXT, is_current BOOLEAN, expires_at DATETIME)")
            .execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE facts_tombstone (name TEXT PRIMARY KEY, deleted_at DATETIME DEFAULT CURRENT_TIMESTAMP, actor TEXT NOT NULL DEFAULT 'local', reason TEXT NOT NULL DEFAULT 'user_requested')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO facts (name, text, is_current) VALUES ('private', 'old secret', 0), ('private', 'new secret', 1)")
            .execute(&pool).await.unwrap();
        let db = Database { pool: pool.clone() };

        assert!(db.delete_fact("private").await.unwrap());
        let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM facts WHERE name='private'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let tombstone: String =
            sqlx::query_scalar("SELECT name FROM facts_tombstone WHERE name='private'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(remaining, 0);
        assert_eq!(tombstone, "private");
        let audit: (String, String) =
            sqlx::query_as("SELECT actor, reason FROM facts_tombstone WHERE name='private'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(audit, ("local".to_string(), "user_requested".to_string()));
        assert!(!db.delete_fact("private").await.unwrap());
    }
}

#[cfg(test)]
mod ttl_tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn expired_current_fact_purges_history_and_records_tombstone() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("CREATE TABLE facts (rowid INTEGER PRIMARY KEY, name TEXT, text TEXT, is_current BOOLEAN, expires_at DATETIME)").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE facts_tombstone (name TEXT PRIMARY KEY, deleted_at DATETIME DEFAULT CURRENT_TIMESTAMP, actor TEXT NOT NULL DEFAULT 'local', reason TEXT NOT NULL DEFAULT 'user_requested')").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO facts (name,text,is_current,expires_at) VALUES ('session','old',0,datetime('now','-2 days')),('session','new',1,datetime('now','-1 second'))").execute(&pool).await.unwrap();
        let db = Database { pool: pool.clone() };
        let rowids = db.purge_expired_facts().await.unwrap();
        assert_eq!(rowids.len(), 2);
        let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM facts WHERE name='session'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let reason: String =
            sqlx::query_scalar("SELECT reason FROM facts_tombstone WHERE name='session'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(remaining, 0);
        assert_eq!(reason, "ttl_expired");
    }
}
