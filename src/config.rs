use std::path::PathBuf;
use crate::error::LsqError;

pub const DEFAULT_HOST: &str = "api.leadsquared.com";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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

/// Directory for large-output files written by lsq-mcp tools.
/// Created on first use. Respects LSQ_MCP_HOME override.
pub fn output_dir() -> Result<PathBuf, LsqError> {
    Ok(lsq_home()?.join("output"))
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
