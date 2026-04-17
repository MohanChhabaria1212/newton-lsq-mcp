pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod login;
pub mod metadata;
pub mod models;
pub mod server;
pub mod tools;

/// Shared mutex for any test that mutates process-wide env vars (LSQ_MCP_HOME).
/// All tests touching env vars must acquire this lock to avoid races.
#[cfg(test)]
pub(crate) static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
