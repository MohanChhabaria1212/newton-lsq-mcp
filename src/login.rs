use std::io::{self, BufRead, Write};
use std::time::Duration;

use crate::auth::{self, Credentials};
use crate::config;
use crate::error::LsqError;

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

    // Derive API host from the browser URL the user already knows
    println!();
    println!("  Paste the URL you use to open LeadSquared in your browser");
    println!("  (e.g. https://app.in21.leadsquared.com/leads)");
    let portal_url = prompt(&stdout, "LSQ Portal URL: ")?;

    let host = match derive_api_host(&portal_url) {
        Some(h) => h,
        None => {
            println!();
            println!("✗ Could not recognise that as a LeadSquared URL.");
            println!("  Expected something like: https://app.in21.leadsquared.com");
            return Err(LsqError::Configure("Invalid portal URL".into()));
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
            println!("  Make sure you entered the email you use to log into the LSQ portal.");
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

/// Derive the LSQ API host from whatever URL the user opens in their browser.
///
/// Handles full URLs with paths/query strings, http or https, trailing slashes.
///
/// Mapping rules (all must end with `.leadsquared.com`):
///   app.leadsquared.com            → api.leadsquared.com
///   app.{cluster}.leadsquared.com  → api-{cluster}.leadsquared.com
///   app-{region}.leadsquared.com   → api-{region}.leadsquared.com
///
/// Returns `None` if the URL is not a recognised LeadSquared portal URL.
pub fn derive_api_host(input: &str) -> Option<String> {
    // Strip protocol, get just the hostname
    let without_proto = input
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    // Take only the host part (before first / or ?)
    let hostname = without_proto
        .split(|c| c == '/' || c == '?' || c == '#')
        .next()?
        .trim()
        .to_lowercase();

    if hostname.is_empty() {
        return None;
    }

    // Must be a leadsquared.com domain
    let subdomain = hostname.strip_suffix(".leadsquared.com")?;

    let api_host = if subdomain == "app" {
        // app.leadsquared.com → api.leadsquared.com
        "api.leadsquared.com".to_string()
    } else if let Some(cluster) = subdomain.strip_prefix("app.") {
        // app.in21.leadsquared.com → api-in21.leadsquared.com
        // Reject if cluster itself contains dots (unexpected nesting)
        if cluster.is_empty() || cluster.contains('.') {
            return None;
        }
        format!("api-{}.leadsquared.com", cluster)
    } else if let Some(region) = subdomain.strip_prefix("app-") {
        // app-us.leadsquared.com → api-us.leadsquared.com
        if region.is_empty() {
            return None;
        }
        format!("api-{}.leadsquared.com", region)
    } else {
        return None;
    };

    Some(api_host)
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

    // One call: validate credentials AND fetch users list
    let users_url = reqwest::Url::parse_with_params(
        &format!("{}/UserManagement.svc/Users.Get", base),
        &[
            ("accessKey", creds.access_key.as_str()),
            ("secretKey", creds.secret_key.as_str()),
        ],
    ).map_err(|e| LsqError::Configure(format!("URL error: {}", e)))?;

    let resp = client
        .get(users_url)
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
    let body: serde_json::Value = resp
        .error_for_status()
        .map_err(LsqError::Api)?
        .json()
        .await
        .map_err(LsqError::Api)?;

    // Users.Get returns an array directly
    let users = body
        .as_array()
        .ok_or(LsqError::NotFound)?;

    // Find the user whose email actually matches — the API may return all users
    // regardless of the filter, and obsolete accounts have emails like
    // "user@domain.com.12345.obsolete". We skip those and match by exact email.
    let email_lower = email.trim().to_lowercase();
    let user = users
        .iter()
        .find(|u| {
            u.get("EmailAddress")
                .and_then(|v| v.as_str())
                .map(|e| e.trim().to_lowercase() == email_lower)
                .unwrap_or(false)
        })
        .ok_or(LsqError::NotFound)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_host_standard_india() {
        assert_eq!(
            derive_api_host("https://app.leadsquared.com/leads"),
            Some("api.leadsquared.com".into())
        );
    }

    #[test]
    fn derive_host_cluster_with_path() {
        assert_eq!(
            derive_api_host("https://app.in21.leadsquared.com/leads/list"),
            Some("api-in21.leadsquared.com".into())
        );
    }

    #[test]
    fn derive_host_region_us() {
        assert_eq!(
            derive_api_host("https://app-us.leadsquared.com"),
            Some("api-us.leadsquared.com".into())
        );
    }

    #[test]
    fn derive_host_no_protocol() {
        assert_eq!(
            derive_api_host("app.in21.leadsquared.com/dashboard"),
            Some("api-in21.leadsquared.com".into())
        );
    }

    #[test]
    fn derive_host_invalid_domain() {
        assert_eq!(derive_api_host("https://google.com"), None);
    }

    #[test]
    fn derive_host_random_string() {
        assert_eq!(derive_api_host("not a url"), None);
    }
}
