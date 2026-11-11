//! Multi-Agent Sync Module (v0.3) - Simplified CRDT Implementation
//!
//! Provides basic conflict-free replicated data types for concurrent agent access.
//! Uses a simple last-write-wins with vector clocks for now.

use anyhow::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Document types that can be synced
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyncDocType {
    Facts,
    Entities,
    Relations,
    Tasks,
    Thoughts,
    Model,
}

/// Vector clock for conflict detection
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorClock {
    #[serde(with = "uuid_serde")]
    clocks: HashMap<Uuid, u64>,
}

mod uuid_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;
    use uuid::Uuid;

    pub(super) fn serialize<S>(map: &HashMap<Uuid, u64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string_map: HashMap<String, u64> =
            map.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        string_map.serialize(serializer)
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<Uuid, u64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string_map: HashMap<String, u64> = HashMap::deserialize(deserializer)?;
        let mut map = HashMap::new();
        for (k, v) in string_map {
            if let Ok(uuid) = Uuid::parse_str(&k) {
                map.insert(uuid, v);
            }
        }
        Ok(map)
    }
}

impl VectorClock {
    pub fn increment(&mut self, peer_id: Uuid) {
        let entry = self.clocks.entry(peer_id).or_insert(0);
        *entry += 1;
    }

    pub fn merge(&mut self, other: &VectorClock) {
        for (peer_id, time) in other.clocks.iter() {
            self.clocks
                .entry(*peer_id)
                .and_modify(|t| *t = (*t).max(*time))
                .or_insert(*time);
        }
    }

    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let mut dominated = false;
        for (peer_id, time) in self.clocks.iter() {
            let other_time = other.clocks.get(peer_id).copied().unwrap_or(0);
            if *time > other_time {
                return false;
            }
            if *time < other_time {
                dominated = true;
            }
        }
        dominated
    }
}

/// Sync document entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub vector_clock: VectorClock,
    pub peer_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub deleted: bool,
}

/// Sync document
pub struct SyncDoc {
    pub doc_type: SyncDocType,
    pub entries: DashMap<String, SyncEntry>,
    pub peer_id: Uuid,
}

/// Multi-agent sync manager
pub struct SyncManager {
    docs: DashMap<SyncDocType, Arc<SyncDoc>>,
    peer_id: Uuid,
}

impl SyncManager {
    pub async fn new(peer_id: Option<Uuid>) -> Result<Self> {
        let peer_id = peer_id.unwrap_or_else(Uuid::new_v4);
        let manager = Self {
            docs: DashMap::new(),
            peer_id,
        };

        for doc_type in [
            SyncDocType::Facts,
            SyncDocType::Entities,
            SyncDocType::Relations,
            SyncDocType::Tasks,
            SyncDocType::Thoughts,
            SyncDocType::Model,
        ] {
            manager.init_doc(doc_type).await?;
        }

        Ok(manager)
    }

    async fn init_doc(&self, doc_type: SyncDocType) -> Result<()> {
        let doc = Arc::new(SyncDoc {
            doc_type,
            entries: DashMap::new(),
            peer_id: self.peer_id,
        });
        self.docs.insert(doc_type, doc);
        Ok(())
    }

    /// Apply a local change
    pub async fn apply_change<F>(&self, doc_type: SyncDocType, f: F) -> Result<()>
    where
        F: FnOnce(&DashMap<String, SyncEntry>) -> Result<()>,
    {
        let doc = self
            .docs
            .get(&doc_type)
            .ok_or_else(|| anyhow::anyhow!("Document not found: {:?}", doc_type))?;
        f(&doc.entries)
    }

    /// Get current document state as JSON
    pub async fn get_state(&self, doc_type: SyncDocType) -> Result<serde_json::Value> {
        let doc = self
            .docs
            .get(&doc_type)
            .ok_or_else(|| anyhow::anyhow!("Document not found: {:?}", doc_type))?;

        let mut map = serde_json::Map::new();
        for entry in doc.entries.iter() {
            map.insert(entry.key().clone(), entry.value().value.clone());
        }
        Ok(serde_json::Value::Object(map))
    }

    /// Get pending changes to send to peers
    pub async fn get_pending_changes(&self, doc_type: SyncDocType) -> Result<Vec<SyncEntry>> {
        let doc = self
            .docs
            .get(&doc_type)
            .ok_or_else(|| anyhow::anyhow!("Document not found: {:?}", doc_type))?;

        let changes: Vec<SyncEntry> = doc.entries.iter().map(|e| e.value().clone()).collect();
        Ok(changes)
    }

    /// Merge changes from another peer
    pub async fn merge_changes(&self, doc_type: SyncDocType, changes: &[SyncEntry]) -> Result<()> {
        let doc = self
            .docs
            .get(&doc_type)
            .ok_or_else(|| anyhow::anyhow!("Document not found: {:?}", doc_type))?;

        for change in changes {
            doc.entries
                .entry(change.key.clone())
                .and_modify(|existing| {
                    if change.vector_clock.happens_before(&existing.vector_clock) {
                        // Existing is newer, keep it
                    } else if existing.vector_clock.happens_before(&change.vector_clock) {
                        // Change is newer, replace
                        *existing = change.clone();
                    } else {
                        // Concurrent - last write wins by timestamp
                        if change.timestamp > existing.timestamp {
                            *existing = change.clone();
                        }
                    }
                })
                .or_insert_with(|| change.clone());
        }
        Ok(())
    }

    /// Sync facts: write_fact with CRDT
    pub async fn sync_write_fact(&self, name: &str, text: &str) -> Result<i64> {
        // Mask to the low 63 bits so the id is always positive (u128->i64 cast can set the sign bit).
        let id = ((uuid::Uuid::new_v4().as_u128() as u64 & 0x7FFF_FFFF_FFFF_FFFF) as i64).max(1);

        self.apply_change(SyncDocType::Facts, |entries| {
            let mut vc = VectorClock::default();
            vc.increment(self.peer_id);

            entries.insert(
                name.to_string(),
                SyncEntry {
                    key: name.to_string(),
                    value: serde_json::json!({
                        "id": id,
                        "text": text,
                        "timestamp": Utc::now().timestamp()
                    }),
                    vector_clock: vc,
                    peer_id: self.peer_id,
                    timestamp: Utc::now(),
                    deleted: false,
                },
            );
            Ok(())
        })
        .await?;

        Ok(id)
    }

    /// Sync read fact
    pub async fn sync_read_fact(&self, name: &str) -> Result<Option<(i64, String)>> {
        let doc = self
            .docs
            .get(&SyncDocType::Facts)
            .ok_or_else(|| anyhow::anyhow!("Facts document not found"))?;

        if let Some(entry) = doc.entries.get(name) {
            if !entry.deleted {
                let id = entry.value.get("id").and_then(|v| v.as_i64());
                let text = entry.value.get("text").and_then(|v| v.as_str());
                if let (Some(id), Some(text)) = (id, text) {
                    return Ok(Some((id, text.to_string())));
                }
            }
        }
        Ok(None)
    }

    /// Sync list all facts
    pub async fn sync_list_facts(&self) -> Result<Vec<String>> {
        let doc = self
            .docs
            .get(&SyncDocType::Facts)
            .ok_or_else(|| anyhow::anyhow!("Facts document not found"))?;

        let mut names = Vec::new();
        for entry in doc.entries.iter() {
            if !entry.deleted {
                names.push(entry.key().clone());
            }
        }
        Ok(names)
    }

    /// Sync entities
    pub async fn sync_create_entity(
        &self,
        name: &str,
        type_: Option<&str>,
        project: Option<&str>,
        observations: &[String],
    ) -> Result<i64> {
        // Mask to the low 63 bits so the id is always positive (u128->i64 cast can set the sign bit).
        let id = ((uuid::Uuid::new_v4().as_u128() as u64 & 0x7FFF_FFFF_FFFF_FFFF) as i64).max(1);

        self.apply_change(SyncDocType::Entities, |entries| {
            let mut vc = VectorClock::default();
            vc.increment(self.peer_id);

            let mut obs_json = Vec::new();
            for obs in observations {
                obs_json.push(serde_json::json!(obs));
            }

            entries.insert(
                name.to_string(),
                SyncEntry {
                    key: name.to_string(),
                    value: serde_json::json!({
                        "id": id,
                        "type": type_,
                        "project": project,
                        "observations": obs_json,
                        "timestamp": Utc::now().timestamp()
                    }),
                    vector_clock: vc,
                    peer_id: self.peer_id,
                    timestamp: Utc::now(),
                    deleted: false,
                },
            );
            Ok(())
        })
        .await?;

        Ok(id)
    }

    /// Get peer ID
    pub fn peer_id(&self) -> Uuid {
        self.peer_id
    }
}

/// Sync message for network transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMessage {
    pub from_peer: Uuid,
    pub doc_type: SyncDocType,
    pub changes: Vec<SyncEntry>,
    pub timestamp: i64,
}

impl SyncMessage {
    pub fn new(from_peer: Uuid, doc_type: SyncDocType, changes: Vec<SyncEntry>) -> Self {
        Self {
            from_peer,
            doc_type,
            changes,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_manager() {
        let manager = SyncManager::new(None).await.unwrap();

        let id = manager
            .sync_write_fact("test_key", "test_value")
            .await
            .unwrap();
        assert!(id > 0);

        let fact = manager.sync_read_fact("test_key").await.unwrap();
        assert!(fact.is_some());
        assert_eq!(fact.unwrap().1, "test_value");

        let facts = manager.sync_list_facts().await.unwrap();
        assert!(facts.contains(&"test_key".to_string()));
    }

    #[tokio::test]
    async fn test_multi_peer_sync() {
        let peer1 = SyncManager::new(Some(Uuid::new_v4())).await.unwrap();
        let peer2 = SyncManager::new(Some(Uuid::new_v4())).await.unwrap();

        peer1
            .sync_write_fact("shared_key", "from_peer1")
            .await
            .unwrap();

        let changes = peer1.get_pending_changes(SyncDocType::Facts).await.unwrap();

        peer2
            .merge_changes(SyncDocType::Facts, &changes)
            .await
            .unwrap();

        let fact = peer2.sync_read_fact("shared_key").await.unwrap();
        assert!(fact.is_some());
        assert_eq!(fact.unwrap().1, "from_peer1");
    }
}
