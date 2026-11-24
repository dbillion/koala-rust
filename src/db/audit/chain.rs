use sha2::{Digest, Sha256};

pub const ZERO_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

pub fn hash(previous: &str, fields: &[&str]) -> String {
    let mut digest = Sha256::new();
    for field in std::iter::once(previous).chain(fields.iter().copied()) {
        digest.update((field.len() as u64).to_be_bytes());
        digest.update(field.as_bytes());
    }
    hex::encode(digest.finalize())
}
