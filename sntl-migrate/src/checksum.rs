use sha2::{Digest, Sha256};

/// Compute a stable short hash of the migration SQL text.
///
/// Used to detect "applied migration file was modified after apply".
/// 13-char prefix matches the `.sentinel/queries/<hash>.json` format
/// chosen by `sntl-schema::normalize`.
pub fn sha256_of_sql(sql: &str) -> String {
    let digest = Sha256::digest(sql.as_bytes());
    hex::encode(&digest[..7])[..13].to_string()
}
