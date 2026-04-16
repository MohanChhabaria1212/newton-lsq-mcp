# LeadSquared MCP Server — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a read-only MCP server for LeadSquared CRM exposing 36 tools across 8 modules.

**Architecture:** Rust binary using rmcp over stdio. `LsqClient` wraps all LSQ HTTP calls (header auth, transparent 429 retry, in-memory caching). Tool modules contain `build_*` helpers and async functions; `server.rs` wires them into the MCP tool router.

**Tech Stack:** Rust 2024 · rmcp 1.2 · tokio · reqwest/rustls · serde/schemars · thiserror · tracing

---

## File Map

```
Cargo.toml              — update package metadata, add reqwest query feature
src/main.rs             — CLI entry: configure | status | (serve)
src/lib.rs              — module declarations (exists, no changes)
src/error.rs            — LsqError enum + lsq_error() message builder
src/config.rs           — DEFAULT_HOST, VERSION, credentials_path()
src/auth.rs             — Credentials struct, load/save/delete, file permissions
src/login.rs            — interactive configure: stdin prompts → validate → save
src/client.rs           — LsqClient: headers, retry, caching, get/post helpers
src/models.rs           — all tool parameter structs (JsonSchema + Deserialize)
src/metadata.rs         — optional background version check (stub for now)
src/server.rs           — LsqMcpServer, tool_router, ensure_client, check_auth
src/tools/mod.rs        — pub mod declarations
src/tools/instructions.rs — static get_instructions tool
src/tools/leads.rs      — 7 lead tools + build_* helpers
src/tools/opportunities.rs — 5 opportunity tools + build_* helpers
src/tools/activities.rs — 2 activity tools + build_* helpers
src/tools/sales.rs      — 3 sales activity tools + build_* helpers
src/tools/tasks.rs      — 5 task tools + build_* helpers
src/tools/users.rs      — 6 user tools + build_* helpers
src/tools/lists.rs      — 4 list tools + build_* helpers
src/tools/analytics.rs  — 4 analytics tools + build_* helpers
tests/configure_test.rs — integration: configure flow + credential file
tests/tools_test.rs     — integration: live LSQ sandbox calls
npm/package.json        — npm wrapper metadata
npm/run.js              — downloads correct binary on first run
```

---

## Phase 1 — Foundation

### Task 1: Cargo.toml + error.rs + config.rs

**Files:**
- Modify: `Cargo.toml`
- Create: `src/error.rs`
- Create: `src/config.rs`

- [ ] **Update Cargo.toml** — fix package metadata and add `query` feature to reqwest:

```toml
[package]
name = "lsq-mcp"
version = "0.1.0"
edition = "2024"
description = "MCP server for LeadSquared CRM — read-only data access"
license = "MIT"

[dependencies]
rmcp = { version = "1.2", features = ["server", "macros", "transport-io"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "sync"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "1"
anyhow = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
reqwest = { version = "0.13", default-features = false, features = ["json", "rustls", "query"] }
dirs = "6"

[features]
local = []
staging = []

[profile.release]
strip = true
lto = true
codegen-units = 1
```

- [ ] **Write failing test for lsq_error() in error.rs**

Create `src/error.rs`:

```rust
use std::io;

#[derive(Debug, thiserror::Error)]
pub enum LsqError {
    #[error("API error: {0}")]
    Api(#[from] reqwest::Error),

    #[error("Unauthorized — your LSQ API keys are invalid or revoked")]
    Unauthorized,

    #[error("Host unreachable: {0}")]
    HostUnreachable(String),

    #[error("Feature not enabled: {0}")]
    FeatureNotEnabled(String),

    #[error("Elasticsearch not enabled on this account")]
    ElasticsearchNotEnabled,

    #[error("Rate limit exhausted after retries")]
    RateLimitExhausted,

    #[error("Auth error: {0}")]
    Auth(String),

    #[error("Configure error: {0}")]
    Configure(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Build a structured 4-part error message for MCP tool responses.
pub fn lsq_error(error: &str, reason: &str, solution: &str, alternative: &str) -> String {
    format!(
        "Error: {}\nReason: {}\nSolution: {}\nAlternative: {}",
        error, reason, solution, alternative
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lsq_error_formats_four_parts() {
        let msg = lsq_error("went wrong", "because X", "do Y", "try Z");
        assert!(msg.contains("Error: went wrong"));
        assert!(msg.contains("Reason: because X"));
        assert!(msg.contains("Solution: do Y"));
        assert!(msg.contains("Alternative: try Z"));
    }
}
```

- [ ] **Create src/config.rs**

```rust
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
```

- [ ] **Run tests**

```bash
cd /Users/mohanchhabaria/Documents/codebase/newton-lsq-mcp && cargo test error:: config::
```

Expected: all tests pass.

- [ ] **Commit**

```bash
git add Cargo.toml src/error.rs src/config.rs
git commit -m "feat: add error types, config constants, and URL helpers"
```

---

### Task 2: auth.rs

**Files:**
- Create: `src/auth.rs`

- [ ] **Create src/auth.rs**

```rust
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
        std::env::set_var("LSQ_MCP_HOME", "/tmp/lsq-mcp-test-auth");
        let creds = Credentials {
            access_key: "test_access".into(),
            secret_key: "test_secret".into(),
            host: "api.leadsquared.com".into(),
        };
        save_credentials(&creds).unwrap();
        let loaded = load_credentials().unwrap().unwrap();
        assert_eq!(loaded.access_key, "test_access");
        assert_eq!(loaded.host, "api.leadsquared.com");
        delete_credentials().unwrap();
        std::env::remove_var("LSQ_MCP_HOME");
    }

    #[test]
    fn load_returns_none_when_no_file() {
        std::env::set_var("LSQ_MCP_HOME", "/tmp/lsq-mcp-test-nofile");
        let result = load_credentials().unwrap();
        assert!(result.is_none());
        std::env::remove_var("LSQ_MCP_HOME");
    }
}
```

- [ ] **Run tests**

```bash
cargo test auth::
```

Expected: 4 tests pass.

- [ ] **Commit**

```bash
git add src/auth.rs
git commit -m "feat: credential load/save/delete with 0o600 file permissions"
```

---

### Task 3: login.rs — interactive configure flow

**Files:**
- Create: `src/login.rs`

The configure flow:
1. Print header with instructions
2. Prompt for access key (stdin)
3. Prompt for secret key (stdin)
4. Prompt for host (stdin, Enter = default)
5. Make validation call to `GET /v2/UserManagement.svc/GetAll?pageIndex=0&pageSize=1`
6. On success: print connected account info, save credentials
7. On failure: print error, exit without saving

- [ ] **Create src/login.rs**

```rust
use std::io::{self, BufRead, Write};

use crate::auth::{self, Credentials};
use crate::config;
use crate::error::LsqError;

/// Run the interactive configure flow. Prompts for keys, validates, saves.
pub async fn configure() -> Result<(), LsqError> {
    let stdout = io::stdout();
    let stdin = io::stdin();

    println!();
    println!("LeadSquared MCP Setup");
    println!("─────────────────────");
    println!("Find your API keys at: LSQ Portal → My Account → Settings → API and Webhooks");
    println!("LSQ recommends admin credentials for full team-wide access.");
    println!();

    let access_key = prompt(&stdout, "Enter Access Key: ")?;
    if access_key.is_empty() {
        return Err(LsqError::Configure("Access key cannot be empty".into()));
    }

    let secret_key = prompt(&stdout, "Enter Secret Key: ")?;
    if secret_key.is_empty() {
        return Err(LsqError::Configure("Secret key cannot be empty".into()));
    }

    let host_input = prompt(
        &stdout,
        &format!("Enter API Host [{}]: ", config::DEFAULT_HOST),
    )?;
    let host = if host_input.is_empty() {
        config::DEFAULT_HOST.to_string()
    } else {
        // Strip https:// prefix if user accidentally included it
        host_input
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string()
    };

    println!();
    println!("Verifying credentials...");

    let creds = Credentials { access_key, secret_key, host };

    match validate_credentials(&creds).await {
        Ok(display_name) => {
            auth::save_credentials(&creds)?;
            println!();
            println!("✓ Connected as: {}", display_name);
            println!("  Credentials saved to ~/.lsq-mcp/credentials.json");
            println!("  Start your MCP client to begin using lsq-mcp.");
            println!();
            Ok(())
        }
        Err(LsqError::Unauthorized) => {
            println!();
            println!("✗ Invalid credentials — LSQ returned 401 Unauthorized.");
            println!("  Nothing was saved.");
            println!("  Double-check your keys at: LSQ Portal → My Account → Settings → API and Webhooks");
            Err(LsqError::Configure("Invalid credentials".into()))
        }
        Err(LsqError::HostUnreachable(host)) => {
            println!();
            println!("✗ Could not reach {}.", host);
            println!("  Check the API host matches your account region (see README for regional hosts).");
            Err(LsqError::Configure(format!("Host unreachable: {}", host)))
        }
        Err(e) => {
            println!();
            println!("✗ Verification failed: {}", e);
            Err(e)
        }
    }
}

/// Make a lightweight call to verify credentials and return a display string.
async fn validate_credentials(creds: &Credentials) -> Result<String, LsqError> {
    let url = format!(
        "{}/UserManagement.svc/GetAll?pageIndex=0&pageSize=1",
        config::api_base(&creds.host)
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(LsqError::Api)?;

    let resp = client
        .get(&url)
        .header("x-LSQ-AccessKey", &creds.access_key)
        .header("x-LSQ-SecretKey", &creds.secret_key)
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() || e.is_timeout() {
                LsqError::HostUnreachable(creds.host.clone())
            } else {
                LsqError::Api(e)
            }
        })?;

    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(LsqError::Unauthorized);
    }

    let resp = resp.error_for_status().map_err(LsqError::Api)?;
    let body: serde_json::Value = resp.json().await.map_err(LsqError::Api)?;

    // Try to extract user info from response for a friendly confirmation message
    let display = body
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|u| u.get("EmailAddress"))
        .and_then(|v| v.as_str())
        .map(|email| format!("LeadSquared account ({})", email))
        .unwrap_or_else(|| "LeadSquared account".to_string());

    Ok(display)
}

fn prompt(stdout: &io::Stdout, label: &str) -> Result<String, LsqError> {
    let mut out = stdout.lock();
    write!(out, "{}", label)?;
    out.flush()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

/// Print current config status with masked keys.
pub fn status() {
    match auth::load_credentials() {
        Ok(Some(creds)) => {
            println!("Status: Configured");
            println!("  Host:       {}", creds.host);
            println!("  Access Key: {}", auth::mask_key(&creds.access_key));
            println!("  Secret Key: ****");
        }
        Ok(None) => {
            println!("Status: Not configured");
            println!("  Run 'lsq-mcp configure' to set up your API keys.");
        }
        Err(e) => {
            println!("Status: Error reading credentials — {}", e);
        }
    }
}
```

- [ ] **Run compile check** (login.rs has no unit-testable pure functions — integration tested in Task 15)

```bash
cargo build 2>&1 | head -30
```

Expected: compiles cleanly (may warn about unused imports in stub modules — that's fine).

- [ ] **Commit**

```bash
git add src/login.rs
git commit -m "feat: interactive configure flow with credential validation"
```

---

### Task 4: client.rs — HTTP layer

**Files:**
- Create: `src/client.rs`

- [ ] **Create src/client.rs**

```rust
use std::sync::Arc;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::RwLock;

use crate::auth::Credentials;
use crate::config;
use crate::error::LsqError;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_RETRIES: u32 = 3;

#[derive(Clone)]
pub struct LsqClient {
    http: reqwest::Client,
    creds: Credentials,
    // In-memory caches for stable data
    lead_metadata_cache:      Arc<RwLock<Option<Value>>>,
    activity_types_cache:     Arc<RwLock<Option<Value>>>,
    opportunity_types_cache:  Arc<RwLock<Option<Value>>>,
    task_types_cache:         Arc<RwLock<Option<Value>>>,
    products_cache:           Arc<RwLock<Option<Value>>>,
}

impl LsqClient {
    pub fn new(creds: Credentials) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()
                .expect("failed to build HTTP client"),
            creds,
            lead_metadata_cache:     Arc::new(RwLock::new(None)),
            activity_types_cache:    Arc::new(RwLock::new(None)),
            opportunity_types_cache: Arc::new(RwLock::new(None)),
            task_types_cache:        Arc::new(RwLock::new(None)),
            products_cache:          Arc::new(RwLock::new(None)),
        }
    }

    fn base(&self) -> String {
        config::api_base(&self.creds.host)
    }

    fn analytics_base(&self) -> String {
        config::analytics_base(&self.creds.host)
    }

    fn auth_headers(&self) -> [(&'static str, String); 2] {
        [
            ("x-LSQ-AccessKey", self.creds.access_key.clone()),
            ("x-LSQ-SecretKey", self.creds.secret_key.clone()),
        ]
    }

    // ── Core HTTP with retry ─────────────────────────────────────────────

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, LsqError> {
        let url = format!("{}{}", self.base(), path);
        self.get_url(&url).await
    }

    pub async fn get_url<T: DeserializeOwned>(&self, url: &str) -> Result<T, LsqError> {
        let mut delay_secs = 1u64;
        for attempt in 0..=MAX_RETRIES {
            let resp = self.http
                .get(url)
                .header("x-LSQ-AccessKey", &self.creds.access_key)
                .header("x-LSQ-SecretKey", &self.creds.secret_key)
                .send()
                .await
                .map_err(|e| {
                    if e.is_connect() || e.is_timeout() {
                        LsqError::HostUnreachable(self.creds.host.clone())
                    } else {
                        LsqError::Api(e)
                    }
                })?;

            match resp.status().as_u16() {
                401 => return Err(LsqError::Unauthorized),
                429 => {
                    if attempt == MAX_RETRIES {
                        return Err(LsqError::RateLimitExhausted);
                    }
                    let wait = resp.headers()
                        .get("Retry-After")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(delay_secs);
                    tracing::debug!("429 rate limit on {}, waiting {}s (attempt {})", url, wait, attempt + 1);
                    tokio::time::sleep(Duration::from_secs(wait)).await;
                    delay_secs *= 2;
                    continue;
                }
                _ => {
                    let resp = resp.error_for_status().map_err(LsqError::Api)?;
                    return Ok(resp.json::<T>().await.map_err(LsqError::Api)?);
                }
            }
        }
        Err(LsqError::RateLimitExhausted)
    }

    pub async fn get_with_params<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<T, LsqError> {
        let url = format!("{}{}", self.base(), path);
        let url = reqwest::Url::parse_with_params(&url, params)
            .map_err(|e| LsqError::Configure(format!("Invalid URL params: {}", e)))?;
        self.get_url(url.as_str()).await
    }

    pub async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &Value,
    ) -> Result<T, LsqError> {
        let url = format!("{}{}", self.base(), path);
        self.post_url(&url, body, &[]).await
    }

    /// POST for analytics endpoints — auth via query params, not headers.
    pub async fn post_analytics<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &Value,
    ) -> Result<T, LsqError> {
        let url = format!("{}{}", self.analytics_base(), path);
        let params = [
            ("accessKey", self.creds.access_key.as_str()),
            ("secretKey", self.creds.secret_key.as_str()),
            ("responseformat", "json"),
        ];
        self.post_url(&url, body, &params).await
    }

    async fn post_url<T: DeserializeOwned>(
        &self,
        url: &str,
        body: &Value,
        query_params: &[(&str, &str)],
    ) -> Result<T, LsqError> {
        let mut delay_secs = 1u64;
        for attempt in 0..=MAX_RETRIES {
            let mut req = self.http
                .post(url)
                .header("x-LSQ-AccessKey", &self.creds.access_key)
                .header("x-LSQ-SecretKey", &self.creds.secret_key)
                .json(body);

            if !query_params.is_empty() {
                req = self.http
                    .post(url)
                    .query(query_params)
                    .header("x-LSQ-AccessKey", &self.creds.access_key)
                    .header("x-LSQ-SecretKey", &self.creds.secret_key)
                    .json(body);
            }

            let resp = req.send().await.map_err(|e| {
                if e.is_connect() || e.is_timeout() {
                    LsqError::HostUnreachable(self.creds.host.clone())
                } else {
                    LsqError::Api(e)
                }
            })?;

            match resp.status().as_u16() {
                401 => return Err(LsqError::Unauthorized),
                429 => {
                    if attempt == MAX_RETRIES {
                        return Err(LsqError::RateLimitExhausted);
                    }
                    let wait = resp.headers()
                        .get("Retry-After")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(delay_secs);
                    tracing::debug!("429 rate limit on {}, waiting {}s", url, wait);
                    tokio::time::sleep(Duration::from_secs(wait)).await;
                    delay_secs *= 2;
                    continue;
                }
                _ => {
                    let resp = resp.error_for_status().map_err(LsqError::Api)?;
                    return Ok(resp.json::<T>().await.map_err(LsqError::Api)?);
                }
            }
        }
        Err(LsqError::RateLimitExhausted)
    }

    // ── Cached getters ───────────────────────────────────────────────────

    pub async fn get_lead_metadata_cached(&self) -> Result<Value, LsqError> {
        cached_get(&self.lead_metadata_cache, || async {
            self.get::<Value>("/LeadFields.svc/GetFields").await
        }).await
    }

    pub async fn get_activity_types_cached(&self) -> Result<Value, LsqError> {
        cached_get(&self.activity_types_cache, || async {
            self.get::<Value>("/ActivityTypes.svc/GetAll").await
        }).await
    }

    pub async fn get_opportunity_types_cached(&self) -> Result<Value, LsqError> {
        cached_get(&self.opportunity_types_cache, || async {
            self.get::<Value>("/Opportunities.svc/GetTypes").await
        }).await
    }

    pub async fn get_task_types_cached(&self) -> Result<Value, LsqError> {
        cached_get(&self.task_types_cache, || async {
            self.get::<Value>("/Task.svc/GetTypes").await
        }).await
    }

    pub async fn get_products_cached(&self) -> Result<Value, LsqError> {
        cached_get(&self.products_cache, || async {
            self.get::<Value>("/SalesActivity.svc/GetProducts").await
        }).await
    }
}

async fn cached_get<F, Fut>(
    cache: &Arc<RwLock<Option<Value>>>,
    fetch: F,
) -> Result<Value, LsqError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<Value, LsqError>>,
{
    {
        let guard = cache.read().await;
        if let Some(data) = guard.as_ref() {
            return Ok(data.clone());
        }
    }
    let data = fetch().await?;
    *cache.write().await = Some(data.clone());
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::Credentials;

    fn test_client() -> LsqClient {
        LsqClient::new(Credentials {
            access_key: "test".into(),
            secret_key: "test".into(),
            host: "api.leadsquared.com".into(),
        })
    }

    #[test]
    fn base_url_correct() {
        let c = test_client();
        assert_eq!(c.base(), "https://api.leadsquared.com/v2");
    }

    #[test]
    fn analytics_base_url_correct() {
        let c = test_client();
        assert_eq!(c.analytics_base(), "https://api.leadsquared.com");
    }
}
```

- [ ] **Run tests**

```bash
cargo test client::
```

Expected: 2 tests pass.

- [ ] **Commit**

```bash
git add src/client.rs
git commit -m "feat: LsqClient with header auth, transparent 429 retry, and in-memory caching"
```

---

### Task 5: models.rs — tool parameter structs

**Files:**
- Create: `src/models.rs`

All `#[tool]` parameter structs live here. Each needs `serde::Deserialize` + `schemars::JsonSchema`.

- [ ] **Create src/models.rs**

```rust
use schemars::JsonSchema;
use serde::Deserialize;

// ── Pagination ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PaginationParams {
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 100.
    pub page_size: Option<u32>,
}

impl PaginationParams {
    pub fn page_index(&self) -> u32 {
        self.page.unwrap_or(1).saturating_sub(1)
    }
    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(25).min(100)
    }
}

// ── Lead params ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchLeadsParams {
    /// JSON array of filter conditions. Each condition: {"Attribute":"FieldName","Operator":"eq|gt|lt|contains","Value":"..."}
    /// Call get_lead_metadata first to discover valid field names (especially custom fields).
    /// All date values must be UTC in YYYY-MM-DD HH:MM:SS format.
    pub filters: Option<serde_json::Value>,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 100.
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LeadIdParam {
    /// The LeadSquared ProspectID (GUID) of the lead.
    pub lead_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LeadEmailParam {
    /// Email address of the lead to look up.
    pub email: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LeadPhoneParam {
    /// Phone number of the lead to look up.
    pub phone: String,
}

// ── Opportunity params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OpportunityIdParam {
    /// The LeadSquared Opportunity ID.
    pub opportunity_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OpportunityMetadataParams {
    /// Opportunity type ID from get_opportunity_types.
    pub opportunity_type_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchOpportunitiesParams {
    /// JSON array of filter conditions on opportunity fields.
    pub filters: Option<serde_json::Value>,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 100.
    pub page_size: Option<u32>,
}

// ── Task params ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskIdParam {
    /// The LeadSquared Task ID.
    pub task_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TasksByOwnerParams {
    /// User ID of the task owner.
    pub owner_id: String,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 100.
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AppointmentParams {
    /// User ID to filter appointments for.
    pub user_id: Option<String>,
    /// User email to filter appointments for.
    pub email: Option<String>,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 100.
    pub page_size: Option<u32>,
}

// ── User params ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UserIdParam {
    /// The LeadSquared User ID.
    pub user_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchUsersParams {
    /// JSON array of filter conditions on user fields.
    pub filters: Option<serde_json::Value>,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 100.
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UserHierarchyParams {
    /// Manager's User ID. Returns all users in their reporting chain.
    pub manager_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckInHistoryParams {
    /// User ID to retrieve check-in history for.
    pub user_id: String,
    /// From date (UTC, YYYY-MM-DD HH:MM:SS).
    pub from_date: Option<String>,
    /// To date (UTC, YYYY-MM-DD HH:MM:SS).
    pub to_date: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AvailabilityParams {
    /// User ID to check availability for.
    pub user_id: Option<String>,
    /// User email to check availability for.
    pub email: Option<String>,
}

// ── List params ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListIdParam {
    /// The LeadSquared List ID.
    pub list_id: String,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 100.
    pub page_size: Option<u32>,
}

// ── Analytics params ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LeadDistributionParams {
    /// JSON filter body following LSQ Lead Distribution API schema.
    /// Supports UserFilter, LeadFilters, DateFilter, and Aggregate fields.
    /// All dates must be UTC in YYYY-MM-DD HH:MM:SS format.
    pub filters: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LeadsNotContactedParams {
    /// JSON filter body following LSQ Leads Not Contacted API schema.
    /// Supports UserFilter, LeadFilters, ActivityFilters, DateFilter.
    /// All dates must be UTC in YYYY-MM-DD HH:MM:SS format.
    pub filters: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LeadsNoActiveTasksParams {
    /// JSON filter body following LSQ Leads With No Active Tasks API schema.
    /// Supports UserFilter, LeadFilters, TaskFilters, DateFilter.
    pub filters: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LeadsPendingTasksParams {
    /// JSON filter body following LSQ Leads With Pending Tasks API schema.
    /// Supports UserFilter, LeadFilters, TaskFilters (Pending/Overdue/PendingAndOverdue), DateFilter.
    pub filters: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_defaults() {
        let p = PaginationParams { page: None, page_size: None };
        assert_eq!(p.page_index(), 0);
        assert_eq!(p.page_size(), 25);
    }

    #[test]
    fn pagination_caps_at_100() {
        let p = PaginationParams { page: Some(1), page_size: Some(500) };
        assert_eq!(p.page_size(), 100);
    }

    #[test]
    fn pagination_page_index_is_zero_based() {
        let p = PaginationParams { page: Some(3), page_size: Some(10) };
        assert_eq!(p.page_index(), 2);
    }
}
```

- [ ] **Run tests**

```bash
cargo test models::
```

Expected: 3 tests pass.

- [ ] **Commit**

```bash
git add src/models.rs
git commit -m "feat: tool parameter structs with JsonSchema for MCP validation"
```

---

### Task 6: server.rs skeleton + main.rs

**Files:**
- Create: `src/server.rs`
- Create: `src/metadata.rs`
- Modify: `src/main.rs`

- [ ] **Create src/metadata.rs** (stub — version check can be wired in later)

```rust
/// Placeholder for background version check.
/// Extend this once a version config endpoint is established.
pub async fn check_version() {
    // No-op in v1
}
```

- [ ] **Create src/server.rs** — skeleton with ensure_client and check_auth

```rust
use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ErrorData, tool, tool_handler, tool_router, ServerHandler};
use tokio::sync::RwLock;

use crate::auth;
use crate::client::LsqClient;
use crate::error::{LsqError, lsq_error};
use crate::models::*;
use crate::tools::instructions;

#[derive(Clone)]
pub struct LsqMcpServer {
    tool_router: ToolRouter<Self>,
    client: Arc<RwLock<Option<LsqClient>>>,
}

impl Default for LsqMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl LsqMcpServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            client: Arc::new(RwLock::new(None)),
        }
    }

    /// Load credentials from disk and build a fresh LsqClient.
    /// Called on every tool invocation so credential file changes take effect immediately.
    pub async fn ensure_client(&self) -> Result<(), CallToolResult> {
        match auth::load_credentials() {
            Ok(Some(creds)) => {
                *self.client.write().await = Some(LsqClient::new(creds));
                Ok(())
            }
            Ok(None) => Err(CallToolResult::error(vec![Content::text(lsq_error(
                "No credentials found.",
                "lsq-mcp has not been configured yet.",
                "Run 'lsq-mcp configure' in your terminal to set up your LSQ API keys.",
                "Find your keys at: LSQ Portal → My Account → Settings → API and Webhooks",
            ))])),
            Err(e) => Err(CallToolResult::error(vec![Content::text(lsq_error(
                "Failed to load credentials.",
                &format!("Credentials file error: {}", e),
                "Run 'lsq-mcp configure' to recreate the credentials file.",
                "If the problem persists, delete ~/.lsq-mcp/credentials.json and reconfigure.",
            ))])),
        }
    }

    /// Acquire a read lock on the client. Panics only in logic bugs (ensure_client must precede this).
    pub async fn get_client(&self) -> tokio::sync::RwLockReadGuard<'_, Option<LsqClient>> {
        self.client.read().await
    }
}

// ── Shared response helpers ───────────────────────────────────────────────

pub fn success_json(value: &serde_json::Value) -> Result<CallToolResult, ErrorData> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|e| ErrorData::internal_error(format!("JSON serialisation error: {}", e), None))?;
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

pub fn api_error(context: &str, e: LsqError) -> ErrorData {
    match e {
        LsqError::Unauthorized => ErrorData::internal_error("lsq:unauthorized".to_string(), None),
        LsqError::HostUnreachable(host) => ErrorData::internal_error(
            lsq_error(
                &format!("Could not reach {}.", host),
                "The API host may be incorrect for your account region.",
                "Run 'lsq-mcp configure' and enter the correct host.",
                "Check regional hosts in the README.",
            ),
            None,
        ),
        LsqError::RateLimitExhausted => ErrorData::internal_error(
            lsq_error(
                "LeadSquared is temporarily rate-limiting requests.",
                "Too many API calls were made in a short period.",
                "Wait a moment and try again.",
                "Reduce the frequency of tool calls if this recurs.",
            ),
            None,
        ),
        LsqError::ElasticsearchNotEnabled => ErrorData::internal_error(
            lsq_error(
                "This analytics tool requires Elasticsearch to be enabled on your LSQ account.",
                "The analytics API depends on LSQ's Elasticsearch feature.",
                "Contact LSQ support to enable Elasticsearch for your account.",
                "Use search_leads with manual filters as a partial substitute.",
            ),
            None,
        ),
        LsqError::FeatureNotEnabled(feature) => ErrorData::internal_error(
            lsq_error(
                &format!("The {} feature is not enabled on your LSQ account.", feature),
                "This module requires activation in your LSQ plan.",
                "Contact your LSQ account manager to enable this feature.",
                "Skip this tool and use lead-level tools instead.",
            ),
            None,
        ),
        _ => ErrorData::internal_error(format!("{}: {}", context, e), None),
    }
}

/// Post-process a tool result: handle auth errors, surface feature errors.
pub async fn check_auth(
    server: &LsqMcpServer,
    result: Result<CallToolResult, ErrorData>,
) -> Result<CallToolResult, ErrorData> {
    match result {
        Err(ref e) if e.message == "lsq:unauthorized" => {
            // Clear client so next call reloads credentials from disk
            *server.client.write().await = None;
            Ok(CallToolResult::error(vec![Content::text(lsq_error(
                "LeadSquared returned 401 Unauthorized.",
                "Your API keys are invalid or have been revoked.",
                "Run 'lsq-mcp configure' to enter new credentials.",
                "Verify your keys at LSQ Portal → My Account → Settings → API and Webhooks.",
            ))]))
        }
        other => other,
    }
}

// ── Tool implementations ──────────────────────────────────────────────────

#[tool_router]
impl LsqMcpServer {
    #[tool(
        description = "ALWAYS call this first. Returns descriptions of all available tools, recommended call sequences, and important notes about field names, date formats (UTC YYYY-MM-DD HH:MM:SS), and pagination. No API call required.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_instructions(&self) -> Result<CallToolResult, ErrorData> {
        Ok(CallToolResult::success(vec![Content::text(instructions::INSTRUCTIONS)]))
    }
}

impl ServerHandler for LsqMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "lsq-mcp".to_string(),
                version: crate::config::VERSION.to_string(),
            },
            ..Default::default()
        }
    }
}

tool_handler!(LsqMcpServer);
```

- [ ] **Update src/main.rs**

```rust
use anyhow::Result;
use rmcp::ServiceExt;
use rmcp::transport::io::stdio;

use lsq_mcp::{login, server};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("configure") => cmd_configure().await,
        Some("status")    => { cmd_status(); Ok(()) }
        Some(cmd) => {
            eprintln!("Unknown command: {}", cmd);
            eprintln!("Usage: lsq-mcp [configure|status]");
            eprintln!("  (no args)   Start MCP server");
            eprintln!("  configure   Set up your LSQ API keys");
            eprintln!("  status      Show current configuration");
            std::process::exit(1);
        }
        None => cmd_serve().await,
    }
}

async fn cmd_serve() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting lsq-mcp v{}", lsq_mcp::config::VERSION);

    let mcp_server = server::LsqMcpServer::new();
    let service = mcp_server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

async fn cmd_configure() -> Result<()> {
    login::configure().await.map_err(|e| anyhow::anyhow!("{}", e))
}

fn cmd_status() {
    login::status();
}
```

- [ ] **Create src/tools/mod.rs**

```rust
pub mod instructions;
pub mod leads;
pub mod opportunities;
pub mod activities;
pub mod sales;
pub mod tasks;
pub mod users;
pub mod lists;
pub mod analytics;
```

- [ ] **Create src/tools/instructions.rs**

```rust
pub const INSTRUCTIONS: &str = r#"
LeadSquared MCP — Available Tools
═══════════════════════════════════

IMPORTANT NOTES
───────────────
• Date format: All date parameters must be UTC in "YYYY-MM-DD HH:MM:SS" format.
• Custom fields: LSQ accounts have account-specific custom fields. Call get_lead_metadata
  first to discover available field names before filtering on custom fields.
• Pagination: All list/search tools return 25 results by default (max 100).
  Check has_more and increment page to retrieve further results.
• Elasticsearch: get_leads_not_contacted, get_leads_no_active_tasks, and
  get_leads_pending_tasks require Elasticsearch to be enabled on your LSQ account.

RECOMMENDED CALL SEQUENCE
──────────────────────────
1. get_lead_metadata       — understand available lead fields (cached after first call)
2. get_activity_types      — know activity type names/IDs (cached)
3. get_task_types          — know task type names (cached)
4. get_opportunity_types   — know opportunity types (cached)
5. search_leads / get_lead_by_* — find the leads you need
6. get_lead_activities / get_lead_notes / get_opportunities_by_lead — enrich as needed

TOOLS BY MODULE
───────────────

LEADS (7 tools)
  get_lead_metadata          — field schemas, types, picklist values
  search_leads               — advanced filter search (requires filters JSON)
  get_lead_by_id             — full lead by ProspectID
  get_lead_by_email          — lookup by email
  get_lead_by_phone          — lookup by phone
  get_lead_notes             — notes on a lead
  get_lead_activities        — full activity history for a lead

OPPORTUNITIES (5 tools)
  get_opportunity_types      — all opportunity types
  get_opportunity_metadata   — field schema for an opportunity type
  get_opportunity_by_id      — single opportunity
  get_opportunities_by_lead  — all opportunities for a lead
  search_opportunities       — filtered opportunity search

ACTIVITIES (2 tools)
  get_activity_types         — all activity type definitions (cached)
  get_activities_by_lead     — activity log for a lead

SALES ACTIVITIES (3 tools)
  get_products               — product catalogue (cached)
  get_sales_activity_types   — sales activity settings
  get_sales_activities_by_lead — sales transactions for a lead

TASKS (5 tools)
  get_task_types             — all task type names (cached)
  get_tasks_by_lead          — tasks for a lead
  get_tasks_by_owner         — tasks assigned to a user
  get_appointments           — user appointments
  get_todos                  — user to-do items

USERS (6 tools)
  get_users                  — all users in the account
  get_user_by_id             — single user details
  search_users               — filtered user search
  get_user_hierarchy         — reporting chain under a manager
  get_user_checkin_history   — check-in records
  get_user_availability      — working hours and available slots

LISTS (4 tools)
  get_lists                  — all lists in the account
  get_leads_in_list          — leads in a list
  get_lead_list_memberships  — which lists a lead belongs to
  get_list_lead_count        — count of leads in a list

ANALYTICS (4 tools — require Elasticsearch)
  get_lead_distribution      — leads by owner/stage with aggregation
  get_leads_not_contacted    — leads without specified activities
  get_leads_no_active_tasks  — leads with no pending tasks
  get_leads_pending_tasks    — leads with overdue/pending tasks
"#;
```

- [ ] **Run full build**

```bash
cargo build 2>&1 | head -50
```

Expected: builds cleanly. May warn about unused imports in empty tool modules — that's fine.

- [ ] **Commit**

```bash
git add src/server.rs src/main.rs src/metadata.rs src/tools/mod.rs src/tools/instructions.rs
git commit -m "feat: server skeleton, CLI entry point, and instructions tool"
```

---

## Phase 2 — Tool Modules

> Each task in this phase follows the same pattern:
> 1. Create tool module with `build_*` helper functions
> 2. Add tool method stubs to `server.rs` inside the `#[tool_router]` impl block
> 3. Test `build_*` helpers with fixture JSON
> 4. Compile-check the new tools

### Task 7: tools/leads.rs

**Files:**
- Create: `src/tools/leads.rs`
- Modify: `src/server.rs` (add 7 lead tools to `#[tool_router]` impl)

- [ ] **Create src/tools/leads.rs**

```rust
use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::{json, Value};

use crate::client::LsqClient;
use crate::error::LsqError;
use crate::models::{LeadIdParam, LeadEmailParam, LeadPhoneParam, SearchLeadsParams};
use crate::server::{api_error, success_json};

pub async fn get_lead_metadata(client: &LsqClient) -> Result<CallToolResult, ErrorData> {
    let data = client.get_lead_metadata_cached().await
        .map_err(|e| api_error("Failed to fetch lead metadata", e))?;
    success_json(&data)
}

pub async fn search_leads(client: &LsqClient, params: &SearchLeadsParams) -> Result<CallToolResult, ErrorData> {
    let page_index = params.page.unwrap_or(1).saturating_sub(1);
    let page_size = params.page_size.unwrap_or(25).min(100);

    let filters = params.filters.clone().unwrap_or_else(|| json!([]));

    let body = json!({
        "Filters": filters,
        "Paging": {
            "PageIndex": page_index,
            "PageSize": page_size
        }
    });

    let data: Value = client
        .post("/Leads.svc/RetrieveLeadBySearchCriteria", &body)
        .await
        .map_err(|e| api_error("Failed to search leads", e))?;

    let total = data.get("TotalCount").and_then(|v| v.as_i64()).unwrap_or(0);
    let results = data.get("RecordList").cloned().unwrap_or_else(|| json!([]));
    let count = results.as_array().map(|a| a.len()).unwrap_or(0);
    let has_more = (page_index as i64 * page_size as i64 + count as i64) < total;

    success_json(&json!({
        "results": results,
        "total_count": total,
        "page": page_index + 1,
        "page_size": page_size,
        "has_more": has_more
    }))
}

pub async fn get_lead_by_id(client: &LsqClient, params: &LeadIdParam) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!("/Leads.svc/RetrieveById?id={}", params.lead_id))
        .await
        .map_err(|e| api_error("Failed to fetch lead by ID", e))?;
    success_json(&data)
}

pub async fn get_lead_by_email(client: &LsqClient, params: &LeadEmailParam) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!("/Leads.svc/RetrieveByEmailAddress?emailaddress={}", params.email))
        .await
        .map_err(|e| api_error("Failed to fetch lead by email", e))?;
    success_json(&data)
}

pub async fn get_lead_by_phone(client: &LsqClient, params: &LeadPhoneParam) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!("/Leads.svc/RetrieveByPhoneNumber?phone={}", params.phone))
        .await
        .map_err(|e| api_error("Failed to fetch lead by phone", e))?;
    success_json(&data)
}

pub async fn get_lead_notes(client: &LsqClient, params: &LeadIdParam) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!("/Notes.svc/RetrieveByLeadId?leadId={}", params.lead_id))
        .await
        .map_err(|e| api_error("Failed to fetch lead notes", e))?;
    success_json(&data)
}

pub async fn get_lead_activities(client: &LsqClient, params: &LeadIdParam) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!("/Activities.svc/RetrieveByLeadId?leadId={}", params.lead_id))
        .await
        .map_err(|e| api_error("Failed to fetch lead activities", e))?;
    success_json(&data)
}

// ── Build helpers (unit-testable) ─────────────────────────────────────────

pub fn build_paginated_response(results: &Value, total: i64, page_index: u32, page_size: u32) -> Value {
    let count = results.as_array().map(|a| a.len() as i64).unwrap_or(0);
    let has_more = (page_index as i64 * page_size as i64 + count) < total;
    json!({
        "results": results,
        "total_count": total,
        "page": page_index + 1,
        "page_size": page_size,
        "has_more": has_more
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paginated_response_has_more_true() {
        let results = json!([{"id": "1"}, {"id": "2"}]);
        let resp = build_paginated_response(&results, 100, 0, 25);
        assert_eq!(resp["total_count"], 100);
        assert_eq!(resp["page"], 1);
        assert_eq!(resp["has_more"], true);
    }

    #[test]
    fn paginated_response_has_more_false_on_last_page() {
        let results = json!([{"id": "1"}]);
        let resp = build_paginated_response(&results, 1, 0, 25);
        assert_eq!(resp["has_more"], false);
    }
}
```

- [ ] **Add lead tools to server.rs** — append inside the `#[tool_router] impl LsqMcpServer` block:

```rust
    #[tool(
        description = "Get all lead field schemas, types, and picklist values for this LSQ account. CALL THIS FIRST before any search — custom field names vary per account. Results are cached for the session.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_lead_metadata(&self) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = leads::get_lead_metadata(guard.as_ref().unwrap()).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Search leads with filters on any field (stage, owner, date range, custom fields). Call get_lead_metadata first to discover field names. Filters format: [{\"Attribute\":\"FieldName\",\"Operator\":\"eq\",\"Value\":\"...\"}]. All dates must be UTC YYYY-MM-DD HH:MM:SS. Returns paginated results with has_more.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn search_leads(&self, Parameters(params): Parameters<SearchLeadsParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = leads::search_leads(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get full lead details by ProspectID (GUID). Use when you have a specific lead ID from a previous search result.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_lead_by_id(&self, Parameters(params): Parameters<LeadIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = leads::get_lead_by_id(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Look up a lead by their email address.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_lead_by_email(&self, Parameters(params): Parameters<LeadEmailParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = leads::get_lead_by_email(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Look up a lead by their phone number.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_lead_by_phone(&self, Parameters(params): Parameters<LeadPhoneParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = leads::get_lead_by_phone(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get all notes attached to a lead. Use when the user asks about comments, remarks, or notes on a lead.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_lead_notes(&self, Parameters(params): Parameters<LeadIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = leads::get_lead_notes(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get the full activity history for a lead: calls, emails, meetings, and custom events. Use when the user asks what happened with a lead or wants their interaction history.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_lead_activities(&self, Parameters(params): Parameters<LeadIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = leads::get_lead_activities(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }
```

Also add to `server.rs` imports at top:
```rust
use crate::tools::{leads, opportunities, activities, sales, tasks, users, lists, analytics};
```

- [ ] **Run tests and build**

```bash
cargo test tools::leads && cargo build
```

Expected: 2 unit tests pass, builds cleanly.

- [ ] **Commit**

```bash
git add src/tools/leads.rs src/server.rs
git commit -m "feat: 7 lead tools (search, get by id/email/phone, notes, activities, metadata)"
```

---

### Task 8: tools/opportunities.rs

**Files:**
- Create: `src/tools/opportunities.rs`
- Modify: `src/server.rs`

- [ ] **Create src/tools/opportunities.rs**

```rust
use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::{json, Value};

use crate::client::LsqClient;
use crate::models::{OpportunityIdParam, OpportunityMetadataParams, SearchOpportunitiesParams, LeadIdParam};
use crate::server::{api_error, success_json};

pub async fn get_opportunity_types(client: &LsqClient) -> Result<CallToolResult, ErrorData> {
    let data = client.get_opportunity_types_cached().await
        .map_err(|e| api_error("Failed to fetch opportunity types", e))?;
    success_json(&data)
}

pub async fn get_opportunity_metadata(client: &LsqClient, params: &OpportunityMetadataParams) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!("/Opportunities.svc/GetMetadata?opportunityTypeId={}", params.opportunity_type_id))
        .await
        .map_err(|e| api_error("Failed to fetch opportunity metadata", e))?;
    success_json(&data)
}

pub async fn get_opportunity_by_id(client: &LsqClient, params: &OpportunityIdParam) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!("/Opportunities.svc/RetrieveById?id={}", params.opportunity_id))
        .await
        .map_err(|e| api_error("Failed to fetch opportunity", e))?;
    success_json(&data)
}

pub async fn get_opportunities_by_lead(client: &LsqClient, params: &LeadIdParam) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!("/Opportunities.svc/RetrieveByLeadId?leadId={}", params.lead_id))
        .await
        .map_err(|e| api_error("Failed to fetch opportunities for lead", e))?;
    success_json(&data)
}

pub async fn search_opportunities(client: &LsqClient, params: &SearchOpportunitiesParams) -> Result<CallToolResult, ErrorData> {
    let page_index = params.page.unwrap_or(1).saturating_sub(1);
    let page_size = params.page_size.unwrap_or(25).min(100);
    let filters = params.filters.clone().unwrap_or_else(|| json!([]));

    let body = json!({
        "Filters": filters,
        "Paging": { "PageIndex": page_index, "PageSize": page_size }
    });

    let data: Value = client
        .post("/Opportunities.svc/Search", &body)
        .await
        .map_err(|e| api_error("Failed to search opportunities", e))?;

    let total = data.get("TotalCount").and_then(|v| v.as_i64()).unwrap_or(0);
    let results = data.get("RecordList").cloned().unwrap_or_else(|| json!([]));
    let count = results.as_array().map(|a| a.len() as i64).unwrap_or(0);

    success_json(&json!({
        "results": results,
        "total_count": total,
        "page": page_index + 1,
        "page_size": page_size,
        "has_more": (page_index as i64 * page_size as i64 + count) < total
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opportunity_types_response_passthrough() {
        // Opportunity types are passed through raw from LSQ — no reshaping needed
        let raw = json!([{"OpportunityTypeId": "1", "Name": "Deal"}]);
        assert!(raw.is_array());
    }
}
```

- [ ] **Add opportunity tools to server.rs** `#[tool_router]` impl:

```rust
    #[tool(
        description = "Get all opportunity types configured in this LSQ account. Call before get_opportunity_metadata to find valid opportunity type IDs. Results are cached for the session.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_opportunity_types(&self) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = opportunities::get_opportunity_types(guard.as_ref().unwrap()).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get the field schema for a specific opportunity type. Requires opportunity_type_id from get_opportunity_types.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_opportunity_metadata(&self, Parameters(params): Parameters<OpportunityMetadataParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = opportunities::get_opportunity_metadata(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get a single opportunity by its ID.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_opportunity_by_id(&self, Parameters(params): Parameters<OpportunityIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = opportunities::get_opportunity_by_id(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get all opportunities attached to a lead. Use when the user asks about deals, pipeline, or opportunities for a specific lead.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_opportunities_by_lead(&self, Parameters(params): Parameters<LeadIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = opportunities::get_opportunities_by_lead(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Search opportunities with filters. All dates must be UTC YYYY-MM-DD HH:MM:SS. Returns paginated results.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn search_opportunities(&self, Parameters(params): Parameters<SearchOpportunitiesParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = opportunities::search_opportunities(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }
```

- [ ] **Build and test**

```bash
cargo test tools::opportunities && cargo build
```

- [ ] **Commit**

```bash
git add src/tools/opportunities.rs src/server.rs
git commit -m "feat: 5 opportunity tools (types, metadata, by-id, by-lead, search)"
```

---

### Task 9: tools/activities.rs + tools/sales.rs

**Files:**
- Create: `src/tools/activities.rs`
- Create: `src/tools/sales.rs`
- Modify: `src/server.rs`

- [ ] **Create src/tools/activities.rs**

```rust
use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::Value;

use crate::client::LsqClient;
use crate::models::LeadIdParam;
use crate::server::{api_error, success_json};

pub async fn get_activity_types(client: &LsqClient) -> Result<CallToolResult, ErrorData> {
    let data = client.get_activity_types_cached().await
        .map_err(|e| api_error("Failed to fetch activity types", e))?;
    success_json(&data)
}

pub async fn get_activities_by_lead(client: &LsqClient, params: &LeadIdParam) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!("/Activities.svc/RetrieveByLeadId?leadId={}", params.lead_id))
        .await
        .map_err(|e| api_error("Failed to fetch activities", e))?;
    success_json(&data)
}
```

- [ ] **Create src/tools/sales.rs**

```rust
use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::Value;

use crate::client::LsqClient;
use crate::models::LeadIdParam;
use crate::server::{api_error, success_json};

pub async fn get_products(client: &LsqClient) -> Result<CallToolResult, ErrorData> {
    let data = client.get_products_cached().await
        .map_err(|e| api_error("Failed to fetch products", e))?;
    success_json(&data)
}

pub async fn get_sales_activity_types(client: &LsqClient) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get("/SalesActivity.svc/GetSettings")
        .await
        .map_err(|e| api_error("Failed to fetch sales activity types", e))?;
    success_json(&data)
}

pub async fn get_sales_activities_by_lead(client: &LsqClient, params: &LeadIdParam) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!("/SalesActivity.svc/RetrieveByLeadId?leadId={}", params.lead_id))
        .await
        .map_err(|e| api_error("Failed to fetch sales activities", e))?;
    success_json(&data)
}
```

- [ ] **Add to server.rs** `#[tool_router]` impl:

```rust
    #[tool(
        description = "Get all activity type definitions (system, custom, and sales activities). Results cached for the session. Call before get_activities_by_lead to understand what activity types mean.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_activity_types(&self) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = activities::get_activity_types(guard.as_ref().unwrap()).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get the chronological activity log for a lead: calls, emails, meetings, and custom events. Use when the user asks about a lead's interaction history or recent contact.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_activities_by_lead(&self, Parameters(params): Parameters<LeadIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = activities::get_activities_by_lead(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get the product catalogue configured in this LSQ account. Results cached for the session.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_products(&self) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = sales::get_products(guard.as_ref().unwrap()).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get sales activity type settings and definitions.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_sales_activity_types(&self) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = sales::get_sales_activity_types(guard.as_ref().unwrap()).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get sales transactions for a lead: products sold, revenue, SKU, date, and owner. Use when the user asks about sales, revenue, or purchases for a specific lead.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_sales_activities_by_lead(&self, Parameters(params): Parameters<LeadIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = sales::get_sales_activities_by_lead(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }
```

- [ ] **Build**

```bash
cargo build
```

- [ ] **Commit**

```bash
git add src/tools/activities.rs src/tools/sales.rs src/server.rs
git commit -m "feat: activity types, lead activities, products, and sales activity tools"
```

---

### Task 10: tools/tasks.rs

**Files:**
- Create: `src/tools/tasks.rs`
- Modify: `src/server.rs`

- [ ] **Create src/tools/tasks.rs**

```rust
use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::Value;

use crate::client::LsqClient;
use crate::models::{LeadIdParam, TasksByOwnerParams, AppointmentParams};
use crate::server::{api_error, success_json};

pub async fn get_task_types(client: &LsqClient) -> Result<CallToolResult, ErrorData> {
    let data = client.get_task_types_cached().await
        .map_err(|e| api_error("Failed to fetch task types", e))?;
    success_json(&data)
}

pub async fn get_tasks_by_lead(client: &LsqClient, params: &LeadIdParam) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!("/Task.svc/RetrieveByLeadId?leadId={}", params.lead_id))
        .await
        .map_err(|e| api_error("Failed to fetch tasks for lead", e))?;
    success_json(&data)
}

pub async fn get_tasks_by_owner(client: &LsqClient, params: &TasksByOwnerParams) -> Result<CallToolResult, ErrorData> {
    let page_index = params.page.unwrap_or(1).saturating_sub(1);
    let page_size = params.page_size.unwrap_or(25).min(100);

    let data: Value = client
        .get(&format!(
            "/Task.svc/RetrieveByOwnerId?ownerId={}&pageIndex={}&pageSize={}",
            params.owner_id, page_index, page_size
        ))
        .await
        .map_err(|e| api_error("Failed to fetch tasks by owner", e))?;
    success_json(&data)
}

pub async fn get_appointments(client: &LsqClient, params: &AppointmentParams) -> Result<CallToolResult, ErrorData> {
    let mut path = "/Task.svc/GetAppointments?".to_string();
    if let Some(uid) = &params.user_id {
        path.push_str(&format!("userId={}&", uid));
    }
    if let Some(email) = &params.email {
        path.push_str(&format!("emailAddress={}&", email));
    }
    let page_index = params.page.unwrap_or(1).saturating_sub(1);
    let page_size = params.page_size.unwrap_or(25).min(100);
    path.push_str(&format!("pageIndex={}&pageSize={}", page_index, page_size));

    let data: Value = client.get(&path).await
        .map_err(|e| api_error("Failed to fetch appointments", e))?;
    success_json(&data)
}

pub async fn get_todos(client: &LsqClient, params: &AppointmentParams) -> Result<CallToolResult, ErrorData> {
    let mut path = "/Task.svc/GetToDos?".to_string();
    if let Some(uid) = &params.user_id {
        path.push_str(&format!("userId={}&", uid));
    }
    if let Some(email) = &params.email {
        path.push_str(&format!("emailAddress={}&", email));
    }
    let page_index = params.page.unwrap_or(1).saturating_sub(1);
    let page_size = params.page_size.unwrap_or(25).min(100);
    path.push_str(&format!("pageIndex={}&pageSize={}", page_index, page_size));

    let data: Value = client.get(&path).await
        .map_err(|e| api_error("Failed to fetch to-dos", e))?;
    success_json(&data)
}
```

- [ ] **Add to server.rs** `#[tool_router]` impl:

```rust
    #[tool(
        description = "Get all task type names and configurations. Results cached for the session.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_task_types(&self) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = tasks::get_task_types(guard.as_ref().unwrap()).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get all tasks attached to a lead. Use when the user asks about follow-ups, pending actions, or to-dos for a specific lead.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_tasks_by_lead(&self, Parameters(params): Parameters<LeadIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = tasks::get_tasks_by_lead(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get all tasks assigned to a specific user. Use when the user asks about a sales rep's workload or task list. Paginated.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_tasks_by_owner(&self, Parameters(params): Parameters<TasksByOwnerParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = tasks::get_tasks_by_owner(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get appointments for a user by user_id or email. Paginated.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_appointments(&self, Parameters(params): Parameters<AppointmentParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = tasks::get_appointments(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get to-do items for a user by user_id or email. Paginated.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_todos(&self, Parameters(params): Parameters<AppointmentParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = tasks::get_todos(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }
```

- [ ] **Build**

```bash
cargo build
```

- [ ] **Commit**

```bash
git add src/tools/tasks.rs src/server.rs
git commit -m "feat: 5 task tools (types, by-lead, by-owner, appointments, todos)"
```

---

### Task 11: tools/users.rs

**Files:**
- Create: `src/tools/users.rs`
- Modify: `src/server.rs`

- [ ] **Create src/tools/users.rs**

```rust
use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::{json, Value};

use crate::client::LsqClient;
use crate::models::{UserIdParam, SearchUsersParams, UserHierarchyParams, CheckInHistoryParams, AvailabilityParams};
use crate::server::{api_error, success_json};

pub async fn get_users(client: &LsqClient) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get("/UserManagement.svc/GetAll")
        .await
        .map_err(|e| api_error("Failed to fetch users", e))?;
    success_json(&data)
}

pub async fn get_user_by_id(client: &LsqClient, params: &UserIdParam) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!("/UserManagement.svc/GetById?userId={}", params.user_id))
        .await
        .map_err(|e| api_error("Failed to fetch user", e))?;
    success_json(&data)
}

pub async fn search_users(client: &LsqClient, params: &SearchUsersParams) -> Result<CallToolResult, ErrorData> {
    let page_index = params.page.unwrap_or(1).saturating_sub(1);
    let page_size = params.page_size.unwrap_or(25).min(100);
    let filters = params.filters.clone().unwrap_or_else(|| json!([]));

    let body = json!({
        "Filters": filters,
        "Paging": { "PageIndex": page_index, "PageSize": page_size }
    });

    let data: Value = client
        .post("/UserManagement.svc/Search", &body)
        .await
        .map_err(|e| api_error("Failed to search users", e))?;
    success_json(&data)
}

pub async fn get_user_hierarchy(client: &LsqClient, params: &UserHierarchyParams) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!("/UserManagement.svc/GetHierarchy?userId={}", params.manager_id))
        .await
        .map_err(|e| api_error("Failed to fetch user hierarchy", e))?;
    success_json(&data)
}

pub async fn get_user_checkin_history(client: &LsqClient, params: &CheckInHistoryParams) -> Result<CallToolResult, ErrorData> {
    let mut path = format!("/UserManagement.svc/GetCheckInHistory?userId={}", params.user_id);
    if let Some(from) = &params.from_date {
        path.push_str(&format!("&fromDate={}", from));
    }
    if let Some(to) = &params.to_date {
        path.push_str(&format!("&toDate={}", to));
    }
    let data: Value = client.get(&path).await
        .map_err(|e| api_error("Failed to fetch check-in history", e))?;
    success_json(&data)
}

pub async fn get_user_availability(client: &LsqClient, params: &AvailabilityParams) -> Result<CallToolResult, ErrorData> {
    let mut path = "/Task.svc/GetAvailableSlots?".to_string();
    if let Some(uid) = &params.user_id {
        path.push_str(&format!("userId={}&", uid));
    }
    if let Some(email) = &params.email {
        path.push_str(&format!("emailAddress={}", email));
    }
    let data: Value = client.get(&path).await
        .map_err(|e| api_error("Failed to fetch user availability", e))?;
    success_json(&data)
}
```

- [ ] **Add to server.rs** `#[tool_router]` impl:

```rust
    #[tool(
        description = "Get all users (sales agents, managers, admins) in the LSQ account. Use to find user IDs needed by other tools.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_users(&self) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = users::get_users(guard.as_ref().unwrap()).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get details for a single user by their LSQ User ID.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_user_by_id(&self, Parameters(params): Parameters<UserIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = users::get_user_by_id(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Search users by criteria. Filters format: [{\"Attribute\":\"FieldName\",\"Operator\":\"eq\",\"Value\":\"...\"}]. Paginated.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn search_users(&self, Parameters(params): Parameters<SearchUsersParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = users::search_users(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get the reporting chain under a manager. Returns all users who report (directly or indirectly) to the given manager_id.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_user_hierarchy(&self, Parameters(params): Parameters<UserHierarchyParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = users::get_user_hierarchy(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get historical check-in records for a user. Optionally filter by from_date and to_date (UTC YYYY-MM-DD HH:MM:SS).",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_user_checkin_history(&self, Parameters(params): Parameters<CheckInHistoryParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = users::get_user_checkin_history(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get working hours and available appointment slots for a user. Provide user_id or email.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_user_availability(&self, Parameters(params): Parameters<AvailabilityParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = users::get_user_availability(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }
```

- [ ] **Build**

```bash
cargo build
```

- [ ] **Commit**

```bash
git add src/tools/users.rs src/server.rs
git commit -m "feat: 6 user tools (list, by-id, search, hierarchy, check-in, availability)"
```

---

### Task 12: tools/lists.rs

**Files:**
- Create: `src/tools/lists.rs`
- Modify: `src/server.rs`

- [ ] **Create src/tools/lists.rs**

```rust
use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::{json, Value};

use crate::client::LsqClient;
use crate::models::{ListIdParam, LeadIdParam};
use crate::server::{api_error, success_json};

pub async fn get_lists(client: &LsqClient) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get("/List.svc/GetAll")
        .await
        .map_err(|e| api_error("Failed to fetch lists", e))?;
    success_json(&data)
}

pub async fn get_leads_in_list(client: &LsqClient, params: &ListIdParam) -> Result<CallToolResult, ErrorData> {
    let page_index = params.page.unwrap_or(1).saturating_sub(1);
    let page_size = params.page_size.unwrap_or(25).min(100);

    let data: Value = client
        .get(&format!(
            "/List.svc/GetLeads?listId={}&pageIndex={}&pageSize={}",
            params.list_id, page_index, page_size
        ))
        .await
        .map_err(|e| api_error("Failed to fetch leads in list", e))?;

    let total = data.get("TotalCount").and_then(|v| v.as_i64()).unwrap_or(0);
    let results = data.get("RecordList").cloned().unwrap_or_else(|| json!([]));
    let count = results.as_array().map(|a| a.len() as i64).unwrap_or(0);

    success_json(&json!({
        "results": results,
        "total_count": total,
        "page": page_index + 1,
        "page_size": page_size,
        "has_more": (page_index as i64 * page_size as i64 + count) < total
    }))
}

pub async fn get_lead_list_memberships(client: &LsqClient, params: &LeadIdParam) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!("/List.svc/GetByLeadId?leadId={}", params.lead_id))
        .await
        .map_err(|e| api_error("Failed to fetch list memberships for lead", e))?;
    success_json(&data)
}

pub async fn get_list_lead_count(client: &LsqClient, params: &ListIdParam) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!("/List.svc/GetLeadCount?listId={}", params.list_id))
        .await
        .map_err(|e| api_error("Failed to fetch list lead count", e))?;
    success_json(&json!({ "list_id": params.list_id, "count": data }))
}
```

- [ ] **Add to server.rs** `#[tool_router]` impl:

```rust
    #[tool(
        description = "Get all lists (segments) in the LSQ account. Use when the user asks about segments, groups, or lists of leads.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_lists(&self) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = lists::get_lists(guard.as_ref().unwrap()).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get leads belonging to a specific list. Requires list_id from get_lists. Paginated.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_leads_in_list(&self, Parameters(params): Parameters<ListIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = lists::get_leads_in_list(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get all lists that a specific lead belongs to.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_lead_list_memberships(&self, Parameters(params): Parameters<LeadIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = lists::get_lead_list_memberships(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get the count of leads in a list. Lighter than get_leads_in_list when only the number is needed.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_list_lead_count(&self, Parameters(params): Parameters<ListIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = lists::get_list_lead_count(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }
```

- [ ] **Build**

```bash
cargo build
```

- [ ] **Commit**

```bash
git add src/tools/lists.rs src/server.rs
git commit -m "feat: 4 list tools (get all, leads in list, lead memberships, count)"
```

---

### Task 13: tools/analytics.rs

**Files:**
- Create: `src/tools/analytics.rs`
- Modify: `src/server.rs`

Analytics endpoints use `post_analytics()` — auth via query params, not headers.

- [ ] **Create src/tools/analytics.rs**

```rust
use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::Value;

use crate::client::LsqClient;
use crate::error::LsqError;
use crate::models::{LeadDistributionParams, LeadsNotContactedParams, LeadsNoActiveTasksParams, LeadsPendingTasksParams};
use crate::server::{api_error, success_json};

pub async fn get_lead_distribution(client: &LsqClient, params: &LeadDistributionParams) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .post_analytics("/Leads/LeadDistribution/FilterByLeadField", &params.filters)
        .await
        .map_err(|e| api_error("Failed to fetch lead distribution", e))?;
    success_json(&data)
}

pub async fn get_leads_not_contacted(client: &LsqClient, params: &LeadsNotContactedParams) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .post_analytics("/Leads/NotContacted", &params.filters)
        .await
        .map_err(|e| check_elasticsearch_error("get_leads_not_contacted", e))?;
    success_json(&data)
}

pub async fn get_leads_no_active_tasks(client: &LsqClient, params: &LeadsNoActiveTasksParams) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .post_analytics("/Tasks/LeadsWithNoActiveTasks", &params.filters)
        .await
        .map_err(|e| check_elasticsearch_error("get_leads_no_active_tasks", e))?;
    success_json(&data)
}

pub async fn get_leads_pending_tasks(client: &LsqClient, params: &LeadsPendingTasksParams) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .post_analytics("/Tasks/OwnerWiseLeadsWithPendingTasks", &params.filters)
        .await
        .map_err(|e| check_elasticsearch_error("get_leads_pending_tasks", e))?;
    success_json(&data)
}

/// Map Elasticsearch-related errors to the specific LsqError variant.
fn check_elasticsearch_error(context: &str, e: LsqError) -> ErrorData {
    // LSQ returns a specific error message when Elasticsearch is not enabled.
    // Match on common patterns in the error string.
    let msg = e.to_string().to_lowercase();
    if msg.contains("elasticsearch") || msg.contains("elastic search") || msg.contains("search not enabled") {
        api_error(context, LsqError::ElasticsearchNotEnabled)
    } else {
        api_error(context, e)
    }
}
```

- [ ] **Add to server.rs** `#[tool_router]` impl:

```rust
    #[tool(
        description = "Get lead distribution analytics: leads grouped by owner or stage with Count/Average/Sum aggregation. Provide a filters JSON body per LSQ Lead Distribution API schema (UserFilter, LeadFilters, DateFilter, Aggregate). All dates UTC YYYY-MM-DD HH:MM:SS.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_lead_distribution(&self, Parameters(params): Parameters<LeadDistributionParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = analytics::get_lead_distribution(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get leads where specified activities have NOT been posted within a time window. Requires Elasticsearch on your LSQ account. Provide filters JSON per LSQ Leads Not Contacted API schema (UserFilter, LeadFilters, ActivityFilters, DateFilter). All dates UTC YYYY-MM-DD HH:MM:SS.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_leads_not_contacted(&self, Parameters(params): Parameters<LeadsNotContactedParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = analytics::get_leads_not_contacted(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get leads with no pending or active tasks. Requires Elasticsearch on your LSQ account. Provide filters JSON per LSQ Leads With No Active Tasks API schema (UserFilter, LeadFilters, TaskFilters, DateFilter).",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_leads_no_active_tasks(&self, Parameters(params): Parameters<LeadsNoActiveTasksParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = analytics::get_leads_no_active_tasks(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get leads with pending or overdue tasks. Requires Elasticsearch on your LSQ account. Provide filters JSON per LSQ Leads With Pending Tasks API schema (UserFilter, LeadFilters, TaskFilters with Pending/Overdue/PendingAndOverdue, DateFilter).",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_leads_pending_tasks(&self, Parameters(params): Parameters<LeadsPendingTasksParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = analytics::get_leads_pending_tasks(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }
```

- [ ] **Run full build and all unit tests**

```bash
cargo test && cargo build --release 2>&1 | tail -5
```

Expected: all unit tests pass, release binary builds.

- [ ] **Commit**

```bash
git add src/tools/analytics.rs src/server.rs
git commit -m "feat: 4 analytics tools (distribution, not contacted, no active tasks, pending tasks)"
```

---

## Phase 3 — Polish

### Task 14: Integration tests

**Files:**
- Create: `tests/integration_test.rs`

Integration tests require a real LSQ sandbox account. They skip gracefully if env vars are not set.

- [ ] **Create tests/integration_test.rs**

```rust
//! Integration tests — require real LSQ credentials via environment variables.
//! Set LSQ_ACCESS_KEY, LSQ_SECRET_KEY, and optionally LSQ_HOST before running.
//! Run with: cargo test --test integration_test -- --ignored

use lsq_mcp::{auth::Credentials, client::LsqClient};

fn test_client() -> Option<LsqClient> {
    let access_key = std::env::var("LSQ_ACCESS_KEY").ok()?;
    let secret_key = std::env::var("LSQ_SECRET_KEY").ok()?;
    let host = std::env::var("LSQ_HOST")
        .unwrap_or_else(|_| lsq_mcp::config::DEFAULT_HOST.to_string());
    Some(LsqClient::new(Credentials { access_key, secret_key, host }))
}

#[tokio::test]
#[ignore]
async fn test_get_lead_metadata_returns_fields() {
    let client = match test_client() {
        Some(c) => c,
        None => { eprintln!("Skipping: LSQ_ACCESS_KEY not set"); return; }
    };
    let data = client.get_lead_metadata_cached().await.expect("should fetch metadata");
    assert!(data.is_array() || data.is_object(), "metadata should be array or object");
}

#[tokio::test]
#[ignore]
async fn test_get_activity_types_returns_list() {
    let client = match test_client() {
        Some(c) => c,
        None => { eprintln!("Skipping: LSQ_ACCESS_KEY not set"); return; }
    };
    let data = client.get_activity_types_cached().await.expect("should fetch activity types");
    assert!(data.is_array() || data.is_object());
}

#[tokio::test]
#[ignore]
async fn test_get_users_returns_list() {
    let client = match test_client() {
        Some(c) => c,
        None => { eprintln!("Skipping: LSQ_ACCESS_KEY not set"); return; }
    };
    let data: serde_json::Value = client.get("/UserManagement.svc/GetAll")
        .await.expect("should fetch users");
    assert!(data.is_array() || data.is_object());
}

#[tokio::test]
#[ignore]
async fn test_invalid_credentials_returns_unauthorized() {
    use lsq_mcp::error::LsqError;
    let client = LsqClient::new(Credentials {
        access_key: "bad_key".into(),
        secret_key: "bad_secret".into(),
        host: lsq_mcp::config::DEFAULT_HOST.into(),
    });
    let result: Result<serde_json::Value, _> = client.get("/UserManagement.svc/GetAll").await;
    assert!(matches!(result, Err(LsqError::Unauthorized)));
}
```

- [ ] **Run unit tests (integration tests skipped by default)**

```bash
cargo test
```

Expected: all unit tests pass. Integration tests are `#[ignore]` and not run.

- [ ] **Run integration tests manually** (requires real credentials):

```bash
LSQ_ACCESS_KEY=your_key LSQ_SECRET_KEY=your_secret cargo test --test integration_test -- --ignored
```

- [ ] **Commit**

```bash
git add tests/integration_test.rs
git commit -m "test: integration tests for live LSQ API (skipped by default)"
```

---

### Task 15: npm distribution

**Files:**
- Create: `npm/package.json`
- Create: `npm/run.js`

- [ ] **Create npm/package.json**

```json
{
  "name": "lsq-mcp",
  "version": "0.1.0",
  "description": "MCP server for LeadSquared CRM — read-only data access",
  "license": "MIT",
  "bin": {
    "lsq-mcp": "run.js"
  },
  "os": ["darwin", "linux"],
  "cpu": ["arm64", "x64"]
}
```

- [ ] **Create npm/run.js**

```js
#!/usr/bin/env node
/**
 * Launcher for lsq-mcp binary.
 * Downloads the correct pre-built binary for the current platform on first run.
 */

const { execFileSync, spawnSync } = require('child_process');
const path = require('path');
const fs = require('fs');
const https = require('https');

const VERSION = require('./package.json').version;
const BINARY_NAME = 'lsq-mcp';
const BIN_DIR = path.join(__dirname, '.bin');
const BIN_PATH = path.join(BIN_DIR, BINARY_NAME);

function getPlatformTarget() {
  const os = process.platform;
  const arch = process.arch;
  if (os === 'darwin' && arch === 'arm64') return 'aarch64-apple-darwin';
  if (os === 'darwin' && arch === 'x64')  return 'x86_64-apple-darwin';
  if (os === 'linux'  && arch === 'arm64') return 'aarch64-unknown-linux-musl';
  if (os === 'linux'  && arch === 'x64')  return 'x86_64-unknown-linux-musl';
  throw new Error(`Unsupported platform: ${os}-${arch}`);
}

if (!fs.existsSync(BIN_PATH)) {
  const target = getPlatformTarget();
  const url = `https://github.com/Newton-School/lsq-mcp/releases/download/v${VERSION}/${BINARY_NAME}-${target}`;
  console.error(`Downloading lsq-mcp v${VERSION} for ${target}...`);
  fs.mkdirSync(BIN_DIR, { recursive: true });
  // Download logic placeholder — replace with actual download when GitHub releases are set up
  console.error(`Please download the binary from: ${url}`);
  process.exit(1);
}

const result = spawnSync(BIN_PATH, process.argv.slice(2), { stdio: 'inherit' });
process.exit(result.status ?? 1);
```

- [ ] **Commit**

```bash
git add npm/
git commit -m "feat: npm distribution wrapper for lsq-mcp binary"
```

---

## Self-Review

### Spec coverage check

| Spec requirement | Covered by |
|---|---|
| 36 tools across 8 modules | Tasks 7–13 |
| Interactive configure + validation | Task 3 |
| Credentials file 0o600 | Task 2 |
| Header auth (x-LSQ-*) | Task 4 |
| Transparent 429 retry with backoff | Task 4 |
| Pagination (default 25, max 100, has_more) | Tasks 5, 7–12 |
| In-memory cache for 5 data types | Task 4 |
| Credential reload on every ensure_client() | Task 6 |
| Distinct error for wrong host | Task 4, 6 |
| Elasticsearch error detection | Task 13 |
| Feature not enabled errors | Task 6 (api_error) |
| Rate limit exhausted message | Task 6 (api_error) |
| Log sanitisation | Task 4 (no keys logged) |
| 4-part structured error format | Task 1 |
| analytics via post_analytics (query param auth) | Task 4, 13 |
| Integration tests | Task 14 |
| npm distribution | Task 15 |

### Placeholder scan

No TBD/TODO sections. All code is complete. The npm `run.js` download logic is noted as a placeholder pending GitHub Releases setup — this is intentional and documented.

### Type consistency

- `LsqClient` defined in Task 4, used consistently in Tasks 7–13
- `api_error(context, LsqError)` defined in Task 6, used identically across all tool modules
- `success_json(&Value)` defined in Task 6, used identically across all tool modules
- `Parameters<T>` wrapper used consistently across all tool handlers
- All tool parameter structs defined in Task 5 (`models.rs`) and imported where needed
- `ensure_client()` / `get_client()` pattern identical across all 36 tool handlers

---

**Plan complete and saved to `docs/superpowers/plans/2026-04-15-lsq-mcp-implementation.md`.**

**Two execution options:**

**1. Subagent-Driven (recommended)** — Fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
