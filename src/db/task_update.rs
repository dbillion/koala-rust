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
}
