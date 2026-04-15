pub const INSTRUCTIONS: &str = r#"
LeadSquared MCP — Available Tools
═══════════════════════════════════

IMPORTANT NOTES
───────────────
• Date format: All date parameters must be UTC in "YYYY-MM-DD HH:MM:SS" format.
• Custom fields: LSQ accounts have account-specific custom fields. Call get_lead_metadata
  first to discover available field names before filtering on custom fields.
• Pagination: All list/search tools return 25 results by default (max 100).
  Check has_more and increment page to retrieve further results.
• Elasticsearch: get_leads_not_contacted, get_leads_no_active_tasks, and
  get_leads_pending_tasks require Elasticsearch to be enabled on your LSQ account.

RECOMMENDED CALL SEQUENCE
──────────────────────────
1. get_lead_metadata       — understand available lead fields (cached after first call)
2. get_activity_types      — know activity type names/IDs (cached)
3. get_task_types          — know task type names (cached)
4. get_opportunity_types   — know opportunity types (cached)
5. search_leads / get_lead_by_* — find the leads you need
6. get_lead_activities / get_lead_notes / get_opportunities_by_lead — enrich as needed

TOOLS BY MODULE
───────────────

LEADS (7 tools)
  get_lead_metadata          — field schemas, types, picklist values
  search_leads               — advanced filter search (requires filters JSON)
  get_lead_by_id             — full lead by ProspectID
  get_lead_by_email          — lookup by email
  get_lead_by_phone          — lookup by phone
  get_lead_notes             — notes on a lead
  get_lead_activities        — full activity history for a lead

OPPORTUNITIES (5 tools)
  get_opportunity_types      — all opportunity types
  get_opportunity_metadata   — field schema for an opportunity type
  get_opportunity_by_id      — single opportunity
  get_opportunities_by_lead  — all opportunities for a lead
  search_opportunities       — filtered opportunity search

ACTIVITIES (2 tools)
  get_activity_types         — all activity type definitions (cached)
  get_activities_by_lead     — activity log for a lead

SALES ACTIVITIES (3 tools)
  get_products               — product catalogue (cached)
  get_sales_activity_types   — sales activity settings
  get_sales_activities_by_lead — sales transactions for a lead

TASKS (5 tools)
  get_task_types             — all task type names (cached)
  get_tasks_by_lead          — tasks for a lead
  get_tasks_by_owner         — tasks assigned to a user
  get_appointments           — user appointments
  get_todos                  — user to-do items

USERS (6 tools)
  get_users                  — all users in the account
  get_user_by_id             — single user details
  search_users               — filtered user search
  get_user_hierarchy         — reporting chain under a manager
  get_user_checkin_history   — check-in records
  get_user_availability      — working hours and available slots

LISTS (4 tools)
  get_lists                  — all lists in the account
  get_leads_in_list          — leads in a list
  get_lead_list_memberships  — which lists a lead belongs to
  get_list_lead_count        — count of leads in a list

ANALYTICS (4 tools — require Elasticsearch)
  get_lead_distribution      — leads by owner/stage with aggregation
  get_leads_not_contacted    — leads without specified activities
  get_leads_no_active_tasks  — leads with no pending tasks
  get_leads_pending_tasks    — leads with overdue/pending tasks
"#;
