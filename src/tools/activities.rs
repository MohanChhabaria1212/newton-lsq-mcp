use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::{json, Value};

use crate::client::LsqClient;
use crate::models::ActivitiesByLeadParams;
use crate::server::{api_error, success_json};

pub async fn get_activity_types(client: &LsqClient) -> Result<CallToolResult, ErrorData> {
    let data = client
        .get_activity_types_cached()
        .await
        .map_err(|e| api_error("Failed to fetch activity types", e))?;
    success_json(&data)
}

pub async fn get_activities_by_lead(
    client: &LsqClient,
    params: &ActivitiesByLeadParams,
) -> Result<CallToolResult, ErrorData> {
    let page_index = params.page.unwrap_or(1).saturating_sub(1);
    let page_size = params.page_size.unwrap_or(25).min(25); // LSQ caps activities at 25/page

    // leadId goes as a query param; pagination in body
    let body = json!({
        "Paging": {
            "PageIndex": page_index,
            "PageSize": page_size
        }
    });
    let data: Value = client
        .post(
            &format!("/ProspectActivity.svc/Retrieve?leadId={}", params.lead_id),
            &body,
        )
        .await
        .map_err(|e| api_error("Failed to fetch activities by lead", e))?;
    success_json(&data)
}
