use super::{Database, Task};
use anyhow::Result;
use sqlx::{QueryBuilder, Row, Sqlite};

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
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("INSERT INTO tasks (title, body, status, project, priority) VALUES (?, ?, ?, ?, ?) RETURNING rowid")
            .bind(title).bind(body).bind(status).bind(project).bind(priority).fetch_one(&mut *tx).await?;
        let rowid: i64 = row.get("rowid");
        super::audit::append_tx(
            &mut tx,
            &super::audit::actor(),
            "task.create",
            &format!("task:{}", rowid),
            serde_json::json!({"status": status, "project": project, "priority": priority}),
        )
        .await?;
        tx.commit().await?;
        Ok(rowid)
    }

    pub async fn query_tasks(
        &self,
        status: Option<&str>,
        project: Option<&str>,
        search: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Task>> {
        let mut query = QueryBuilder::<Sqlite>::new(
            "SELECT rowid, title, body, status, project, priority, created_at, updated_at FROM tasks WHERE 1=1",
        );
        if let Some(s) = status {
            query.push(" AND status = ").push_bind(s);
        }
        if let Some(p) = project {
            query.push(" AND project = ").push_bind(p);
        }
        if let Some(s) = search {
            query
                .push(" AND (title LIKE ")
                .push_bind(format!("%{}%", s))
                .push(" OR body LIKE ")
                .push_bind(format!("%{}%", s))
                .push(")");
        }
        query
            .push(" ORDER BY created_at DESC LIMIT ")
            .push_bind(limit);

        let rows = query.build().fetch_all(&self.pool).await?;
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
