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
    /// Override base URL (v2) for unit/integration tests pointing at a mock server.
    #[cfg(test)]
    test_base_url: Option<String>,
    /// Override analytics base URL for unit/integration tests pointing at a mock server.
    #[cfg(test)]
    test_analytics_base_url: Option<String>,
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
            #[cfg(test)]
            test_base_url: None,
            #[cfg(test)]
            test_analytics_base_url: None,
        }
    }

    fn base(&self) -> String {
        #[cfg(test)]
        if let Some(ref url) = self.test_base_url {
            return url.clone();
        }
        config::api_base(&self.creds.host)
    }

    fn analytics_base(&self) -> String {
        #[cfg(test)]
        if let Some(ref url) = self.test_analytics_base_url {
            return url.clone();
        }
        config::analytics_base(&self.creds.host)
    }

    // ── Core HTTP with retry ─────────────────────────────────────────────

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, LsqError> {
        let url = format!("{}{}", self.base(), path);
        self.get_url(&url).await
    }

    pub async fn get_url<T: DeserializeOwned>(&self, url: &str) -> Result<T, LsqError> {
        // Append auth as query params — LSQ's standard auth mechanism
        let auth_url = reqwest::Url::parse_with_params(url, &[
            ("accessKey", self.creds.access_key.as_str()),
            ("secretKey", self.creds.secret_key.as_str()),
        ]).map_err(|e| LsqError::Configure(format!("Invalid URL: {}", e)))?;
        let url = auth_url.as_str();

        let mut delay_secs = 1u64;
        for attempt in 0..=MAX_RETRIES {
            let resp = self.http
                .get(url)
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
                    // Do NOT log `url` here — it contains the access/secret keys as query params.
                    tracing::debug!("429 rate limit, backing off {}s (attempt {})", wait, attempt + 1);
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

    /// POST for analytics endpoints — uses a different base URL and needs
    /// responseformat=json in addition to the standard auth params.
    pub async fn post_analytics<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &Value,
    ) -> Result<T, LsqError> {
        let url = format!("{}{}", self.analytics_base(), path);
        self.post_url(&url, body, &[("responseformat", "json")]).await
    }

    /// All POSTs go through here. Auth is always appended as query params.
    /// `extra_params` are appended after auth (used by analytics for responseformat).
    async fn post_url<T: DeserializeOwned>(
        &self,
        url: &str,
        body: &Value,
        extra_params: &[(&str, &str)],
    ) -> Result<T, LsqError> {
        // Build auth + extra params, then bake into URL once before the retry loop
        let mut all_params = vec![
            ("accessKey", self.creds.access_key.as_str()),
            ("secretKey", self.creds.secret_key.as_str()),
        ];
        all_params.extend_from_slice(extra_params);

        let auth_url = reqwest::Url::parse_with_params(url, &all_params)
            .map_err(|e| LsqError::Configure(format!("Invalid URL: {}", e)))?;
        let url = auth_url.as_str().to_string();

        let mut delay_secs = 1u64;
        for attempt in 0..=MAX_RETRIES {
            let req = self.http
                    .post(&url)
                    .json(body);

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
                    // Do NOT log `url` here — it contains the access/secret keys as query params.
                    tracing::debug!("429 rate limit, backing off {}s", wait);
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
            self.get::<Value>("/LeadManagement.svc/LeadsMetaData.Get").await
        }).await
    }

    pub async fn get_activity_types_cached(&self) -> Result<Value, LsqError> {
        cached_get(&self.activity_types_cache, || async {
            self.get::<Value>("/ProspectActivity.svc/ActivityTypes.Get").await
        }).await
    }

    pub async fn get_opportunity_types_cached(&self) -> Result<Value, LsqError> {
        cached_get(&self.opportunity_types_cache, || async {
            self.get::<Value>("/OpportunityManagement.svc/GetOpportunityTypes").await
        }).await
    }

    pub async fn get_task_types_cached(&self) -> Result<Value, LsqError> {
        cached_get(&self.task_types_cache, || async {
            // LSQ does not publish a dedicated task-types endpoint in their docs.
            // This path is a best-effort guess; update here if the correct path is confirmed.
            self.get::<Value>("/Task.svc/TaskType/GetAll").await
        }).await
    }

    pub async fn get_products_cached(&self) -> Result<Value, LsqError> {
        cached_get(&self.products_cache, || async {
            // LSQ does not publish a dedicated products endpoint in their docs.
            // This path is a best-effort guess; update here if the correct path is confirmed.
            self.get::<Value>("/SalesActivity.svc/Product/GetAll").await
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
impl LsqClient {
    /// Create a client that routes all HTTP calls to a local mock server.
    ///
    /// `server_uri` is the base URI of the mock server, e.g. `http://127.0.0.1:12345`.
    /// Standard v2 API calls will go to `{server_uri}/v2/...`.
    /// Analytics API calls will go to `{server_uri}/...` (no /v2 prefix).
    pub(crate) fn new_for_testing(creds: Credentials, server_uri: &str) -> Self {
        let mut client = Self::new(creds);
        client.test_base_url = Some(format!("{}/v2", server_uri));
        client.test_analytics_base_url = Some(server_uri.to_string());
        client
    }
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
            user_name: None,
            user_email: None,
            user_role: None,
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
