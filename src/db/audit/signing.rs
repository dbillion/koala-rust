use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};

pub struct SignedHead {
    pub key_id: String,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

pub fn sign(head: &str) -> Result<Option<SignedHead>> {
    let Ok(encoded) = std::env::var("KOALA_AUDIT_SIGNING_KEY") else {
        return Ok(None);
    };
    let seed = STANDARD.decode(encoded.trim())?;
    if seed.len() != 32 {
        bail!("KOALA_AUDIT_SIGNING_KEY must be a base64 Ed25519 32-byte seed");
    }
    let pair = Ed25519KeyPair::from_seed_unchecked(&seed)
        .map_err(|_| anyhow::anyhow!("invalid Ed25519 audit signing key"))?;
    Ok(Some(SignedHead {
        key_id: std::env::var("KOALA_AUDIT_KEY_ID").unwrap_or_else(|_| "default".into()),
        public_key: pair.public_key().as_ref().to_vec(),
        signature: pair.sign(head.as_bytes()).as_ref().to_vec(),
    }))
}

pub fn verify(head: &str, public_key: &[u8], signature: &[u8]) -> Result<()> {
    UnparsedPublicKey::new(&ED25519, public_key)
        .verify(head.as_bytes(), signature)
        .map_err(|_| anyhow::anyhow!("audit head signature is invalid"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ed25519_head_signature_detects_tampering() {
        let pair = Ed25519KeyPair::from_seed_unchecked(&[7u8; 32]).unwrap();
        let signature = pair.sign(b"head-hash");
        verify("head-hash", pair.public_key().as_ref(), signature.as_ref()).unwrap();
        assert!(verify(
            "changed-head",
            pair.public_key().as_ref(),
            signature.as_ref()
        )
        .is_err());
    }
}
