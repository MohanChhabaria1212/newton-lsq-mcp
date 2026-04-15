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
            let req = if !query_params.is_empty() {
                self.http
                    .post(url)
                    .query(query_params)
                    .header("x-LSQ-AccessKey", &self.creds.access_key)
                    .header("x-LSQ-SecretKey", &self.creds.secret_key)
                    .json(body)
            } else {
                self.http
                    .post(url)
                    .header("x-LSQ-AccessKey", &self.creds.access_key)
                    .header("x-LSQ-SecretKey", &self.creds.secret_key)
                    .json(body)
            };

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
