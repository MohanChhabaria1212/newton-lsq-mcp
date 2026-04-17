use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::{json, Value};

use crate::client::LsqClient;
use crate::models::{ActivitiesByLeadParams, ActivityIdParam, RecentlyModifiedActivitiesParams};
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

/// Get full details of a single activity by its ID.
pub async fn get_activity_details(
    client: &LsqClient,
    params: &ActivityIdParam,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get_with_params(
            "/ProspectActivity.svc/GetActivityDetails",
            &[("activityId", params.activity_id.as_str())],
        )
        .await
        .map_err(|e| api_error("Failed to fetch activity details", e))?;
    success_json(&data)
}

/// Get the owner of an activity by activity ID (unconfirmed path — update if 404).
pub async fn get_activity_owner(
    client: &LsqClient,
    params: &ActivityIdParam,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get_with_params(
            "/ProspectActivity.svc/ActivityOwner.Get",
            &[("activityId", params.activity_id.as_str())],
        )
        .await
        .map_err(|e| api_error("Failed to fetch activity owner", e))?;
    success_json(&data)
}

/// Get custom activity type settings/schema (unconfirmed path — update if 404).
pub async fn get_activity_settings(client: &LsqClient) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get("/ProspectActivity.svc/CustomActivity/GetActivitySetting")
        .await
        .map_err(|e| api_error("Failed to fetch activity settings", e))?;
    success_json(&data)
}

/// Get activities modified within a date range (unconfirmed path — update if 404).
pub async fn get_recently_modified_activities(
    client: &LsqClient,
    params: &RecentlyModifiedActivitiesParams,
) -> Result<CallToolResult, ErrorData> {
    let page_index = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(25).min(100);

    let body = json!({
        "Parameter": {
            "FromDate": params.from_date,
            "ToDate": params.to_date
        },
        "Paging": { "PageIndex": page_index, "PageSize": page_size }
    });

    let data: Value = client
        .post("/ProspectActivity.svc/RetrieveRecentlyModified", &body)
        .await
        .map_err(|e| api_error("Failed to fetch recently modified activities", e))?;
    success_json(&data)
}
