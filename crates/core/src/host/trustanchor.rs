//! Versioned public IANA root trust data, never network-fetched at accepted boot.
use super::*;
pub fn prepared(_app: &HostApplication, ctx: &OperationContext) -> Result<Vec<u8>> {
    let key = include_bytes!("../../../../platform/etc/unbound/root.key");
    let metadata: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../platform/etc/unbound/root-anchors.lock.json"
    ))
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "embedded trust anchor metadata invalid",
        )
    })?;
    let actual = config::digest(key);
    if metadata
        .get("rootKeySha256")
        .and_then(serde_json::Value::as_str)
        != actual.as_str().strip_prefix("sha256:")
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "embedded public trust anchor hash mismatch",
        ));
    }
    Ok(key.to_vec())
}
