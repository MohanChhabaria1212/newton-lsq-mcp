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
        .get("/UserManagement.svc/Users.Get")
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
            "/UserManagement.svc/User/Retrieve/ByUserId?userId={}",
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
        .post("/UserManagement.svc/User/AdvancedSearch", &body)
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
            "/UserManagement.svc/ReportingHierarchy/RetrieveAllReportingUsers?UserId={}",
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
    let mut body = json!({
        "UserIds": [params.user_id]
    });
    if let Some(ref from) = params.from_date {
        body["FromDate"] = json!(from);
    }
    if let Some(ref to) = params.to_date {
        body["ToDate"] = json!(to);
    }

    let data: Value = client
        .post("/UserManagement.svc/User/GetCheckinCheckoutHistory", &body)
        .await
        .map_err(|e| api_error("Failed to fetch check-in history", e))?;
    success_json(&data)
}

pub async fn get_user_availability(
    client: &LsqClient,
    params: &AvailabilityParams,
) -> Result<CallToolResult, ErrorData> {
    // Use ByUserId endpoint when user_id provided; ByUserSearchCriteria when only email given
    let data: Value = if let Some(ref user_id) = params.user_id {
        let body = json!({ "UserIds": [user_id] });
        client
            .post("/Task.svc/RetrieveAvailableSlots/ByUserId", &body)
            .await
            .map_err(|e| api_error("Failed to fetch user availability", e))?
    } else if let Some(ref email) = params.email {
        let body = json!({ "EmailAddress": email });
        client
            .post("/Task.svc/RetrieveAvailableSlots/ByUserSearchCriteria", &body)
            .await
            .map_err(|e| api_error("Failed to fetch user availability", e))?
    } else {
        return Err(rmcp::ErrorData::new(
            rmcp::model::ErrorCode::INVALID_PARAMS,
            "Either user_id or email is required",
            None,
        ));
    };
    success_json(&data)
}
