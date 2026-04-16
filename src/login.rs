use std::io::{self, BufRead, Write};
use std::time::Duration;

use crate::auth::{self, Credentials};
use crate::config;
use crate::error::LsqError;

/// All known LSQ regional API hosts, in priority order.
const KNOWN_HOSTS: &[(&str, &str)] = &[
    ("api.leadsquared.com",    "India / Global"),
    ("api-us.leadsquared.com", "United States"),
    ("api-au.leadsquared.com", "Australia"),
];

/// Details about the authenticated user, fetched from LSQ during configure.
struct UserIdentity {
    name:  String,
    email: String,
    role:  String,
}

/// Run the interactive configure flow.
pub async fn configure() -> Result<(), LsqError> {
    let stdout = io::stdout();

    println!();
    println!("LeadSquared MCP Setup");
    println!("─────────────────────");
    println!("Find your API keys at: LSQ Portal → My Account → Settings → API and Webhooks");
    println!();

    let email = prompt(&stdout, "Your LSQ email address: ")?;
    if email.is_empty() {
        return Err(LsqError::Configure("Email cannot be empty".into()));
    }

    let access_key = prompt(&stdout, "Access Key: ")?;
    if access_key.is_empty() {
        return Err(LsqError::Configure("Access key cannot be empty".into()));
    }

    let secret_key = prompt(&stdout, "Secret Key: ")?;
    if secret_key.is_empty() {
        return Err(LsqError::Configure("Secret key cannot be empty".into()));
    }

    // Auto-detect API host — no need for the user to know their region
    println!();
    println!("Detecting API region...");

    let host = match detect_host(&access_key, &secret_key).await {
        Some((host, region)) => {
            println!("  Region: {} ({})", region, host);
            host.to_string()
        }
        None => {
            // Detection failed — fall back to manual entry
            println!("  Could not auto-detect region. Please enter your API host.");
            println!("  Regions: {} India/Global  |  {} US  |  {} AU",
                KNOWN_HOSTS[0].0, KNOWN_HOSTS[1].0, KNOWN_HOSTS[2].0);
            println!();
            let host_input = prompt(
                &stdout,
                &format!("API Host [{}]: ", config::DEFAULT_HOST),
            )?;
            if host_input.is_empty() {
                config::DEFAULT_HOST.to_string()
            } else {
                host_input
                    .trim_start_matches("https://")
                    .trim_start_matches("http://")
                    .trim_end_matches('/')
                    .to_string()
            }
        }
    };

    println!();
    println!("Verifying credentials...");

    let creds_base = Credentials {
        access_key,
        secret_key,
        host,
        user_name:  None,
        user_email: None,
        user_role:  None,
    };

    let identity = match validate_and_lookup(&creds_base, &email).await {
        Ok(id) => id,
        Err(LsqError::Unauthorized) => {
            println!();
            println!("✗ Invalid credentials — LSQ returned 401 Unauthorized.");
            println!("  Double-check your keys at: LSQ Portal → My Account → Settings → API and Webhooks");
            return Err(LsqError::Configure("Invalid credentials".into()));
        }
        Err(LsqError::HostUnreachable(ref h)) => {
            println!();
            println!("✗ Could not reach {}.", h);
            return Err(LsqError::Configure(format!("Host unreachable: {}", h)));
        }
        Err(LsqError::NotFound) => {
            println!();
            println!("✗ Email '{}' was not found in this LSQ account.", email);
            println!("  Make sure you entered the email address you use to log into the LSQ portal.");
            println!("  Note: admin credentials are required to look up other users.");
            return Err(LsqError::Configure("Email not found in account".into()));
        }
        Err(e) => {
            println!();
            println!("✗ Verification failed: {}", e);
            return Err(e);
        }
    };

    println!();
    println!("Found account:");
    println!("  Name:   {}", identity.name);
    println!("  Email:  {}", identity.email);
    println!("  Role:   {}", identity.role);
    println!();

    let confirm = prompt(&stdout, "Connect as this user? [y/N]: ")?;
    if !matches!(confirm.trim().to_lowercase().as_str(), "y" | "yes") {
        println!();
        println!("Cancelled. Nothing was saved.");
        return Err(LsqError::Configure("User cancelled".into()));
    }

    let creds = Credentials {
        user_name:  Some(identity.name),
        user_email: Some(identity.email),
        user_role:  Some(identity.role),
        ..creds_base
    };

    auth::save_credentials(&creds)?;
    println!();
    println!("✓ Connected. Credentials saved to ~/.lsq-mcp/credentials.json");
    println!("  Start your MCP client to begin using lsq-mcp.");
    println!();
    Ok(())
}

/// Probe all known LSQ hosts in parallel and return the first one that
/// responds (200 or 401 both confirm the host is reachable with these keys).
/// Returns `(host, region_label)` for the winning host.
async fn detect_host(access_key: &str, secret_key: &str) -> Option<(&'static str, &'static str)> {
    let (r0, r1, r2) = tokio::join!(
        probe_host(access_key, secret_key, KNOWN_HOSTS[0].0),
        probe_host(access_key, secret_key, KNOWN_HOSTS[1].0),
        probe_host(access_key, secret_key, KNOWN_HOSTS[2].0),
    );

    // Pick first reachable host in priority order
    for (result, (host, region)) in [r0, r1, r2].into_iter().zip(KNOWN_HOSTS.iter()) {
        if result {
            return Some((host, region));
        }
    }
    None
}

/// Returns true if the host responds (any HTTP status — even 401 confirms the host exists).
async fn probe_host(access_key: &str, secret_key: &str, host: &str) -> bool {
    let url = format!("{}/UserManagement.svc/Users.Get", config::api_base(host));
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
    else {
        return false;
    };

    client
        .get(&url)
        .header("x-LSQ-AccessKey", access_key)
        .header("x-LSQ-SecretKey", secret_key)
        .send()
        .await
        .is_ok() // any response (including 401) = host is reachable
}

/// Validate credentials and look up the specific user by email.
async fn validate_and_lookup(
    creds: &Credentials,
    email: &str,
) -> Result<UserIdentity, LsqError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(LsqError::Api)?;

    let base = config::api_base(&creds.host);

    // Verify the keys are valid
    let check = client
        .get(format!("{}/UserManagement.svc/Users.Get", base))
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

    if check.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(LsqError::Unauthorized);
    }
    check.error_for_status().map_err(LsqError::Api)?;

    // Look up the specific user by email
    let search_body = serde_json::json!({
        "Filters": [
            { "Attribute": "EmailAddress", "Operator": "eq", "Value": email }
        ],
        "Paging": { "PageIndex": 0, "PageSize": 1 }
    });

    let resp = client
        .post(format!("{}/UserManagement.svc/User/AdvancedSearch", base))
        .header("x-LSQ-AccessKey", &creds.access_key)
        .header("x-LSQ-SecretKey", &creds.secret_key)
        .json(&search_body)
        .send()
        .await
        .map_err(LsqError::Api)?
        .error_for_status()
        .map_err(LsqError::Api)?;

    let body: serde_json::Value = resp.json().await.map_err(LsqError::Api)?;

    // AdvancedSearch returns { Users: [...] } or array directly — handle both
    let users = body
        .get("Users")
        .or_else(|| body.get("RecordList"))
        .and_then(|v| v.as_array())
        .or_else(|| body.as_array())
        .ok_or(LsqError::NotFound)?;

    let user = users.first().ok_or(LsqError::NotFound)?;

    let first = user.get("FirstName").and_then(|v| v.as_str()).unwrap_or("");
    let last  = user.get("LastName").and_then(|v| v.as_str()).unwrap_or("");
    let name  = format!("{} {}", first, last).trim().to_string();
    let name  = if name.is_empty() {
        user.get("UserName")
            .and_then(|v| v.as_str())
            .unwrap_or(email)
            .to_string()
    } else {
        name
    };

    let found_email = user
        .get("EmailAddress")
        .and_then(|v| v.as_str())
        .unwrap_or(email)
        .to_string();

    let role = user
        .get("RoleName")
        .or_else(|| user.get("UserRole"))
        .or_else(|| user.get("Role"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    Ok(UserIdentity { name, email: found_email, role })
}

fn prompt(stdout: &io::Stdout, label: &str) -> Result<String, LsqError> {
    let mut out = stdout.lock();
    write!(out, "{}", label)?;
    out.flush()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

/// Print current config status.
pub fn status() {
    match auth::load_credentials() {
        Ok(Some(creds)) => {
            println!("Status: Configured");
            println!("  Host:       {}", creds.host);
            if let (Some(name), Some(email), Some(role)) =
                (&creds.user_name, &creds.user_email, &creds.user_role)
            {
                println!("  Connected:  {} ({}) — {}", name, email, role);
            }
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
