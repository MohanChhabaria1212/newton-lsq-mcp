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

/// Path to the credentials file, honouring LSQ_MCP_HOME override.
pub fn credentials_path() -> Result<PathBuf, LsqError> {
    let dir = match std::env::var("LSQ_MCP_HOME") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => dirs::home_dir()
            .ok_or_else(|| LsqError::Auth("Could not determine home directory".into()))?
            .join(".lsq-mcp"),
    };
    Ok(dir.join("credentials.json"))
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
        std::env::set_var("LSQ_MCP_HOME", "/tmp/test-lsq");
        let path = credentials_path().unwrap();
        assert_eq!(path, std::path::PathBuf::from("/tmp/test-lsq/credentials.json"));
        std::env::remove_var("LSQ_MCP_HOME");
    }
}
