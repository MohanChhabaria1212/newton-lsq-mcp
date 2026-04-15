use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::{json, Value};

use crate::client::LsqClient;
use crate::models::{LeadEmailParam, LeadIdParam, LeadPhoneParam, SearchLeadsParams};
use crate::server::{api_error, success_json};

pub async fn get_lead_metadata(client: &LsqClient) -> Result<CallToolResult, ErrorData> {
    let data = client
        .get_lead_metadata_cached()
        .await
        .map_err(|e| api_error("Failed to fetch lead metadata", e))?;
    success_json(&data)
}

pub async fn search_leads(
    client: &LsqClient,
    params: &SearchLeadsParams,
) -> Result<CallToolResult, ErrorData> {
    let page_index = params.page.unwrap_or(1).saturating_sub(1);
    let page_size = params.page_size.unwrap_or(25).min(100);
    let filters = params.filters.clone().unwrap_or_else(|| json!([]));

    let body = json!({
        "Filters": filters,
        "Paging": {
            "PageIndex": page_index,
            "PageSize": page_size
        }
    });

    let data: Value = client
        .post("/Leads.svc/RetrieveLeadBySearchCriteria", &body)
        .await
        .map_err(|e| api_error("Failed to search leads", e))?;

    let total = data.get("TotalCount").and_then(|v| v.as_i64()).unwrap_or(0);
    let results = data.get("RecordList").cloned().unwrap_or_else(|| json!([]));
    let count = results.as_array().map(|a| a.len() as i64).unwrap_or(0);
    let has_more = (page_index as i64 * page_size as i64 + count) < total;

    success_json(&json!({
        "results": results,
        "total_count": total,
        "page": page_index + 1,
        "page_size": page_size,
        "has_more": has_more
    }))
}

pub async fn get_lead_by_id(
    client: &LsqClient,
    params: &LeadIdParam,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!("/Leads.svc/RetrieveById?id={}", params.lead_id))
        .await
        .map_err(|e| api_error("Failed to fetch lead by ID", e))?;
    success_json(&data)
}

pub async fn get_lead_by_email(
    client: &LsqClient,
    params: &LeadEmailParam,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!(
            "/Leads.svc/RetrieveByEmailAddress?emailaddress={}",
            params.email
        ))
        .await
        .map_err(|e| api_error("Failed to fetch lead by email", e))?;
    success_json(&data)
}

pub async fn get_lead_by_phone(
    client: &LsqClient,
    params: &LeadPhoneParam,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!(
            "/Leads.svc/RetrieveByPhoneNumber?phone={}",
            params.phone
        ))
        .await
        .map_err(|e| api_error("Failed to fetch lead by phone", e))?;
    success_json(&data)
}

pub async fn get_lead_notes(
    client: &LsqClient,
    params: &LeadIdParam,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!(
            "/Notes.svc/RetrieveByLeadId?leadId={}",
            params.lead_id
        ))
        .await
        .map_err(|e| api_error("Failed to fetch lead notes", e))?;
    success_json(&data)
}

pub async fn get_lead_activities(
    client: &LsqClient,
    params: &LeadIdParam,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!(
            "/Activities.svc/RetrieveByLeadId?leadId={}",
            params.lead_id
        ))
        .await
        .map_err(|e| api_error("Failed to fetch lead activities", e))?;
    success_json(&data)
}

// ── Build helpers (unit-testable) ─────────────────────────────────────────

pub fn build_paginated_response(
    results: &Value,
    total: i64,
    page_index: u32,
    page_size: u32,
) -> Value {
    let count = results.as_array().map(|a| a.len() as i64).unwrap_or(0);
    let has_more = (page_index as i64 * page_size as i64 + count) < total;
    json!({
        "results": results,
        "total_count": total,
        "page": page_index + 1,
        "page_size": page_size,
        "has_more": has_more
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paginated_response_has_more_true() {
        let results = json!([{"id": "1"}, {"id": "2"}]);
        let resp = build_paginated_response(&results, 100, 0, 25);
        assert_eq!(resp["total_count"], 100);
        assert_eq!(resp["page"], 1);
        assert_eq!(resp["has_more"], true);
    }

    #[test]
    fn paginated_response_has_more_false_on_last_page() {
        let results = json!([{"id": "1"}]);
        let resp = build_paginated_response(&results, 1, 0, 25);
        assert_eq!(resp["has_more"], false);
    }
}
