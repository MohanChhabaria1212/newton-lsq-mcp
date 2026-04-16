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

    #[error("Not found")]
    NotFound,

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
