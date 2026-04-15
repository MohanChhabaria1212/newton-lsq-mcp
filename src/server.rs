use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::{ErrorData, ServerHandler, tool, tool_handler, tool_router};
use tokio::sync::RwLock;

use crate::auth;
use crate::client::LsqClient;
use crate::error::{LsqError, lsq_error};
use crate::models::*;
use crate::tools::instructions;
use crate::tools::leads;
use crate::tools::opportunities;

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
        description = "Get the full activity history for a lead — every interaction logged in LSQ. Returns all activity types mixed together; filter by activity type name if needed.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_lead_activities(&self, Parameters(params): Parameters<LeadIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = leads::get_lead_activities(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get all opportunity types available on this LSQ account. Call this before get_opportunity_metadata to get valid type IDs.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_opportunity_types(&self) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = opportunities::get_opportunity_types(guard.as_ref().unwrap()).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get the field schema for a specific opportunity type. Use the opportunity_type_id from get_opportunity_types.",
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
        description = "Get all opportunities associated with a lead. Returns all opportunity types for that lead.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_opportunities_by_lead(&self, Parameters(params): Parameters<LeadIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = opportunities::get_opportunities_by_lead(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Search opportunities with filter conditions. Returns paginated results with has_more. Use get_opportunity_metadata to discover valid filter field names.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn search_opportunities(&self, Parameters(params): Parameters<SearchOpportunitiesParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = opportunities::search_opportunities(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for LsqMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("lsq-mcp", crate::config::VERSION))
            .with_protocol_version(ProtocolVersion::V_2024_11_05)
    }
}
