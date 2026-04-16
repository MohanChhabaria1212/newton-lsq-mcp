use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::fs::PermissionsExt;

use serde::{Deserialize, Serialize};

use crate::config;
use crate::error::LsqError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub access_key: String,
    pub secret_key: String,
    pub host: String,
    /// Full name of the connected LSQ user — stored at configure time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    /// Email of the connected LSQ user — stored at configure time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
    /// Role of the connected LSQ user — stored at configure time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_role: Option<String>,
}

pub fn load_credentials() -> Result<Option<Credentials>, LsqError> {
    let path = config::credentials_path()?;
    match fs::read_to_string(&path) {
        Ok(contents) => {
            let creds: Credentials = serde_json::from_str(&contents)
                .map_err(|e| LsqError::Auth(format!("Credentials file is malformed: {}", e)))?;
            Ok(Some(creds))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn save_credentials(creds: &Credentials) -> Result<(), LsqError> {
    let path = config::credentials_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    }

    let contents = serde_json::to_string_pretty(creds)?;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

pub fn delete_credentials() -> Result<(), LsqError> {
    let path = config::credentials_path()?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Returns a masked display of the access key (first 4 chars + ****).
pub fn mask_key(key: &str) -> String {
    if key.len() <= 4 {
        return "****".to_string();
    }
    format!("{}****", &key[..4])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialise all tests that mutate LSQ_MCP_HOME to avoid race conditions
    // with other tests in the same binary that also touch that env var.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn mask_key_shows_first_four() {
        assert_eq!(mask_key("abcdefgh"), "abcd****");
    }

    #[test]
    fn mask_key_short_input() {
        assert_eq!(mask_key("ab"), "****");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let _guard = ENV_LOCK.lock().unwrap();
        // SAFETY: single-threaded test binary, no concurrent env access
        unsafe { std::env::set_var("LSQ_MCP_HOME", "/tmp/lsq-mcp-test-auth"); }
        let creds = Credentials {
            access_key: "test_access".into(),
            secret_key: "test_secret".into(),
            host: "api.leadsquared.com".into(),
            user_name: Some("Test User".into()),
            user_email: Some("test@example.com".into()),
            user_role: Some("Admin".into()),
        };
        save_credentials(&creds).unwrap();
        let loaded = load_credentials().unwrap().unwrap();
        assert_eq!(loaded.access_key, "test_access");
        assert_eq!(loaded.host, "api.leadsquared.com");
        delete_credentials().unwrap();
        unsafe { std::env::remove_var("LSQ_MCP_HOME"); }
    }

    #[test]
    fn load_returns_none_when_no_file() {
        let _guard = ENV_LOCK.lock().unwrap();
        // SAFETY: single-threaded test binary, no concurrent env access
        unsafe { std::env::set_var("LSQ_MCP_HOME", "/tmp/lsq-mcp-test-nofile"); }
        let result = load_credentials().unwrap();
        assert!(result.is_none());
        unsafe { std::env::remove_var("LSQ_MCP_HOME"); }
    }
}
