use super::{Database, EntityNode, Relation};
use anyhow::Result;
use sqlx::Row;

impl Database {
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
}
