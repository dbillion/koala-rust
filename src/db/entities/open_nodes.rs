use crate::db::{Database, EntityNode, Relation};
use anyhow::Result;
use sqlx::Row;

impl Database {
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
                    .bind(name).bind(name).fetch_all(&self.pool).await?.into_iter().map(|r| Relation {
                        from: r.get("from_entity"), to: r.get("to_entity"), rel_type: r.get("rel_type"),
                    }).collect();
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
