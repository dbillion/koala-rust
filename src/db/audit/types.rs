use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AuditRecord {
    pub id: i64,
    pub timestamp: String,
    pub actor: String,
    pub action: String,
    pub subject: String,
    pub details_json: String,
    pub previous_hash: String,
    pub entry_hash: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditVerification {
    pub entries: usize,
    pub head_hash: String,
    pub signed: bool,
    pub trusted_key: bool,
    pub key_id: Option<String>,
    pub public_key: Option<String>,
    pub signature: Option<String>,
}
