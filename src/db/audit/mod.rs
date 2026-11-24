mod chain;
mod signing;
mod types;
mod verify;

use anyhow::{bail, Result};
use chrono::Utc;
use serde_json::Value;
use sqlx::{Sqlite, Transaction};

pub use types::{AuditRecord, AuditVerification};

pub fn actor() -> String {
    std::env::var("KOALA_AUDIT_ACTOR").unwrap_or_else(|_| "local".into())
}

pub(super) async fn append_tx(
    tx: &mut Transaction<'_, Sqlite>,
    actor: &str,
    action: &str,
    subject: &str,
    details: Value,
) -> Result<()> {
    let previous: Option<String> =
        sqlx::query_scalar("SELECT entry_hash FROM audit_log ORDER BY id DESC LIMIT 1")
            .fetch_optional(&mut **tx)
            .await?;
    let previous = previous.unwrap_or_else(|| chain::ZERO_HASH.into());
    let timestamp = Utc::now().to_rfc3339();
    let details_json = serde_json::to_string(&details)?;
    let fields = [
        timestamp.as_str(),
        actor,
        action,
        subject,
        details_json.as_str(),
    ];
    let entry_hash = chain::hash(&previous, &fields);
    let signed = signing::sign(&entry_hash)?;
    if std::env::var("KOALA_AUDIT_REQUIRE_SIGNATURE").as_deref() == Ok("1") && signed.is_none() {
        bail!("KOALA_AUDIT_SIGNING_KEY is required for audited writes");
    }
    sqlx::query("INSERT INTO audit_log (timestamp,actor,action,subject,details_json,previous_hash,entry_hash) VALUES (?,?,?,?,?,?,?)")
        .bind(&timestamp).bind(actor).bind(action).bind(subject).bind(&details_json)
        .bind(&previous).bind(&entry_hash).execute(&mut **tx).await?;
    let (key_id, public_key, signature) = signed.map_or((None, None, None), |s| {
        (Some(s.key_id), Some(s.public_key), Some(s.signature))
    });
    sqlx::query("UPDATE audit_head SET head_hash=?,key_id=?,public_key=?,signature=?,updated_at=? WHERE singleton=1")
        .bind(entry_hash).bind(key_id).bind(public_key).bind(signature).bind(timestamp)
        .execute(&mut **tx).await?;
    Ok(())
}

#[cfg(test)]
mod tests;
