use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::Value;

use crate::client::LsqClient;
use crate::models::SalesActivitiesByLeadParams;
use crate::server::{api_error, success_json};

pub async fn get_products(client: &LsqClient) -> Result<CallToolResult, ErrorData> {
    let data = client
        .get_products_cached()
        .await
        .map_err(|e| api_error("Failed to fetch products", e))?;
    success_json(&data)
}

pub async fn get_sales_activity_types(client: &LsqClient) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get("/SalesActivity.svc/RetrieveSetting")
        .await
        .map_err(|e| api_error("Failed to fetch sales activity types", e))?;
    success_json(&data)
}

pub async fn get_sales_activities_by_lead(
    client: &LsqClient,
    params: &SalesActivitiesByLeadParams,
) -> Result<CallToolResult, ErrorData> {
    let page_index = params.page.unwrap_or(1).saturating_sub(1);
    let page_size = params.page_size.unwrap_or(25).min(100);

    let data: Value = client
        .get(&format!(
            "/SalesActivity.svc/RetrieveByLeadId?leadId={}&pageIndex={}&pageSize={}",
            params.lead_id, page_index, page_size
        ))
        .await
        .map_err(|e| api_error("Failed to fetch sales activities by lead", e))?;
    success_json(&data)
}
