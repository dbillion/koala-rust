use super::Database;
use anyhow::Result;

impl Database {
    pub async fn update_task(
        &self,
        id: i64,
        title: Option<&str>,
        body: Option<&str>,
        status: Option<&str>,
        project: Option<&str>,
        priority: Option<&str>,
    ) -> Result<bool> {
        let mut query = sqlx::QueryBuilder::<sqlx::Sqlite>::new("UPDATE tasks SET ");
        let mut has_update = false;

        macro_rules! push_update {
            ($column:literal, $value:expr) => {
                if let Some(value) = $value {
                    if has_update {
                        query.push(", ");
                    }
                    query.push($column).push(" = ").push_bind(value);
                    has_update = true;
                }
            };
        }

        push_update!("title", title);
        push_update!("body", body);
        push_update!("status", status);
        push_update!("project", project);
        push_update!("priority", priority);

        if !has_update {
            return Ok(false);
        }

        query
            .push(", updated_at = CURRENT_TIMESTAMP WHERE rowid = ")
            .push_bind(id);

        let mut tx = self.pool.begin().await?;
        let result = query.build().execute(&mut *tx).await?;
        if result.rows_affected() > 0 {
            super::audit::append_tx(
                &mut tx,
                &super::audit::actor(),
                "task.update",
                &format!("task:{}", id),
                serde_json::json!({"title_changed": title.is_some(), "body_changed": body.is_some(),
                    "status": status, "project": project, "priority": priority}),
            )
            .await?;
        }
        tx.commit().await?;
        Ok(result.rows_affected() > 0)
    }
}
