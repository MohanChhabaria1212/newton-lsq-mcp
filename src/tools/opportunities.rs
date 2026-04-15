use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::{json, Value};

use crate::client::LsqClient;
use crate::models::{LeadIdParam, OpportunityIdParam, OpportunityMetadataParams, SearchOpportunitiesParams};
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
            "/Opportunities.svc/GetMetaData?opportunityTypeId={}",
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
            "/Opportunities.svc/RetrieveById?id={}",
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
    let data: Value = client
        .get(&format!(
            "/Opportunities.svc/RetrieveByLeadId?leadId={}",
            params.lead_id
        ))
        .await
        .map_err(|e| api_error("Failed to fetch opportunities by lead", e))?;
    success_json(&data)
}

pub async fn search_opportunities(
    client: &LsqClient,
    params: &SearchOpportunitiesParams,
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
        .post("/Opportunities.svc/Search", &body)
        .await
        .map_err(|e| api_error("Failed to search opportunities", e))?;

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
