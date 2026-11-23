use super::Database;
use anyhow::Result;
use sqlx::Row;

impl Database {
    /// Names whose current fact version has expired, before purge removes them.
    pub async fn expired_fact_names(&self) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT DISTINCT name FROM facts WHERE is_current = 1 AND expires_at IS NOT NULL AND datetime(expires_at) <= CURRENT_TIMESTAMP",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|row| row.get("name")).collect())
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
                .bind(&name).execute(&mut *tx).await?;
            super::super::audit::append_tx(
                &mut tx,
                &super::super::audit::actor(),
                "fact.expire",
                &name,
                serde_json::json!({"reason": "ttl_expired"}),
            )
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
            super::super::audit::append_tx(
                &mut tx,
                actor,
                "fact.delete",
                name,
                serde_json::json!({"reason": reason, "versions_deleted": deleted}),
            )
            .await?;
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
}
