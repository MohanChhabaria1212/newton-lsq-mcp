use std::io::{self, BufRead, Write};

use crate::auth::{self, Credentials};
use crate::config;
use crate::error::LsqError;

/// Run the interactive configure flow. Prompts for keys, validates, saves.
pub async fn configure() -> Result<(), LsqError> {
    let stdout = io::stdout();

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
        "{}/UserManagement.svc/Users.Get",
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
