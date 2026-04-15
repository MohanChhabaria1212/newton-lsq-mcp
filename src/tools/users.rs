use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::{json, Value};

use crate::client::LsqClient;
use crate::models::{
    AvailabilityParams, CheckInHistoryParams, SearchUsersParams, UserHierarchyParams, UserIdParam,
};
use crate::server::{api_error, success_json};

pub async fn get_users(client: &LsqClient) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get("/UserManagement.svc/GetAll?pageIndex=0&pageSize=200")
        .await
        .map_err(|e| api_error("Failed to fetch users", e))?;
    success_json(&data)
}

pub async fn get_user_by_id(
    client: &LsqClient,
    params: &UserIdParam,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!(
            "/UserManagement.svc/GetById?userId={}",
            params.user_id
        ))
        .await
        .map_err(|e| api_error("Failed to fetch user by ID", e))?;
    success_json(&data)
}

pub async fn search_users(
    client: &LsqClient,
    params: &SearchUsersParams,
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
        .post("/UserManagement.svc/Search", &body)
        .await
        .map_err(|e| api_error("Failed to search users", e))?;
    success_json(&data)
}

pub async fn get_user_hierarchy(
    client: &LsqClient,
    params: &UserHierarchyParams,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!(
            "/UserManagement.svc/GetHierarchy?managerId={}",
            params.manager_id
        ))
        .await
        .map_err(|e| api_error("Failed to fetch user hierarchy", e))?;
    success_json(&data)
}

pub async fn get_user_checkin_history(
    client: &LsqClient,
    params: &CheckInHistoryParams,
) -> Result<CallToolResult, ErrorData> {
    let mut query = format!("userId={}", params.user_id);
    if let Some(ref from) = params.from_date {
        query.push_str(&format!("&fromDate={}", from));
    }
    if let Some(ref to) = params.to_date {
        query.push_str(&format!("&toDate={}", to));
    }

    let data: Value = client
        .get(&format!("/UserManagement.svc/GetCheckInHistory?{}", query))
        .await
        .map_err(|e| api_error("Failed to fetch check-in history", e))?;
    success_json(&data)
}

pub async fn get_user_availability(
    client: &LsqClient,
    params: &AvailabilityParams,
) -> Result<CallToolResult, ErrorData> {
    let mut query = String::new();
    if let Some(ref user_id) = params.user_id {
        query.push_str(&format!("userId={}", user_id));
    }
    if let Some(ref email) = params.email {
        if !query.is_empty() {
            query.push('&');
        }
        query.push_str(&format!("email={}", email));
    }

    let data: Value = client
        .get(&format!("/UserManagement.svc/GetAvailability?{}", query))
        .await
        .map_err(|e| api_error("Failed to fetch user availability", e))?;
    success_json(&data)
}
