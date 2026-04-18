//! Integration tests for lsq-mcp tool functions.
//!
//! Each test spins up a wiremock HTTP server, builds an `LsqClient` pointing
//! at it, calls a tool function directly, and asserts on the returned
//! `CallToolResult` or `ErrorData`.
//!
//! Tests that write output files acquire `ENV_MUTEX` and use a `TempDir`
//! as `LSQ_MCP_HOME` to avoid touching the real filesystem.

use rmcp::model::CallToolResult;
use serde_json::{json, Value};
use tempfile::TempDir;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::Credentials;
use crate::client::LsqClient;
use crate::models::*;
use crate::tools::{activities, analytics, leads, lists, opportunities, sales, tasks, users};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn test_creds() -> Credentials {
    Credentials {
        access_key: "ak-test".into(),
        secret_key: "sk-test".into(),
        host: "unused.example.com".into(),
        user_name: None,
        user_email: None,
        user_role: None,
    }
}

fn make_client(server: &MockServer) -> LsqClient {
    LsqClient::new_for_testing(test_creds(), &server.uri())
}

/// Set LSQ_MCP_HOME to a temp directory. The MutexGuard must be kept alive
/// for the duration of the test to prevent concurrent env-var mutation.
fn temp_home() -> (TempDir, std::sync::MutexGuard<'static, ()>) {
    let guard = crate::ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let dir = TempDir::new().expect("tempdir creation failed");
    // SAFETY: ENV_MUTEX serialises all tests that touch this env var.
    unsafe { std::env::set_var("LSQ_MCP_HOME", dir.path()); }
    (dir, guard)
}

/// Extract and parse the JSON payload from the first text content item.
fn result_json(result: &CallToolResult) -> Value {
    let raw = serde_json::to_value(result).expect("CallToolResult should be serialisable");
    let text = raw["content"][0]["text"]
        .as_str()
        .expect("first content item should be text");
    serde_json::from_str(text).expect("text content should be valid JSON")
}

// ── Leads ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn search_leads_no_filter_returns_results() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/LeadManagement.svc/Leads.Get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"ProspectID": "id-1", "FirstName": "Alice"},
            {"ProspectID": "id-2", "FirstName": "Bob"},
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let params = SearchLeadsParams {
        lookup_name: None, lookup_value: None, operator: None,
        page: None, page_size: None, output_file: None,
    };
    let result = leads::search_leads(&client, &params).await.expect("happy path");
    let json = result_json(&result);
    assert_eq!(json["count"], 2);
    assert_eq!(json["results"][0]["FirstName"], "Alice");
    assert_eq!(json["has_more"], false);
}

#[tokio::test]
async fn search_leads_with_filter_sends_correct_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/LeadManagement.svc/Leads.Get"))
        .and(body_json(json!({
            "Parameter": {
                "LookupName": "EmailAddress",
                "LookupValue": "alice@example.com",
                "SqlOperator": "="
            },
            "Paging": {"PageIndex": 1, "PageSize": 25}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"ProspectID": "id-1", "EmailAddress": "alice@example.com"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let params = SearchLeadsParams {
        lookup_name: Some("EmailAddress".into()),
        lookup_value: Some("alice@example.com".into()),
        operator: Some("=".into()),
        page: None, page_size: None, output_file: None,
    };
    let json = result_json(&leads::search_leads(&client, &params).await.expect("happy path"));
    assert_eq!(json["count"], 1);
    assert_eq!(json["results"][0]["EmailAddress"], "alice@example.com");
}

#[tokio::test]
async fn search_leads_has_more_true_when_full_page() {
    let server = MockServer::start().await;
    // Return exactly page_size (3) items → has_more must be true
    Mock::given(method("POST"))
        .and(path("/v2/LeadManagement.svc/Leads.Get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"ProspectID": "a"}, {"ProspectID": "b"}, {"ProspectID": "c"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(&leads::search_leads(&client, &SearchLeadsParams {
        lookup_name: None, lookup_value: None, operator: None,
        page: Some(1), page_size: Some(3), output_file: None,
    }).await.expect("happy path"));
    assert_eq!(json["has_more"], true);
    assert_eq!(json["count"], 3);
}

#[tokio::test]
async fn search_leads_has_more_false_when_partial_page() {
    let server = MockServer::start().await;
    // Return 2 items with page_size=3 → has_more must be false
    Mock::given(method("POST"))
        .and(path("/v2/LeadManagement.svc/Leads.Get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"ProspectID": "a"}, {"ProspectID": "b"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(&leads::search_leads(&client, &SearchLeadsParams {
        lookup_name: None, lookup_value: None, operator: None,
        page: Some(1), page_size: Some(3), output_file: None,
    }).await.expect("happy path"));
    assert_eq!(json["has_more"], false);
    assert_eq!(json["count"], 2);
}

#[tokio::test]
async fn search_leads_empty_results() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/LeadManagement.svc/Leads.Get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(&leads::search_leads(&client, &SearchLeadsParams {
        lookup_name: None, lookup_value: None, operator: None,
        page: None, page_size: None, output_file: None,
    }).await.expect("happy path"));
    assert_eq!(json["count"], 0);
    assert_eq!(json["has_more"], false);
    assert!(json["results"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn get_lead_metadata_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/LeadManagement.svc/LeadsMetaData.Get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"SchemaName": "EmailAddress", "DataType": "Text"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(&leads::get_lead_metadata(&client).await.expect("happy path"));
    assert_eq!(json[0]["SchemaName"], "EmailAddress");
}

#[tokio::test]
async fn get_lead_metadata_cached_after_first_call() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/LeadManagement.svc/LeadsMetaData.Get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"SchemaName": "Phone"}])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let _ = leads::get_lead_metadata(&client).await.expect("first call");
    let _ = leads::get_lead_metadata(&client).await.expect("second call (cached)");

    let reqs = server.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1, "metadata endpoint should be hit exactly once");
}

#[tokio::test]
async fn get_lead_by_id_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/LeadManagement.svc/Leads.GetById"))
        .and(query_param("id", "lead-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!(
            {"ProspectID": "lead-abc", "FirstName": "Alice"}
        )))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &leads::get_lead_by_id(&client, &LeadIdParam { lead_id: "lead-abc".into() })
            .await.expect("happy path"),
    );
    assert_eq!(json["ProspectID"], "lead-abc");
}

#[tokio::test]
async fn get_lead_by_email_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/LeadManagement.svc/Leads.GetByEmailaddress"))
        .and(query_param("emailaddress", "alice@example.com"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!(
            {"ProspectID": "id-1", "EmailAddress": "alice@example.com"}
        )))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &leads::get_lead_by_email(&client, &LeadEmailParam { email: "alice@example.com".into() })
            .await.expect("happy path"),
    );
    assert_eq!(json["EmailAddress"], "alice@example.com");
}

#[tokio::test]
async fn get_lead_by_phone_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/LeadManagement.svc/RetrieveLeadByPhoneNumber"))
        .and(query_param("phone", "9876543210"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!(
            {"ProspectID": "id-2", "Phone": "9876543210"}
        )))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &leads::get_lead_by_phone(&client, &LeadPhoneParam { phone: "9876543210".into() })
            .await.expect("happy path"),
    );
    assert_eq!(json["Phone"], "9876543210");
}

#[tokio::test]
async fn get_lead_notes_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/LeadManagement.svc/RetrieveNote"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"NoteId": "n1", "NoteText": "Follow up call"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &leads::get_lead_notes(&client, &LeadIdParam { lead_id: "lead-abc".into() })
            .await.expect("happy path"),
    );
    assert_eq!(json[0]["NoteText"], "Follow up call");
}

#[tokio::test]
async fn get_lead_activities_sends_lead_id_as_query_param() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/ProspectActivity.svc/Retrieve"))
        .and(query_param("leadId", "lead-xyz"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"ActivityId": "act-1", "ActivityEvent": "Phone Call"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &leads::get_lead_activities(&client, &LeadIdParam { lead_id: "lead-xyz".into() })
            .await.expect("happy path"),
    );
    assert_eq!(json[0]["ActivityEvent"], "Phone Call");
}

#[tokio::test]
async fn quick_search_leads_sends_key_param() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/LeadManagement.svc/Leads.GetByQuickSearch"))
        .and(query_param("key", "alice"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"ProspectID": "id-1", "FirstName": "Alice"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &leads::quick_search_leads(&client, &QuickSearchLeadsParams {
            key: "alice".into(), output_file: None,
        }).await.expect("happy path"),
    );
    assert_eq!(json[0]["FirstName"], "Alice");
}

#[tokio::test]
async fn get_leads_by_ids_sends_ids_in_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/LeadManagement.svc/Leads/Retrieve/ByIds"))
        .and(body_json(json!({
            "SearchParameters": {"LeadIds": ["id-1", "id-2"]},
            "Columns": {"Include_CSV": ""},
            "Paging": {"PageIndex": 1, "PageSize": 25}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"ProspectID": "id-1"}, {"ProspectID": "id-2"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &leads::get_leads_by_ids(&client, &GetLeadsByIdsParams {
            lead_ids: vec!["id-1".into(), "id-2".into()],
            columns: None, page: None, page_size: None, output_file: None,
        }).await.expect("happy path"),
    );
    assert_eq!(json.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn get_lead_owner_sends_identifier_and_value() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/LeadManagement.svc/LeadOwner.Get"))
        .and(query_param("LeadIdentifier", "EmailAddress"))
        .and(query_param("value", "alice@example.com"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!(
            {"UserId": "user-1", "UserName": "Bob"}
        )))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &leads::get_lead_owner(&client, &LeadOwnerParams {
            lead_identifier: "EmailAddress".into(),
            value: "alice@example.com".into(),
        }).await.expect("happy path"),
    );
    assert_eq!(json["UserName"], "Bob");
}

#[tokio::test]
async fn get_recently_modified_leads_sends_date_range() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/LeadManagement.svc/Leads.RecentlyModified"))
        .and(body_json(json!({
            "Parameter": {
                "FromDate": "2026-01-01 00:00:00",
                "ToDate": "2026-01-31 23:59:59"
            },
            "Columns": {"Include_CSV": "ProspectID,FirstName,LastName,EmailAddress,ModifiedOn"},
            "Paging": {"PageIndex": 1, "PageSize": 100}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"ProspectID": "id-1", "ModifiedOn": "2026-01-15 10:00:00"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &leads::get_recently_modified_leads(&client, &RecentlyModifiedLeadsParams {
            from_date: "2026-01-01 00:00:00".into(),
            to_date: "2026-01-31 23:59:59".into(),
            columns: None, page: None, page_size: None, output_file: None,
        }).await.expect("happy path"),
    );
    assert_eq!(json[0]["ProspectID"], "id-1");
}

// ── Opportunities ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_opportunity_types_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/OpportunityManagement.svc/GetOpportunityTypes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"OpportunityEventTypeId": 1, "Name": "Sales"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(&opportunities::get_opportunity_types(&client).await.expect("happy path"));
    assert_eq!(json[0]["Name"], "Sales");
}

#[tokio::test]
async fn get_opportunity_types_cached() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/OpportunityManagement.svc/GetOpportunityTypes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"Name": "Renewal"}])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let _ = opportunities::get_opportunity_types(&client).await.expect("first call");
    let _ = opportunities::get_opportunity_types(&client).await.expect("second call");

    assert_eq!(
        server.received_requests().await.unwrap().len(), 1,
        "opportunity types should be cached after first call"
    );
}

#[tokio::test]
async fn get_opportunity_by_id_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/OpportunityManagement.svc/GetOpportunityDetails"))
        .and(query_param("OpportunityId", "opp-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!(
            {"OpportunityId": "opp-1", "Stage": "Proposal"}
        )))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &opportunities::get_opportunity_by_id(&client, &OpportunityIdParam {
            opportunity_id: "opp-1".into(),
        }).await.expect("happy path"),
    );
    assert_eq!(json["Stage"], "Proposal");
}

#[tokio::test]
async fn get_opportunities_by_lead_sends_lead_id() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/OpportunityManagement.svc/GetOpportunitiesOfLead"))
        .and(query_param("leadId", "lead-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"OpportunityId": "opp-1"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &opportunities::get_opportunities_by_lead(&client, &LeadIdParam { lead_id: "lead-1".into() })
            .await.expect("happy path"),
    );
    assert_eq!(json[0]["OpportunityId"], "opp-1");
}

#[tokio::test]
async fn search_opportunities_returns_paginated_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/OpportunityManagement.svc/Retrieve/BySearchParameter"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!(
            {"RecordCount": 5, "List": [{"OpportunityId": "opp-1"}]}
        )))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &opportunities::search_opportunities(&client, &SearchOpportunitiesParams {
            opportunity_type_code: None, advanced_search: None,
            page: None, page_size: None, output_file: None,
        }).await.expect("happy path"),
    );
    assert_eq!(json["total_count"], 5);
    assert_eq!(json["results"][0]["OpportunityId"], "opp-1");
}

// ── Activities ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_activity_types_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/ProspectActivity.svc/ActivityTypes.Get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"ActivityEventId": 1, "Name": "Phone Call"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(&activities::get_activity_types(&client).await.expect("happy path"));
    assert_eq!(json[0]["Name"], "Phone Call");
}

#[tokio::test]
async fn get_activity_types_cached() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/ProspectActivity.svc/ActivityTypes.Get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"Name": "Email"}])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let _ = activities::get_activity_types(&client).await.expect("first call");
    let _ = activities::get_activity_types(&client).await.expect("second call");

    assert_eq!(
        server.received_requests().await.unwrap().len(), 1,
        "activity types should be cached"
    );
}

#[tokio::test]
async fn get_activities_by_lead_sends_lead_id_as_query_param() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/ProspectActivity.svc/Retrieve"))
        .and(query_param("leadId", "lead-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"ActivityId": "act-1", "ActivityEvent": "Demo"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &activities::get_activities_by_lead(&client, &ActivitiesByLeadParams {
            lead_id: "lead-abc".into(), page: None, page_size: None, output_file: None,
        }).await.expect("happy path"),
    );
    assert_eq!(json[0]["ActivityEvent"], "Demo");
}

#[tokio::test]
async fn get_recently_modified_activities_sends_date_range() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/ProspectActivity.svc/RetrieveRecentlyModified"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"ActivityId": "act-99"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &activities::get_recently_modified_activities(&client, &RecentlyModifiedActivitiesParams {
            from_date: "2026-01-01 00:00:00".into(),
            to_date: "2026-01-31 23:59:59".into(),
            page: None, page_size: None, output_file: None,
        }).await.expect("happy path"),
    );
    assert_eq!(json[0]["ActivityId"], "act-99");
}

// ── Sales ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_products_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/SalesActivity.svc/Product/GetAll"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"ProductId": "p1", "Name": "Enterprise"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(&sales::get_products(&client).await.expect("happy path"));
    assert_eq!(json[0]["Name"], "Enterprise");
}

#[tokio::test]
async fn get_products_cached() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/SalesActivity.svc/Product/GetAll"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"Name": "Basic"}])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let _ = sales::get_products(&client).await.expect("first call");
    let _ = sales::get_products(&client).await.expect("second call");

    assert_eq!(
        server.received_requests().await.unwrap().len(), 1,
        "products should be cached"
    );
}

#[tokio::test]
async fn get_sales_activities_by_lead_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/SalesActivity.svc/RetrieveByLeadId"))
        .and(query_param("leadId", "lead-s1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"SaleId": "sale-1", "Amount": 5000}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &sales::get_sales_activities_by_lead(&client, &SalesActivitiesByLeadParams {
            lead_id: "lead-s1".into(), page: None, page_size: None, output_file: None,
        }).await.expect("happy path"),
    );
    assert_eq!(json[0]["SaleId"], "sale-1");
}

// ── Tasks ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_task_types_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/Task.svc/TaskType/GetAll"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"TaskTypeId": 1, "Name": "Call"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(&tasks::get_task_types(&client).await.expect("happy path"));
    assert_eq!(json[0]["Name"], "Call");
}

#[tokio::test]
async fn get_task_types_cached() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/Task.svc/TaskType/GetAll"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"Name": "Meeting"}])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let _ = tasks::get_task_types(&client).await.expect("first call");
    let _ = tasks::get_task_types(&client).await.expect("second call");

    assert_eq!(
        server.received_requests().await.unwrap().len(), 1,
        "task types should be cached"
    );
}

#[tokio::test]
async fn get_tasks_by_lead_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/LeadManagement.svc/RetrieveTaskByLeadId"))
        .and(query_param("leadId", "lead-t1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"TaskId": "task-1", "TaskType": "Call"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &tasks::get_tasks_by_lead(&client, &TasksByLeadParams {
            lead_id: "lead-t1".into(), page: None, page_size: None, output_file: None,
        }).await.expect("happy path"),
    );
    assert_eq!(json[0]["TaskId"], "task-1");
}

#[tokio::test]
async fn get_tasks_by_owner_sends_owner_id_in_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/Task.svc/Retrieve"))
        .and(body_json(json!({
            "UserId": "user-42",
            "PageIndex": 0,
            "PageSize": 25
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"TaskId": "task-2", "OwnerId": "user-42"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &tasks::get_tasks_by_owner(&client, &TasksByOwnerParams {
            owner_id: "user-42".into(), page: None, page_size: None, output_file: None,
        }).await.expect("happy path"),
    );
    assert_eq!(json[0]["OwnerId"], "user-42");
}

#[tokio::test]
async fn get_appointments_sends_user_id() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/Task.svc/RetrieveAppointments"))
        .and(query_param("userId", "user-5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"TaskId": "appt-1", "StartDateTime": "2026-04-20 10:00:00"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &tasks::get_appointments(&client, &AppointmentParams {
            user_id: Some("user-5".into()), email: None,
            page: None, page_size: None, output_file: None,
        }).await.expect("happy path"),
    );
    assert_eq!(json[0]["TaskId"], "appt-1");
}

#[tokio::test]
async fn get_todos_sends_email_param() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/Task.svc/RetrieveToDos"))
        .and(query_param("email", "rep@corp.com"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"TaskId": "todo-1", "DueDate": "2026-04-21"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &tasks::get_todos(&client, &AppointmentParams {
            user_id: None, email: Some("rep@corp.com".into()),
            page: None, page_size: None, output_file: None,
        }).await.expect("happy path"),
    );
    assert_eq!(json[0]["TaskId"], "todo-1");
}

// ── Users ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_users_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/UserManagement.svc/Users.Get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"UserId": "u1", "UserName": "Alice"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &users::get_users(&client, &GetUsersParams { output_file: None })
            .await.expect("happy path"),
    );
    assert_eq!(json[0]["UserName"], "Alice");
}

#[tokio::test]
async fn get_user_by_id_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/UserManagement.svc/User/Retrieve/ByUserId"))
        .and(query_param("userId", "u-99"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!(
            {"UserId": "u-99", "EmailAddress": "rep@corp.com"}
        )))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &users::get_user_by_id(&client, &UserIdParam { user_id: "u-99".into() })
            .await.expect("happy path"),
    );
    assert_eq!(json["UserId"], "u-99");
}

#[tokio::test]
async fn search_users_sends_filters_in_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/UserManagement.svc/User/AdvancedSearch"))
        .and(body_json(json!({
            "Filters": [{"Attribute": "Role", "Value": "Admin"}],
            "Paging": {"PageIndex": 0, "PageSize": 25}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"UserId": "u-admin"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &users::search_users(&client, &SearchUsersParams {
            filters: Some(json!([{"Attribute": "Role", "Value": "Admin"}])),
            page: None, page_size: None, output_file: None,
        }).await.expect("happy path"),
    );
    assert_eq!(json[0]["UserId"], "u-admin");
}

#[tokio::test]
async fn get_user_hierarchy_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/UserManagement.svc/ReportingHierarchy/RetrieveAllReportingUsers"))
        .and(query_param("UserId", "mgr-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"UserId": "rep-1"}, {"UserId": "rep-2"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &users::get_user_hierarchy(&client, &UserHierarchyParams { manager_id: "mgr-1".into() })
            .await.expect("happy path"),
    );
    assert_eq!(json.as_array().unwrap().len(), 2);
}

// ── Lists ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_lists_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/LeadManagement.svc/Lists.Get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"ListId": "list-1", "ListName": "Hot Leads"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(&lists::get_lists(&client).await.expect("happy path"));
    assert_eq!(json[0]["ListName"], "Hot Leads");
}

#[tokio::test]
async fn get_leads_in_list_sends_pagination_params() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/LeadManagement.svc/List.GetLeads"))
        .and(query_param("listId", "list-1"))
        .and(query_param("pageIndex", "0"))
        .and(query_param("pageSize", "25"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"ProspectID": "lead-1"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &lists::get_leads_in_list(&client, &GetLeadsInListParams {
            list_id: "list-1".into(), page: None, page_size: None, output_file: None,
        }).await.expect("happy path"),
    );
    assert_eq!(json[0]["ProspectID"], "lead-1");
}

#[tokio::test]
async fn get_lead_list_memberships_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/List.svc/GetByLeadId"))
        .and(query_param("leadId", "lead-m1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"ListId": "list-1"}, {"ListId": "list-2"}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &lists::get_lead_list_memberships(&client, &LeadListMembershipsParam {
            lead_id: "lead-m1".into(),
        }).await.expect("happy path"),
    );
    assert_eq!(json.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn get_list_lead_count_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/List.svc/GetLeadCount"))
        .and(query_param("listId", "list-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"Count": 342})))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &lists::get_list_lead_count(&client, &ListIdParam {
            list_id: "list-1".into(), page: None, page_size: None,
        }).await.expect("happy path"),
    );
    assert_eq!(json["Count"], 342);
}

// ── Analytics (no /v2 prefix — uses analytics_base) ──────────────────────────

#[tokio::test]
async fn get_lead_distribution_uses_analytics_url() {
    let server = MockServer::start().await;
    // Analytics base URL has no /v2 — path is registered without it
    Mock::given(method("POST"))
        .and(path("/Leads/LeadDistribution/FilterByLeadField"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"Stage": "New", "Count": 120}
        ])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &analytics::get_lead_distribution(&client, &LeadDistributionParams {
            filters: json!({"Aggregate": "Stage"}),
        }).await.expect("happy path"),
    );
    assert_eq!(json[0]["Stage"], "New");
}

#[tokio::test]
async fn get_leads_not_contacted_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Leads/LeadsNotContacted"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!(
            {"Leads": [{"ProspectID": "lead-nc"}], "TotalCount": 1}
        )))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &analytics::get_leads_not_contacted(&client, &LeadsNotContactedParams {
            filters: json!({}),
        }).await.expect("happy path"),
    );
    assert_eq!(json["TotalCount"], 1);
}

#[tokio::test]
async fn get_leads_no_active_tasks_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Leads/LeadsWithNoActiveTasks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"TotalCount": 55})))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &analytics::get_leads_no_active_tasks(&client, &LeadsNoActiveTasksParams {
            filters: json!({}),
        }).await.expect("happy path"),
    );
    assert_eq!(json["TotalCount"], 55);
}

#[tokio::test]
async fn get_leads_pending_tasks_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Leads/LeadsWithPendingTasks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"TotalCount": 12})))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let json = result_json(
        &analytics::get_leads_pending_tasks(&client, &LeadsPendingTasksParams {
            filters: json!({"TaskFilters": {"Status": "Overdue"}}),
        }).await.expect("happy path"),
    );
    assert_eq!(json["TotalCount"], 12);
}

// ── HTTP error handling ───────────────────────────────────────────────────────

#[tokio::test]
async fn http_401_returns_unauthorized_error_data() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/LeadManagement.svc/Leads.Get"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let result = leads::search_leads(&client, &SearchLeadsParams {
        lookup_name: None, lookup_value: None, operator: None,
        page: None, page_size: None, output_file: None,
    }).await;

    assert!(result.is_err(), "401 should produce Err(ErrorData)");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("unauthorized"),
        "error message should contain 'unauthorized', got: {}",
        err.message
    );
}

#[tokio::test]
async fn http_429_retries_once_then_succeeds() {
    let server = MockServer::start().await;

    // Register 429 FIRST so it is tried first (wiremock matches in registration order).
    // up_to_n_times(1) means it is consumed after one hit.
    Mock::given(method("POST"))
        .and(path("/v2/LeadManagement.svc/Leads.Get"))
        .respond_with(
            ResponseTemplate::new(429).insert_header("Retry-After", "0")
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Register 200 SECOND — matches once 429 mock is exhausted.
    Mock::given(method("POST"))
        .and(path("/v2/LeadManagement.svc/Leads.Get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let result = leads::search_leads(&client, &SearchLeadsParams {
        lookup_name: None, lookup_value: None, operator: None,
        page: None, page_size: None, output_file: None,
    }).await;

    assert!(result.is_ok(), "should succeed after one retry; got: {:?}", result);
    let reqs = server.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 2, "expected 2 requests: 1×429 then 1×200");
}

#[tokio::test]
async fn http_429_exhausted_returns_rate_limit_error() {
    let server = MockServer::start().await;
    // Always 429 — all 4 attempts (0..=MAX_RETRIES=3) fail.
    // Retry-After: 0 keeps the test instant.
    Mock::given(method("POST"))
        .and(path("/v2/LeadManagement.svc/Leads.Get"))
        .respond_with(
            ResponseTemplate::new(429).insert_header("Retry-After", "0")
        )
        .mount(&server)
        .await;

    let client = make_client(&server);
    let result = leads::search_leads(&client, &SearchLeadsParams {
        lookup_name: None, lookup_value: None, operator: None,
        page: None, page_size: None, output_file: None,
    }).await;

    assert!(result.is_err(), "exhausted retries should produce Err");
    let err = result.unwrap_err();
    assert!(
        err.message.to_lowercase().contains("rate"),
        "error should mention rate limiting, got: {}",
        err.message
    );
    // attempt 0,1,2 sleep+retry; attempt 3 returns immediately → 4 total requests
    let reqs = server.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 4, "expected 4 requests (MAX_RETRIES=3 means attempts 0..=3)");
}

#[tokio::test]
async fn http_500_returns_error_data() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/LeadManagement.svc/Leads.Get"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let client = make_client(&server);
    let result = leads::search_leads(&client, &SearchLeadsParams {
        lookup_name: None, lookup_value: None, operator: None,
        page: None, page_size: None, output_file: None,
    }).await;

    assert!(result.is_err(), "500 should produce Err(ErrorData)");
}

// ── File output ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn explicit_output_file_written_to_output_dir() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/LeadManagement.svc/Leads.Get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"ProspectID": "id-1"}, {"ProspectID": "id-2"}
        ])))
        .mount(&server)
        .await;

    let (dir, _guard) = temp_home();
    let client = make_client(&server);
    let result = leads::search_leads(&client, &SearchLeadsParams {
        lookup_name: None, lookup_value: None, operator: None,
        page: None, page_size: None,
        output_file: Some("myresults.json".into()),
    }).await.expect("file write happy path");

    // Result should be a summary object containing the file path
    let summary = result_json(&result);
    assert!(summary.get("file").is_some(), "result should be a file summary, got: {}", summary);

    // The file must exist under {output_dir}/myresults.json
    let output_path = dir.path().join("output").join("myresults.json");
    assert!(output_path.exists(), "output file should exist at {:?}", output_path);

    // File content should contain the full results
    let written: Value = serde_json::from_str(
        &std::fs::read_to_string(&output_path).unwrap()
    ).unwrap();
    assert_eq!(written["count"], 2);
}

#[tokio::test]
async fn output_file_directory_prefix_is_stripped() {
    // Providing '/tmp/leads.json' should land in {output_dir}/leads.json — /tmp/ stripped.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/LeadManagement.svc/Leads.Get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&server)
        .await;

    let (dir, _guard) = temp_home();
    let client = make_client(&server);
    let _ = leads::search_leads(&client, &SearchLeadsParams {
        lookup_name: None, lookup_value: None, operator: None,
        page: None, page_size: None,
        output_file: Some("/tmp/stripped.json".into()),
    }).await.expect("strip-prefix path");

    // Must land in our output dir, not in /tmp/
    let output_path = dir.path().join("output").join("stripped.json");
    assert!(output_path.exists(), "directory prefix should be stripped; file should be in output_dir");
}

#[tokio::test]
async fn auto_threshold_writes_file_when_response_exceeds_100kb() {
    let server = MockServer::start().await;
    // 200 leads × ~630 bytes each ≈ 126 KB pretty-printed → triggers auto-threshold
    let big_leads: Vec<Value> = (0..200)
        .map(|i| json!({
            "ProspectID": format!("id-{:08}", i),
            "FirstName": "x".repeat(300),
            "LastName": "y".repeat(300),
        }))
        .collect();
    Mock::given(method("POST"))
        .and(path("/v2/LeadManagement.svc/Leads.Get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&big_leads))
        .mount(&server)
        .await;

    let (dir, _guard) = temp_home();
    let client = make_client(&server);
    // No explicit output_file — auto-threshold should trigger
    let result = leads::search_leads(&client, &SearchLeadsParams {
        lookup_name: None, lookup_value: None, operator: None,
        page: Some(1), page_size: Some(200), output_file: None,
    }).await.expect("auto-threshold happy path");

    let summary = result_json(&result);
    let file_path = summary["file"]
        .as_str()
        .expect("response should be a file summary when > 100 KB");

    // File must exist and live under our output dir
    assert!(
        std::path::Path::new(file_path).exists(),
        "auto-threshold file should exist at {}", file_path
    );
    assert!(
        std::path::Path::new(file_path).starts_with(dir.path().join("output")),
        "auto-threshold file should be inside the output dir"
    );

    // Sanity-check the written JSON is parseable and has the right shape
    let written: Value = serde_json::from_str(
        &std::fs::read_to_string(file_path).unwrap()
    ).unwrap();
    assert_eq!(written["count"], 200);
}

// ── Path security ─────────────────────────────────────────────────────────────

#[test]
fn validated_output_path_rejects_dotdot_traversal() {
    let (_dir, _guard) = temp_home();
    let result = crate::server::validated_output_path("../../etc/passwd");
    assert!(result.is_err(), "'..' components must be rejected");
    let msg = result.unwrap_err().message;
    assert!(msg.contains(".."), "error should mention the '..' problem, got: {}", msg);
}

#[test]
fn validated_output_path_rejects_embedded_dotdot() {
    let (_dir, _guard) = temp_home();
    let result = crate::server::validated_output_path("subdir/../../secret.txt");
    assert!(result.is_err(), "embedded '..' should be rejected");
}

#[test]
fn validated_output_path_strips_directory_prefix() {
    let (dir, _guard) = temp_home();
    let path = crate::server::validated_output_path("/tmp/leads.json")
        .expect("plain filename with directory prefix should be accepted");
    assert_eq!(path.file_name().unwrap(), "leads.json");
    assert!(
        path.starts_with(dir.path().join("output")),
        "resolved path must be inside the output dir"
    );
}

#[test]
fn validated_output_path_plain_filename_resolves_to_output_dir() {
    let (dir, _guard) = temp_home();
    let path = crate::server::validated_output_path("results.json")
        .expect("plain filename should be accepted");
    assert_eq!(path, dir.path().join("output").join("results.json"));
}

// ── Output directory cleanup ──────────────────────────────────────────────────

#[test]
fn cleanup_prunes_oldest_files_when_over_limit() {
    let (dir, _guard) = temp_home();
    let output_dir = dir.path().join("output");
    std::fs::create_dir_all(&output_dir).unwrap();

    let total = crate::config::MAX_OUTPUT_FILES + 1;
    for i in 0..total {
        std::fs::write(output_dir.join(format!("file_{:04}.json", i)), b"{}").unwrap();
    }
    assert_eq!(
        std::fs::read_dir(&output_dir).unwrap().count(), total,
        "pre-condition: should have created {} files", total
    );

    crate::config::cleanup_output_dir();

    let remaining = std::fs::read_dir(&output_dir).unwrap().count();
    assert!(
        remaining <= crate::config::MAX_OUTPUT_FILES,
        "after cleanup, at most {} files should remain; found {}",
        crate::config::MAX_OUTPUT_FILES, remaining
    );
}

#[test]
fn cleanup_does_nothing_when_at_limit() {
    let (dir, _guard) = temp_home();
    let output_dir = dir.path().join("output");
    std::fs::create_dir_all(&output_dir).unwrap();

    for i in 0..crate::config::MAX_OUTPUT_FILES {
        std::fs::write(output_dir.join(format!("file_{:04}.json", i)), b"{}").unwrap();
    }

    crate::config::cleanup_output_dir();

    let remaining = std::fs::read_dir(&output_dir).unwrap().count();
    assert_eq!(
        remaining, crate::config::MAX_OUTPUT_FILES,
        "cleanup should not delete files when at the limit"
    );
}

#[test]
fn cleanup_does_nothing_when_dir_does_not_exist() {
    let (_dir, _guard) = temp_home();
    // output_dir does NOT exist — cleanup should silently no-op
    crate::config::cleanup_output_dir();
    // If we reach here without panic, the test passes
}
