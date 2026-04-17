use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::{json, Value};

use crate::client::LsqClient;
use crate::models::{
    GetOpportunitiesByLeadFieldParams, IsOpportunityEnabledParams, LeadIdParam, OpportunityIdParam,
    OpportunityMetadataParams, SearchOpportunitiesParams,
};
use crate::server::{api_error, success_json};

pub async fn get_opportunity_types(client: &LsqClient) -> Result<CallToolResult, ErrorData> {
    let data = client
        .get_opportunity_types_cached()
        .await
        .map_err(|e| api_error("Failed to fetch opportunity types", e))?;
    success_json(&data)
}

pub async fn get_opportunity_metadata(
    client: &LsqClient,
    params: &OpportunityMetadataParams,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!(
            "/OpportunityManagement.svc/GetOpportunityTypeMetadata?code={}",
            params.opportunity_type_id
        ))
        .await
        .map_err(|e| api_error("Failed to fetch opportunity metadata", e))?;
    success_json(&data)
}

pub async fn get_opportunity_by_id(
    client: &LsqClient,
    params: &OpportunityIdParam,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!(
            "/OpportunityManagement.svc/GetOpportunityDetails?OpportunityId={}",
            params.opportunity_id
        ))
        .await
        .map_err(|e| api_error("Failed to fetch opportunity by ID", e))?;
    success_json(&data)
}

pub async fn get_opportunities_by_lead(
    client: &LsqClient,
    params: &LeadIdParam,
) -> Result<CallToolResult, ErrorData> {
    // leadId goes as query param; empty body triggers default (all types)
    let data: Value = client
        .post(
            &format!("/OpportunityManagement.svc/GetOpportunitiesOfLead?leadId={}", params.lead_id),
            &json!({}),
        )
        .await
        .map_err(|e| api_error("Failed to fetch opportunities by lead", e))?;
    success_json(&data)
}

pub async fn search_opportunities(
    client: &LsqClient,
    params: &SearchOpportunitiesParams,
) -> Result<CallToolResult, ErrorData> {
    // LSQ PageIndex is 1-based
    let page_index = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(25).min(100);

    // AdvancedSearch must be a JSON-encoded string, not an object
    let advanced_search = params.advanced_search
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "{}".to_string());

    let mut body = json!({
        "Paging": { "PageIndex": page_index, "PageSize": page_size },
        "AdvancedSearch": advanced_search
    });

    if let Some(code) = params.opportunity_type_code {
        body["OpportunityEventCode"] = json!(code);
    }

    let data: Value = client
        .post("/OpportunityManagement.svc/Retrieve/BySearchParameter", &body)
        .await
        .map_err(|e| api_error("Failed to search opportunities", e))?;

    // Response: { "RecordCount": N, "List": [...] }
    let total = data.get("RecordCount").and_then(|v| v.as_i64()).unwrap_or(0);
    let results = data.get("List").cloned().unwrap_or_else(|| json!([]));
    let count = results.as_array().map(|a| a.len() as i64).unwrap_or(0);
    let has_more = (page_index as i64 - 1) * page_size as i64 + count < total;

    success_json(&json!({
        "results": results,
        "total_count": total,
        "page": page_index,
        "page_size": page_size,
        "has_more": has_more
    }))
}

/// Check whether the Opportunity feature is enabled for an organisation.
pub async fn is_opportunity_enabled(
    client: &LsqClient,
    params: &IsOpportunityEnabledParams,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get_with_params(
            "/OpportunityManagement.svc/IsOpportunityEnabled",
            &[("orgId", params.org_id.as_str())],
        )
        .await
        .map_err(|e| api_error("Failed to check if opportunities are enabled", e))?;
    success_json(&data)
}

/// Get opportunities by matching a unique lead field (e.g. Mobile, EmailAddress).
pub async fn get_opportunities_by_lead_field(
    client: &LsqClient,
    params: &GetOpportunitiesByLeadFieldParams,
) -> Result<CallToolResult, ErrorData> {
    let page_index = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(25).min(100);
    let op = params.operator.as_deref().unwrap_or("=");
    let columns = params.columns.as_deref().unwrap_or("");

    let body = json!({
        "Parameter": {
            "LookupName": params.lookup_name,
            "LookupValue": params.lookup_value,
            "SqlOperator": op
        },
        "Columns": { "Include_CSV": columns },
        "Paging": { "PageIndex": page_index, "PageSize": page_size }
    });

    let data: Value = client
        .post("/OpportunityManagement.svc/GetOpportunitiesByUniqueLeadField", &body)
        .await
        .map_err(|e| api_error("Failed to fetch opportunities by lead field", e))?;
    success_json(&data)
}

/// Get activities logged on an opportunity (unconfirmed path — update if 404).
pub async fn get_activities_of_opportunity(
    client: &LsqClient,
    params: &OpportunityIdParam,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get_with_params(
            "/OpportunityManagement.svc/GetActivitiesOfOpportunity",
            &[("opportunityId", params.opportunity_id.as_str())],
        )
        .await
        .map_err(|e| api_error("Failed to fetch activities of opportunity", e))?;
    success_json(&data)
}
