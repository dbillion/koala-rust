use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::Koala;

/// User profile and working preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub stack: Vec<String>,
    pub preferences: Vec<String>,
    pub general_facts: Vec<Value>,
}

/// Project-specific context including architecture, decisions, and status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub project: String,
    pub architecture: Option<String>,
    pub status: Option<Value>,
    pub recent_decisions: Vec<Value>,
    pub latest_handoff: Option<Value>,
    pub open_tasks: Vec<Value>,
}

/// Unified context payload for session bootstrap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapContext {
    pub user: UserContext,
    pub project: ProjectContext,
}

/// Immutable handoff checkpoint record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffCheckpoint {
    pub project: String,
    pub summary: String,
    pub next_step: String,
    pub blockers: Vec<String>,
    pub timestamp: String,
}

impl Koala {
    /// Retrieve current user working profile and preferences
    pub async fn get_user_context(&self) -> Result<UserContext> {
        let mut stack = Vec::new();
        let mut preferences = Vec::new();
        let mut general_facts = Vec::new();

        if let Ok(Some((_, text))) = self.db.read_fact("user_stack").await {
            stack.push(text);
        }
        if let Ok(Some((_, text))) = self.db.read_fact("user_preference").await {
            preferences.push(text);
        }
        if let Ok(Some((_, text))) = self.db.read_fact("business_need").await {
            preferences.push(text);
        }

        // Search for other user:* facts
        if let Ok(all_facts) = self.db.list_facts().await {
            for fact_name in all_facts {
                if fact_name.starts_with("user:") {
                    if let Ok(Some((id, text))) = self.db.read_fact(&fact_name).await {
                        general_facts.push(json!({
                            "id": id,
                            "name": fact_name,
                            "text": text
                        }));
                    }
                }
            }
        }

        Ok(UserContext {
            stack,
            preferences,
            general_facts,
        })
    }

    /// Restore project architecture, active status, decisions, and handoff
    pub async fn get_project_context(&self, project: &str) -> Result<ProjectContext> {
        let arch_key = format!("project:{}:architecture", project);
        let status_key = format!("project:{}:status", project);
        let handoff_key = format!("handoff:{}:latest", project);

        // Architecture
        let architecture = if let Ok(Some((_, text))) = self.db.read_fact(&arch_key).await {
            Some(text)
        } else if let Ok(Some((_, text))) = self.db.read_fact(&format!("project:{}", project)).await {
            Some(text)
        } else {
            None
        };

        // Status
        let status = if let Ok(Some((_, text))) = self.db.read_fact(&status_key).await {
            serde_json::from_str::<Value>(&text).ok().or_else(|| Some(json!({"status": text})))
        } else {
            None
        };

        // Latest handoff
        let latest_handoff = if let Ok(Some((_, text))) = self.db.read_fact(&handoff_key).await {
            serde_json::from_str::<Value>(&text).ok().or_else(|| Some(json!({"summary": text})))
        } else {
            None
        };

        // Recent decisions
        let mut recent_decisions = Vec::new();
        let dec_prefix = format!("project:{}:decision", project);
        if let Ok(facts) = self.db.list_facts().await {
            for fact_name in facts {
                if fact_name.starts_with(&dec_prefix) {
                    if let Ok(Some((_, text))) = self.db.read_fact(&fact_name).await {
                        if let Ok(val) = serde_json::from_str::<Value>(&text) {
                            recent_decisions.push(val);
                        } else {
                            recent_decisions.push(json!({"decision": text, "name": fact_name}));
                        }
                    }
                }
            }
        }

        // Open tasks
        let mut open_tasks = Vec::new();
        if let Ok(tasks) = self.db.query_tasks(None, Some(project), None, 20).await {
            for t in tasks {
                if t.status != "done" && t.status != "completed" {
                    open_tasks.push(json!({
                        "id": t.id,
                        "title": t.title,
                        "status": t.status,
                        "priority": t.priority,
                        "body": t.body
                    }));
                }
            }
        }

        Ok(ProjectContext {
            project: project.to_string(),
            architecture,
            status,
            recent_decisions,
            latest_handoff,
            open_tasks,
        })
    }

    /// Unified bootstrap combining user context and project context
    pub async fn bootstrap_context(&self, project: &str) -> Result<BootstrapContext> {
        let user = self.get_user_context().await?;
        let project_ctx = self.get_project_context(project).await?;
        Ok(BootstrapContext {
            user,
            project: project_ctx,
        })
    }

    /// Record an architectural or technical decision
    pub async fn record_decision(
        &self,
        project: &str,
        decision: &str,
        rationale: Option<&str>,
    ) -> Result<Value> {
        let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
        let fact_name = format!("project:{}:decision:{}", project, timestamp);
        let payload = json!({
            "project": project,
            "decision": decision,
            "rationale": rationale,
            "recorded_at": Utc::now().to_rfc3339()
        });
        let payload_str = serde_json::to_string(&payload)?;

        self.remember_semantic(&fact_name, &payload_str).await?;

        // Also add observation to project entity
        let _ = self.db.create_entity(project, Some("project"), Some(project), &[]).await;
        let obs_text = if let Some(r) = rationale {
            format!("Decision: {} (Rationale: {})", decision, r)
        } else {
            format!("Decision: {}", decision)
        };
        let _ = self.db.add_observations(project, &[obs_text]).await;

        Ok(payload)
    }

    /// Update project status and working state
    pub async fn update_project_status(
        &self,
        project: &str,
        status: &str,
        blockers: &[String],
        next_step: Option<&str>,
    ) -> Result<Value> {
        let status_key = format!("project:{}:status", project);
        let payload = json!({
            "project": project,
            "status": status,
            "blockers": blockers,
            "next_step": next_step,
            "updated_at": Utc::now().to_rfc3339()
        });
        let payload_str = serde_json::to_string(&payload)?;

        self.remember_semantic(&status_key, &payload_str).await?;
        Ok(payload)
    }

    /// Create and persist an immutable handoff checkpoint
    pub async fn create_handoff(
        &self,
        project: &str,
        summary: &str,
        next_step: &str,
        blockers: &[String],
    ) -> Result<HandoffCheckpoint> {
        let now_str = Utc::now().to_rfc3339();
        let checkpoint = HandoffCheckpoint {
            project: project.to_string(),
            summary: summary.to_string(),
            next_step: next_step.to_string(),
            blockers: blockers.to_vec(),
            timestamp: now_str.clone(),
        };

        let checkpoint_json = serde_json::to_string(&checkpoint)?;
        let latest_key = format!("handoff:{}:latest", project);
        let timestamp_key = format!(
            "handoff:{}:{}",
            project,
            Utc::now().format("%Y%m%d%H%M%S")
        );

        // Store latest handoff and historical checkpoint
        self.remember_semantic(&latest_key, &checkpoint_json).await?;
        self.remember_semantic(&timestamp_key, &checkpoint_json).await?;

        // Automatically update project status to reflect the checkpoint
        let _ = self.update_project_status(project, summary, blockers, Some(next_step)).await;

        Ok(checkpoint)
    }

    /// Search memory across facts and handoffs with optional project filter
    pub async fn search_memory(
        &self,
        query: &str,
        project: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<Value>> {
        let k = limit.unwrap_or(5);
        let hits = self.recall(query, k * 2, 0.3).await?;
        let mut results = Vec::new();

        for hit in hits {
            if let Some(proj) = project {
                if !hit.name.contains(proj) && !hit.text.contains(proj) {
                    continue;
                }
            }
            results.push(json!({
                "rowid": hit.rowid,
                "name": hit.name,
                "text": hit.text,
                "score": hit.score,
                "distance": hit.distance,
                "keyword_overlap": hit.keyword_overlap
            }));
            if results.len() >= k {
                break;
            }
        }

        Ok(results)
    }
}
