use rmcp::model::*;
use rmcp::ErrorData;
use serde_json::{json, Value};

use crate::client::LsqClient;
use crate::models::{
    GetLeadsByIdsParams, LeadEmailParam, LeadIdParam, LeadOwnerParams, LeadPhoneParam,
    QuickSearchLeadsParams, RecentlyModifiedLeadsParams, SearchLeadsParams,
};
use crate::server::{api_error, success_json, success_json_opt};

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
    // LSQ PageIndex is 1-based
    let page_index = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(25).min(1000);

    // Build Parameter: single-field lookup or empty object for all leads
    let parameter = match (&params.lookup_name, &params.lookup_value) {
        (Some(name), Some(value)) if !name.is_empty() => {
            let op = params.operator.as_deref().unwrap_or("=");
            json!({
                "LookupName": name,
                "LookupValue": value,
                "SqlOperator": op
            })
        }
        _ => json!({}),
    };

    let body = json!({
        "Parameter": parameter,
        "Paging": {
            "PageIndex": page_index,
            "PageSize": page_size
        }
    });

    // Response is a direct JSON array of lead objects
    let results: Value = client
        .post("/LeadManagement.svc/Leads.Get", &body)
        .await
        .map_err(|e| api_error("Failed to search leads", e))?;

    let count = results.as_array().map(|a| a.len()).unwrap_or(0);
    // No TotalCount in response; infer has_more from a full page
    let has_more = count == page_size as usize;

    success_json_opt(
        &json!({ "results": results, "count": count, "page": page_index, "page_size": page_size, "has_more": has_more }),
        params.output_file.as_deref(),
    )
}

pub async fn get_lead_by_id(
    client: &LsqClient,
    params: &LeadIdParam,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get(&format!("/LeadManagement.svc/Leads.GetById?id={}", params.lead_id))
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
            "/LeadManagement.svc/Leads.GetByEmailaddress?emailaddress={}",
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
            "/LeadManagement.svc/RetrieveLeadByPhoneNumber?phone={}",
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
    let body = json!({
        "Parameter": {
            "RelatedId": params.lead_id,
            "RelatedEntityTypeId": 1
        }
    });
    let data: Value = client
        .post("/LeadManagement.svc/RetrieveNote", &body)
        .await
        .map_err(|e| api_error("Failed to fetch lead notes", e))?;
    success_json(&data)
}

pub async fn get_lead_activities(
    client: &LsqClient,
    params: &LeadIdParam,
) -> Result<CallToolResult, ErrorData> {
    // leadId goes as a query param; pagination goes in the body
    let body = json!({
        "Paging": { "PageIndex": 0, "PageSize": 100 }
    });
    let data: Value = client
        .post(
            &format!("/ProspectActivity.svc/Retrieve?leadId={}", params.lead_id),
            &body,
        )
        .await
        .map_err(|e| api_error("Failed to fetch lead activities", e))?;
    success_json(&data)
}

/// Full-text search across name, email, phone, company, city, country.
pub async fn quick_search_leads(
    client: &LsqClient,
    params: &QuickSearchLeadsParams,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get_with_params(
            "/LeadManagement.svc/Leads.GetByQuickSearch",
            &[("key", params.key.as_str())],
        )
        .await
        .map_err(|e| api_error("Failed to quick-search leads", e))?;
    success_json_opt(&data, params.output_file.as_deref())
}

/// Bulk-fetch leads by a list of ProspectIDs.
pub async fn get_leads_by_ids(
    client: &LsqClient,
    params: &GetLeadsByIdsParams,
) -> Result<CallToolResult, ErrorData> {
    let page_index = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(25).min(1000);
    let columns = params.columns.as_deref().unwrap_or("");

    let body = json!({
        "SearchParameters": { "LeadIds": params.lead_ids },
        "Columns": { "Include_CSV": columns },
        "Paging": { "PageIndex": page_index, "PageSize": page_size }
    });

    let data: Value = client
        .post("/LeadManagement.svc/Leads/Retrieve/ByIds", &body)
        .await
        .map_err(|e| api_error("Failed to fetch leads by IDs", e))?;
    success_json_opt(&data, params.output_file.as_deref())
}

/// Get the owner of a lead by any unique field (e.g. EmailAddress, LeadId, Phone).
pub async fn get_lead_owner(
    client: &LsqClient,
    params: &LeadOwnerParams,
) -> Result<CallToolResult, ErrorData> {
    let data: Value = client
        .get_with_params(
            "/LeadManagement.svc/LeadOwner.Get",
            &[
                ("LeadIdentifier", params.lead_identifier.as_str()),
                ("value", params.value.as_str()),
            ],
        )
        .await
        .map_err(|e| api_error("Failed to fetch lead owner", e))?;
    success_json(&data)
}

/// Get leads modified within a date range.
pub async fn get_recently_modified_leads(
    client: &LsqClient,
    params: &RecentlyModifiedLeadsParams,
) -> Result<CallToolResult, ErrorData> {
    let page_index = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(100).min(1000);
    let columns = params.columns.as_deref().unwrap_or("ProspectID,FirstName,LastName,EmailAddress,ModifiedOn");

    let body = json!({
        "Parameter": {
            "FromDate": params.from_date,
            "ToDate": params.to_date
        },
        "Columns": { "Include_CSV": columns },
        "Paging": { "PageIndex": page_index, "PageSize": page_size }
    });

    let data: Value = client
        .post("/LeadManagement.svc/Leads.RecentlyModified", &body)
        .await
        .map_err(|e| api_error("Failed to fetch recently modified leads", e))?;
    success_json_opt(&data, params.output_file.as_deref())
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
