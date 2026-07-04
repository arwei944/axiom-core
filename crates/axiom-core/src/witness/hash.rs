use crate::version::SchemaVersion;

pub fn compute_signal_fingerprint(
    signal_type: &str,
    schema_version: SchemaVersion,
    payload: &serde_json::Value,
) -> crate::Result<[u8; 32]> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(signal_type.as_bytes());
    hasher.update(schema_version.0.to_le_bytes());
    let bytes =
        serde_json::to_vec(payload).map_err(|e| crate::AxiomError::WitnessSerialization {
            cell_id: "unknown".into(),
            message: format!("signal fingerprint payload: {e}"),
        })?;
    hasher.update(&bytes);
    let result = hasher.finalize();
    let mut fp = [0u8; 32];
    fp.copy_from_slice(&result);
    Ok(fp)
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut truncated = s.chars().take(max).collect::<String>();
        truncated.push_str("...");
        truncated
    }
}
