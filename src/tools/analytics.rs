use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::Value;

use crate::client::LsqClient;
use crate::models::{
    LeadDistributionParams, LeadsNoActiveTasksParams, LeadsNotContactedParams,
    LeadsPendingTasksParams,
};
use crate::server::{api_error, success_json};

pub async fn get_lead_distribution(
    client: &LsqClient,
    params: &LeadDistributionParams,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .post_analytics("/Leads/LeadDistribution/FilterByLeadField", &params.filters)
        .await
        .map_err(|e| api_error("Failed to fetch lead distribution", e))?;
    success_json(&data)
}

pub async fn get_leads_not_contacted(
    client: &LsqClient,
    params: &LeadsNotContactedParams,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .post_analytics("/Leads/LeadsNotContacted", &params.filters)
        .await
        .map_err(|e| api_error("Failed to fetch leads not contacted", e))?;
    success_json(&data)
}

pub async fn get_leads_no_active_tasks(
    client: &LsqClient,
    params: &LeadsNoActiveTasksParams,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .post_analytics("/Leads/LeadsWithNoActiveTasks", &params.filters)
        .await
        .map_err(|e| api_error("Failed to fetch leads with no active tasks", e))?;
    success_json(&data)
}

pub async fn get_leads_pending_tasks(
    client: &LsqClient,
    params: &LeadsPendingTasksParams,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .post_analytics("/Leads/LeadsWithPendingTasks", &params.filters)
        .await
        .map_err(|e| api_error("Failed to fetch leads with pending tasks", e))?;
    success_json(&data)
}
