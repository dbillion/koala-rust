mod open_nodes;

use super::Database;
use anyhow::Result;
use sqlx::Row;

impl Database {
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
        super::audit::append_tx(&mut tx, &super::audit::actor(), "entity.create", name,
            serde_json::json!({"rowid": rowid, "type": type_, "project": project, "observations": observations.len()})).await?;
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
        if added > 0 {
            super::audit::append_tx(
                &mut tx,
                &super::audit::actor(),
                "entity.observations.add",
                name,
                serde_json::json!({"count": added}),
            )
            .await?;
        }
        tx.commit().await?;
        Ok(added)
    }
}
