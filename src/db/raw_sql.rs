use super::Database;
use anyhow::Result;
use sqlx::{Column, Row};

impl Database {
    /// Execute raw SQL query
    pub async fn raw_sql(&self, query: &str) -> Result<Vec<serde_json::Value>> {
        use sqlx::{TypeInfo, ValueRef};
        // This is intentionally the raw-SQL API. The complete statement is
        // supplied by the caller and is not composed with any other input.
        let rows = sqlx::query(sqlx::AssertSqlSafe(query))
            .fetch_all(&self.pool)
            .await?;
        let mut results = Vec::new();
        for row in rows {
            let mut obj = serde_json::Map::new();
            let columns = row.columns();
            for (i, col) in columns.iter().enumerate() {
                let col_name = col.name();
                let value = if let Ok(raw) = row.try_get_raw(i) {
                    if raw.is_null() {
                        serde_json::Value::Null
                    } else {
                        match raw.type_info().name() {
                            "INTEGER" => row
                                .try_get::<i64, _>(i)
                                .map(|v| serde_json::Value::Number(v.into()))
                                .unwrap_or(serde_json::Value::Null),
                            "REAL" | "FLOAT" => row
                                .try_get::<f64, _>(i)
                                .ok()
                                .and_then(serde_json::Number::from_f64)
                                .map(serde_json::Value::Number)
                                .unwrap_or(serde_json::Value::Null),
                            "BOOLEAN" => row
                                .try_get::<bool, _>(i)
                                .map(serde_json::Value::Bool)
                                .unwrap_or(serde_json::Value::Null),
                            "TEXT" => row
                                .try_get::<String, _>(i)
                                .map(serde_json::Value::String)
                                .unwrap_or(serde_json::Value::Null),
                            "BLOB" => {
                                if let Ok(bytes) = row.try_get::<Vec<u8>, _>(i) {
                                    serde_json::Value::String(base64::Engine::encode(
                                        &base64::engine::general_purpose::STANDARD,
                                        &bytes,
                                    ))
                                } else {
                                    serde_json::Value::Null
                                }
                            }
                            _ => {
                                if let Ok(s) = row.try_get::<String, _>(i) {
                                    serde_json::Value::String(s)
                                } else if let Ok(n) = row.try_get::<i64, _>(i) {
                                    serde_json::Value::Number(n.into())
                                } else if let Ok(f) = row.try_get::<f64, _>(i) {
                                    serde_json::Number::from_f64(f)
                                        .map(serde_json::Value::Number)
                                        .unwrap_or(serde_json::Value::Null)
                                } else {
                                    serde_json::Value::Null
                                }
                            }
                        }
                    }
                } else {
                    serde_json::Value::Null
                };
                obj.insert(col_name.to_string(), value);
            }
            results.push(serde_json::Value::Object(obj));
        }
        Ok(results)
    }
}
