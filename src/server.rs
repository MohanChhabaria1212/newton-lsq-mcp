use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::{ErrorData, ServerHandler, tool, tool_handler, tool_router};
use tokio::sync::RwLock;

use crate::auth;
use crate::client::LsqClient;
use crate::error::{LsqError, lsq_error};
use crate::models::{
    ActivitiesByLeadParams, ActivityIdParam, AvailabilityParams, AppointmentParams,
    CheckInHistoryParams, GetLeadsByIdsParams, GetOpportunitiesByLeadFieldParams,
    IsOpportunityEnabledParams, LeadDistributionParams, LeadEmailParam, LeadIdParam,
    LeadListMembershipsParam, LeadOwnerParams, LeadPhoneParam, LeadsNoActiveTasksParams,
    LeadsNotContactedParams, LeadsPendingTasksParams, ListIdParam, OpportunityIdParam,
    OpportunityMetadataParams, QuickSearchLeadsParams, RecentlyModifiedActivitiesParams,
    RecentlyModifiedLeadsParams, SearchLeadsParams, SearchOpportunitiesParams,
    SearchUsersParams, SalesActivitiesByLeadParams, TasksByLeadParams, TasksByOwnerParams,
    UserHierarchyParams, UserIdParam,
};
use crate::tools::activities;
use crate::tools::analytics;
use crate::tools::instructions;
use crate::tools::leads;
use crate::tools::lists;
use crate::tools::opportunities;
use crate::tools::sales;
use crate::tools::tasks;
use crate::tools::users;

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
        description = "Full-text quick search for leads across name, email, phone, company, city and country. Returns matching leads. Use for fuzzy/partial lookups when you don't know the exact field to filter on.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn quick_search_leads(&self, Parameters(params): Parameters<QuickSearchLeadsParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = leads::quick_search_leads(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Fetch multiple leads at once by a list of ProspectIDs. More efficient than calling get_lead_by_id in a loop. Pass up to 10,000 IDs.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_leads_by_ids(&self, Parameters(params): Parameters<GetLeadsByIdsParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = leads::get_leads_by_ids(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get the assigned owner (sales rep) of a lead. Look up by any unique field — e.g. lead_identifier='EmailAddress', value='john@example.com'.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_lead_owner(&self, Parameters(params): Parameters<LeadOwnerParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = leads::get_lead_owner(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get leads modified between two UTC timestamps. Useful for syncing or reviewing recent changes. Dates must be UTC YYYY-MM-DD HH:MM:SS.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_recently_modified_leads(&self, Parameters(params): Parameters<RecentlyModifiedLeadsParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = leads::get_recently_modified_leads(guard.as_ref().unwrap(), &params).await;
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
        description = "Search opportunities with advanced filters. Provide opportunity_type_code (from get_opportunity_types) and optional advanced_search JSON. Returns paginated results.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn search_opportunities(&self, Parameters(params): Parameters<SearchOpportunitiesParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = opportunities::search_opportunities(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Check whether the Opportunity Management feature is enabled on an LSQ account. Requires the organisation ID from the LSQ portal.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn is_opportunity_enabled(&self, Parameters(params): Parameters<IsOpportunityEnabledParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = opportunities::is_opportunity_enabled(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get opportunities for leads matching a unique field value (e.g. Mobile, EmailAddress). Useful when you know a lead's phone/email but not their ProspectID.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_opportunities_by_lead_field(&self, Parameters(params): Parameters<GetOpportunitiesByLeadFieldParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = opportunities::get_opportunities_by_lead_field(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get activities logged on an opportunity by its ID. Note: path is unconfirmed — if this returns a 404, report it so the endpoint can be corrected.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_activities_of_opportunity(&self, Parameters(params): Parameters<OpportunityIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = opportunities::get_activities_of_opportunity(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get all activity type definitions for this LSQ account — names, IDs, and field schemas. Cached after first call. Call this before filtering activities by type.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_activity_types(&self) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = activities::get_activity_types(guard.as_ref().unwrap()).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get paginated activity log for a lead. Returns all activity types; filter by activity name after retrieval if needed.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_activities_by_lead(&self, Parameters(params): Parameters<ActivitiesByLeadParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = activities::get_activities_by_lead(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get full details of a single activity record by its activity ID — includes all field values and metadata.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_activity_details(&self, Parameters(params): Parameters<ActivityIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = activities::get_activity_details(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get the owner (assigned user) of a specific activity. Note: path is unconfirmed — if this returns a 404, report it so the endpoint can be corrected.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_activity_owner(&self, Parameters(params): Parameters<ActivityIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = activities::get_activity_owner(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get custom activity type settings and schema for this LSQ account. Note: path is unconfirmed — if this returns a 404, report it so the endpoint can be corrected.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_activity_settings(&self) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = activities::get_activity_settings(guard.as_ref().unwrap()).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get activities modified between two UTC timestamps. Dates must be UTC YYYY-MM-DD HH:MM:SS. Note: path is unconfirmed — if this returns a 404, report it so the endpoint can be corrected.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_recently_modified_activities(&self, Parameters(params): Parameters<RecentlyModifiedActivitiesParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = activities::get_recently_modified_activities(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get the product catalogue. Cached after first call. Products are referenced in sales activity records.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_products(&self) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = sales::get_products(guard.as_ref().unwrap()).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get all sales activity type configurations for this account.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_sales_activity_types(&self) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = sales::get_sales_activity_types(guard.as_ref().unwrap()).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get sales activity (transaction) records for a lead. Returns paginated sales interactions with product and revenue data.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_sales_activities_by_lead(&self, Parameters(params): Parameters<SalesActivitiesByLeadParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = sales::get_sales_activities_by_lead(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get all task type definitions for this LSQ account. Cached after first call.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_task_types(&self) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = tasks::get_task_types(guard.as_ref().unwrap()).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get paginated tasks for a lead. Returns all task types; filter by type name if needed.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_tasks_by_lead(&self, Parameters(params): Parameters<TasksByLeadParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = tasks::get_tasks_by_lead(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get tasks assigned to a specific user (by user ID). Returns paginated results.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_tasks_by_owner(&self, Parameters(params): Parameters<TasksByOwnerParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = tasks::get_tasks_by_owner(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get appointment tasks for a user (by user_id or email). Returns scheduled meetings and calls.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_appointments(&self, Parameters(params): Parameters<AppointmentParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = tasks::get_appointments(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get to-do tasks for a user (by user_id or email). Returns follow-up items and reminders.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_todos(&self, Parameters(params): Parameters<AppointmentParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = tasks::get_todos(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get all users in the LSQ account. Returns up to 200 users. Use search_users for accounts with more users or to filter by specific attributes.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_users(&self) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = users::get_users(guard.as_ref().unwrap()).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get a single user's details by their user ID.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_user_by_id(&self, Parameters(params): Parameters<UserIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = users::get_user_by_id(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Search users with filter conditions. Useful for large accounts or filtering by role, team, or other attributes.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn search_users(&self, Parameters(params): Parameters<SearchUsersParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = users::search_users(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get all users in a manager's reporting chain (hierarchy). Pass the manager's user ID.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_user_hierarchy(&self, Parameters(params): Parameters<UserHierarchyParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = users::get_user_hierarchy(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get field check-in history for a user. Optionally filter by date range (UTC YYYY-MM-DD HH:MM:SS).",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_user_checkin_history(&self, Parameters(params): Parameters<CheckInHistoryParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = users::get_user_checkin_history(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get working hours and availability slots for a user (by user_id or email).",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_user_availability(&self, Parameters(params): Parameters<AvailabilityParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = users::get_user_availability(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get all lists (static and dynamic) in the LSQ account. Returns list names, IDs, and types.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_lists(&self) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = lists::get_lists(guard.as_ref().unwrap()).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get paginated leads in a specific list. Use get_lists first to find the list ID.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_leads_in_list(&self, Parameters(params): Parameters<ListIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = lists::get_leads_in_list(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get all lists that a specific lead belongs to. Pass the lead's ProspectID.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_lead_list_memberships(&self, Parameters(params): Parameters<LeadListMembershipsParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = lists::get_lead_list_memberships(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get the total number of leads in a list without fetching the leads themselves.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_list_lead_count(&self, Parameters(params): Parameters<ListIdParam>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = lists::get_list_lead_count(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get lead distribution by owner, stage, or other dimensions with aggregation. Requires Elasticsearch enabled on your LSQ account. Pass filters JSON following the LSQ Lead Distribution API schema.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_lead_distribution(&self, Parameters(params): Parameters<LeadDistributionParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = analytics::get_lead_distribution(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get leads that have not been contacted (no qualifying activity) in a date range. Requires Elasticsearch. Pass filters JSON following the LSQ Leads Not Contacted API schema.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_leads_not_contacted(&self, Parameters(params): Parameters<LeadsNotContactedParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = analytics::get_leads_not_contacted(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get leads that have no active (pending) tasks. Requires Elasticsearch. Pass filters JSON following the LSQ Leads With No Active Tasks API schema.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_leads_no_active_tasks(&self, Parameters(params): Parameters<LeadsNoActiveTasksParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = analytics::get_leads_no_active_tasks(guard.as_ref().unwrap(), &params).await;
        check_auth(self, result).await
    }

    #[tool(
        description = "Get leads with overdue or pending tasks. Requires Elasticsearch. Pass filters JSON following the LSQ Leads With Pending Tasks API schema (use TaskFilters.Status: Pending/Overdue/PendingAndOverdue).",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn get_leads_pending_tasks(&self, Parameters(params): Parameters<LeadsPendingTasksParams>) -> Result<CallToolResult, ErrorData> {
        if let Err(e) = self.ensure_client().await { return Ok(e); }
        let guard = self.get_client().await;
        let result = analytics::get_leads_pending_tasks(guard.as_ref().unwrap(), &params).await;
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
