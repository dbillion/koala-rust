use super::{Database, Task};
use anyhow::Result;
use sqlx::Row;

impl Database {
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
}
