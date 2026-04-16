use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::Value;

use crate::client::LsqClient;
use crate::models::{LeadListMembershipsParam, ListIdParam};
use crate::server::{api_error, success_json};

pub async fn get_lists(client: &LsqClient) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get("/LeadManagement.svc/Lists.Get")
        .await
        .map_err(|e| api_error("Failed to fetch lists", e))?;
    success_json(&data)
}

pub async fn get_leads_in_list(
    client: &LsqClient,
    params: &ListIdParam,
) -> Result<CallToolResult, ErrorData> {
    let page_index = params.page.unwrap_or(1).saturating_sub(1);
    let page_size = params.page_size.unwrap_or(25).min(100);

    let data: Value = client
        .get(&format!(
            "/LeadManagement.svc/List.GetLeads?listId={}&pageIndex={}&pageSize={}",
            params.list_id, page_index, page_size
        ))
        .await
        .map_err(|e| api_error("Failed to fetch leads in list", e))?;
    success_json(&data)
}

pub async fn get_lead_list_memberships(
    client: &LsqClient,
    params: &LeadListMembershipsParam,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!(
            "/List.svc/GetByLeadId?leadId={}",
            params.lead_id
        ))
        .await
        .map_err(|e| api_error("Failed to fetch lead list memberships", e))?;
    success_json(&data)
}

pub async fn get_list_lead_count(
    client: &LsqClient,
    params: &ListIdParam,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!("/List.svc/GetLeadCount?listId={}", params.list_id))
        .await
        .map_err(|e| api_error("Failed to fetch list lead count", e))?;
    success_json(&data)
}
