use super::Database;
use anyhow::Result;
use sqlx::Row;

impl Database {
    /// Traverse the relations graph bidirectionally (multi-hop) from seed names.
    ///
    /// SQLite `WITH RECURSIVE` CTE; no graph extension needed.
    pub async fn create_relation(&self, from: &str, to: &str, rel_type: &str) -> Result<i64> {
        let row = sqlx::query("INSERT INTO relations (from_entity, to_entity, rel_type) VALUES (?, ?, ?) RETURNING rowid")
            .bind(from).bind(to).bind(rel_type).fetch_one(&self.pool).await?;
        Ok(row.get("rowid"))
    }

    pub async fn traverse_relations(
        &self,
        seeds: &[String],
        max_depth: i32,
    ) -> Result<Vec<(String, i32)>> {
        if seeds.is_empty() || max_depth < 0 {
            return Ok(Vec::new());
        }
        let mut results: Vec<(String, i32)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for seed in seeds {
            let sql = "
                WITH RECURSIVE reachable(name, depth) AS (
                    SELECT ?1, 0
                    UNION ALL
                    SELECT rel.to_entity, r.depth + 1 FROM reachable r
                    JOIN relations rel ON rel.from_entity = r.name
                    WHERE r.depth < ?2
                    UNION
                    SELECT rel.from_entity, r.depth + 1 FROM reachable r
                    JOIN relations rel ON rel.to_entity = r.name
                    WHERE r.depth < ?2
                )
                SELECT DISTINCT name, depth FROM reachable
            ";
            let rows = sqlx::query(sql)
                .bind(seed)
                .bind(max_depth)
                .bind(max_depth)
                .fetch_all(&self.pool)
                .await?;
            for row in rows {
                let name: String = row.get(0);
                let depth: i32 = row.get(1);
                if seen.insert(name.clone()) {
                    results.push((name, depth));
                }
            }
        }
        results.sort_by_key(|(_, d)| *d);
        Ok(results)
    }

    pub async fn search_nodes(&self, query: &str, limit: i64) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT DISTINCT name FROM (
                SELECT name FROM entities WHERE name LIKE ?
                UNION
                SELECT entity_name as name FROM observations WHERE content LIKE ?
                UNION
                SELECT name FROM facts WHERE name LIKE ? AND is_current = 1 AND (expires_at IS NULL OR datetime(expires_at) > CURRENT_TIMESTAMP)
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
}
