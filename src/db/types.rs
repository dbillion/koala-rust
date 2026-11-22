use chrono::{DateTime, Utc};

#[derive(Debug, serde::Serialize)]
pub struct EntityNode {
    pub name: String,
    pub type_: Option<String>,
    pub project: Option<String>,
    pub observations: Vec<String>,
    pub relations: Vec<Relation>,
}

#[derive(Debug, serde::Serialize)]
pub struct Relation {
    pub from: String,
    pub to: String,
    pub rel_type: String,
}

#[derive(Debug, serde::Serialize)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub status: String,
    pub project: String,
    pub priority: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, serde::Serialize)]
pub struct RankExplanation {
    pub name: String,
    pub signal: String,
    pub score: f64,
}

#[derive(Debug, serde::Serialize)]
pub struct HealthInfo {
    pub tables: i64,
    pub pages: i64,
    pub journal: bool,
    pub cache_used: i64,
    pub usage_tracked: i64,
    pub pending_flush: bool,
}
