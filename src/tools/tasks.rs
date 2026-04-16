use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::{json, Value};

use crate::client::LsqClient;
use crate::models::{AppointmentParams, TasksByLeadParams, TasksByOwnerParams};
use crate::server::{api_error, success_json};

pub async fn get_task_types(client: &LsqClient) -> Result<CallToolResult, ErrorData> {
    let data = client
        .get_task_types_cached()
        .await
        .map_err(|e| api_error("Failed to fetch task types", e))?;
    success_json(&data)
}

pub async fn get_tasks_by_lead(
    client: &LsqClient,
    params: &TasksByLeadParams,
) -> Result<CallToolResult, ErrorData> {
    let page_index = params.page.unwrap_or(1).saturating_sub(1);
    let page_size = params.page_size.unwrap_or(25).min(100);

    let data: Value = client
        .get(&format!(
            "/LeadManagement.svc/RetrieveTaskByLeadId?leadId={}&pageIndex={}&pageSize={}",
            params.lead_id, page_index, page_size
        ))
        .await
        .map_err(|e| api_error("Failed to fetch tasks by lead", e))?;
    success_json(&data)
}

pub async fn get_tasks_by_owner(
    client: &LsqClient,
    params: &TasksByOwnerParams,
) -> Result<CallToolResult, ErrorData> {
    let page_index = params.page.unwrap_or(1).saturating_sub(1);
    let page_size = params.page_size.unwrap_or(25).min(100);

    let body = json!({
        "UserId": params.owner_id,
        "PageIndex": page_index,
        "PageSize": page_size
    });
    let data: Value = client
        .post("/Task.svc/Retrieve", &body)
        .await
        .map_err(|e| api_error("Failed to fetch tasks by owner", e))?;
    success_json(&data)
}

pub async fn get_appointments(
    client: &LsqClient,
    params: &AppointmentParams,
) -> Result<CallToolResult, ErrorData> {
    let page_index = params.page.unwrap_or(1).saturating_sub(1);
    let page_size = params.page_size.unwrap_or(25).min(100);

    let mut query = format!("pageIndex={}&pageSize={}", page_index, page_size);
    if let Some(ref user_id) = params.user_id {
        query.push_str(&format!("&userId={}", user_id));
    }
    if let Some(ref email) = params.email {
        query.push_str(&format!("&email={}", email));
    }

    let data: Value = client
        .get(&format!("/Task.svc/RetrieveAppointments?{}", query))
        .await
        .map_err(|e| api_error("Failed to fetch appointments", e))?;
    success_json(&data)
}

pub async fn get_todos(
    client: &LsqClient,
    params: &AppointmentParams,
) -> Result<CallToolResult, ErrorData> {
    let page_index = params.page.unwrap_or(1).saturating_sub(1);
    let page_size = params.page_size.unwrap_or(25).min(100);

    let mut query = format!("pageIndex={}&pageSize={}", page_index, page_size);
    if let Some(ref user_id) = params.user_id {
        query.push_str(&format!("&userId={}", user_id));
    }
    if let Some(ref email) = params.email {
        query.push_str(&format!("&email={}", email));
    }

    let data: Value = client
        .get(&format!("/Task.svc/RetrieveToDos?{}", query))
        .await
        .map_err(|e| api_error("Failed to fetch todos", e))?;
    success_json(&data)
}
