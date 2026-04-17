use std::path::PathBuf;
use crate::error::LsqError;

pub const DEFAULT_HOST: &str = "api.leadsquared.com";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum number of files kept in the output directory.
/// When exceeded, the oldest files are pruned automatically.
pub const MAX_OUTPUT_FILES: usize = 100;

/// Base URL for standard v2 API endpoints.
pub fn api_base(host: &str) -> String {
    format!("https://{}/v2", host)
}

/// Base URL for analytics endpoints (no /v2 prefix).
pub fn analytics_base(host: &str) -> String {
    format!("https://{}", host)
}

/// Resolve the ~/.lsq-mcp directory (or LSQ_MCP_HOME override).
fn lsq_home() -> Result<PathBuf, LsqError> {
    match std::env::var("LSQ_MCP_HOME") {
        Ok(dir) => Ok(PathBuf::from(dir)),
        Err(_) => dirs::home_dir()
            .ok_or_else(|| LsqError::Auth("Could not determine home directory".into()))
            .map(|h| h.join(".lsq-mcp")),
    }
}

/// Path to the credentials file, honouring LSQ_MCP_HOME override.
pub fn credentials_path() -> Result<PathBuf, LsqError> {
    Ok(lsq_home()?.join("credentials.json"))
}

/// Path to the AES-256 symmetric key used to encrypt credentials.
/// Stored separately from credentials so the two files must both be
/// obtained for the credentials to be readable (defence-in-depth).
/// chmod 0o600 — only the owning user can read it.
pub fn keyfile_path() -> Result<PathBuf, LsqError> {
    Ok(lsq_home()?.join(".key"))
}

/// Directory for large-output files written by lsq-mcp tools.
/// Created on first use. Respects LSQ_MCP_HOME override.
pub fn output_dir() -> Result<PathBuf, LsqError> {
    Ok(lsq_home()?.join("output"))
}

/// Prune the oldest output files when the count exceeds MAX_OUTPUT_FILES.
/// Prevents unbounded disk growth from auto-threshold writes.
/// Best-effort — silently ignores all I/O errors.
pub fn cleanup_output_dir() {
    let dir = match output_dir() {
        Ok(d) => d,
        Err(_) => return,
    };
    if !dir.exists() {
        return;
    }

    let mut entries: Vec<(std::time::SystemTime, PathBuf)> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| {
                let e = e.ok()?;
                let meta = e.metadata().ok()?;
                let modified = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
                Some((modified, e.path()))
            })
            .collect(),
        Err(_) => return,
    };

    if entries.len() <= MAX_OUTPUT_FILES {
        return;
    }

    // Sort oldest-first; delete the excess
    entries.sort_by_key(|(t, _)| *t);
    for (_, path) in entries.iter().take(entries.len() - MAX_OUTPUT_FILES) {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_base_includes_v2() {
        assert_eq!(api_base("api.leadsquared.com"), "https://api.leadsquared.com/v2");
    }

    #[test]
    fn analytics_base_no_v2() {
        assert_eq!(analytics_base("api.leadsquared.com"), "https://api.leadsquared.com");
    }

    #[test]
    fn credentials_path_uses_lsq_mcp_home_when_set() {
        let _guard = crate::ENV_MUTEX.lock().unwrap();
        // SAFETY: ENV_MUTEX serialises all env-var-touching tests across modules.
        unsafe { std::env::set_var("LSQ_MCP_HOME", "/tmp/test-lsq"); }
        let path = credentials_path().unwrap();
        assert_eq!(path, std::path::PathBuf::from("/tmp/test-lsq/credentials.json"));
        unsafe { std::env::remove_var("LSQ_MCP_HOME"); }
    }
}
