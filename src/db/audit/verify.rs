use super::{
    chain, signing,
    types::{AuditRecord, AuditVerification},
};
use crate::db::Database;
use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use sqlx::{Row, SqlitePool};
async fn records(pool: &SqlitePool) -> Result<Vec<AuditRecord>> {
    let rows = sqlx::query("SELECT id,timestamp,actor,action,subject,details_json,previous_hash,entry_hash FROM audit_log ORDER BY id")
        .fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|r| AuditRecord {
            id: r.get("id"),
            timestamp: r.get("timestamp"),
            actor: r.get("actor"),
            action: r.get("action"),
            subject: r.get("subject"),
            details_json: r.get("details_json"),
            previous_hash: r.get("previous_hash"),
            entry_hash: r.get("entry_hash"),
        })
        .collect())
}

impl Database {
    pub async fn audit_export(&self) -> Result<Vec<AuditRecord>> {
        records(&self.pool).await
    }

    pub async fn audit_verify(&self) -> Result<AuditVerification> {
        let rows = records(&self.pool).await?;
        let mut previous = chain::ZERO_HASH.to_string();
        for row in &rows {
            if row.previous_hash != previous {
                bail!("audit chain previous hash mismatch at row {}", row.id);
            }
            let fields = [
                row.timestamp.as_str(),
                row.actor.as_str(),
                row.action.as_str(),
                row.subject.as_str(),
                row.details_json.as_str(),
            ];
            let expected = chain::hash(&previous, &fields);
            if row.entry_hash != expected {
                bail!("audit chain entry hash mismatch at row {}", row.id);
            }
            previous = expected;
        }
        let head = sqlx::query(
            "SELECT head_hash,key_id,public_key,signature FROM audit_head WHERE singleton=1",
        )
        .fetch_one(&self.pool)
        .await?;
        let head_hash: String = head.get("head_hash");
        if head_hash != previous {
            bail!("audit head does not match the final log entry");
        }
        let key_id: Option<String> = head.get("key_id");
        let public_key: Option<Vec<u8>> = head.get("public_key");
        let signature: Option<Vec<u8>> = head.get("signature");
        let signed = match (&public_key, &signature) {
            (Some(key), Some(sig)) => {
                signing::verify(&head_hash, key, sig)?;
                true
            }
            (None, None) => false,
            _ => bail!("audit head has an incomplete signature"),
        };
        if std::env::var("KOALA_AUDIT_REQUIRE_SIGNATURE").as_deref() == Ok("1") && !signed {
            bail!("audit head is not signed");
        }
        let trusted_key = if let Ok(encoded) = std::env::var("KOALA_AUDIT_TRUSTED_PUBLIC_KEY") {
            let trusted = STANDARD.decode(encoded.trim())?;
            if public_key.as_deref() != Some(trusted.as_slice()) {
                bail!("audit public key does not match the externally pinned key");
            }
            true
        } else {
            false
        };
        if std::env::var("KOALA_AUDIT_REQUIRE_TRUSTED_KEY").as_deref() == Ok("1") && !trusted_key {
            bail!("KOALA_AUDIT_TRUSTED_PUBLIC_KEY is required to verify the audit signer");
        }
        Ok(AuditVerification {
            entries: rows.len(),
            head_hash,
            signed,
            trusted_key,
            key_id,
            public_key: public_key.map(|key| STANDARD.encode(key)),
            signature: signature.map(|sig| STANDARD.encode(sig)),
        })
    }
}
