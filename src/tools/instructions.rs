pub const INSTRUCTIONS: &str = r#"
LeadSquared MCP — Available Tools
═══════════════════════════════════

STRICT CONSTRAINT — read before doing anything
───────────────────────────────────────────────
This MCP is the ONLY approved channel for accessing LeadSquared data.
• NEVER write or run Python, shell, or any other script to call LSQ APIs.
• NEVER ask the user to run curl commands, scripts, or code for LSQ data.
• NEVER attempt to call LSQ API endpoints using any tool outside this MCP.
• If a required capability is not covered by the tools below, say so clearly —
  do not work around the gap with code execution.
All LeadSquared data access must flow through the tools listed here.

IMPORTANT NOTES
───────────────
• Date format: All date parameters must be UTC in "YYYY-MM-DD HH:MM:SS" format.
• Custom fields: LSQ accounts have account-specific custom fields. Call get_lead_metadata
  first to discover available field names before filtering on custom fields.
• Pagination: All list/search tools return 25 results by default (max 1000).
  Check has_more and increment page to retrieve further results.
• Large responses: Tools with an output_file param can write results to a file path
  you specify. Responses over 100 KB are written to ~/.lsq-mcp/output/ automatically.
• Elasticsearch: get_leads_not_contacted, get_leads_no_active_tasks, and
  get_leads_pending_tasks require Elasticsearch to be enabled on your LSQ account.

RECOMMENDED CALL SEQUENCE
──────────────────────────
1. get_lead_metadata       — understand available lead fields (cached after first call)
2. get_activity_types      — know activity type names/IDs (cached)
3. get_task_types          — know task type names (cached)
4. get_opportunity_types   — know opportunity types (cached)
5. search_leads / quick_search_leads / get_lead_by_* — find the leads you need
6. get_lead_activities / get_lead_notes / get_opportunities_by_lead — enrich as needed

TOOLS BY MODULE
───────────────

LEADS (11 tools)
  get_lead_metadata          — field schemas, types, picklist values (cached)
  search_leads               — filter by any field (LookupName/LookupValue/SqlOperator)
  quick_search_leads         — full-text search across name, email, phone, company, city, country
  get_leads_by_ids           — bulk fetch up to 10,000 leads by ProspectID list
  get_lead_by_id             — full lead by ProspectID
  get_lead_by_email          — lookup by email
  get_lead_by_phone          — lookup by phone
  get_lead_owner             — assigned owner of a lead (lookup by any unique field)
  get_recently_modified_leads — leads modified in a date range
  get_lead_notes             — notes on a lead
  get_lead_activities        — full activity history for a lead (all types mixed)

OPPORTUNITIES (8 tools)
  get_opportunity_types      — all opportunity types (cached)
  get_opportunity_metadata   — field schema for a specific opportunity type
  get_opportunity_by_id      — single opportunity by ID
  get_opportunities_by_lead  — all opportunities for a lead
  get_opportunities_by_lead_field — opportunities by matching a unique lead field (email, phone)
  search_opportunities       — filtered search (admin only; requires opportunity_type_code)
  is_opportunity_enabled     — check if Opportunity feature is active for an org
  get_activities_of_opportunity — activities logged on an opportunity

ACTIVITIES (6 tools)
  get_activity_types         — all activity type definitions (cached)
  get_activities_by_lead     — paginated activity log for a lead
  get_activity_details       — full details of a single activity by ID
  get_activity_owner         — owner of an activity
  get_activity_settings      — custom activity type schema
  get_recently_modified_activities — activities modified in a date range

SALES ACTIVITIES (3 tools)
  get_products               — product catalogue (cached)
  get_sales_activity_types   — sales activity type configurations
  get_sales_activities_by_lead — sales transactions for a lead

TASKS (5 tools)
  get_task_types             — all task type definitions (cached)
  get_tasks_by_lead          — tasks for a lead
  get_tasks_by_owner         — tasks assigned to a user
  get_appointments           — appointment tasks for a user
  get_todos                  — to-do tasks for a user

USERS (6 tools)
  get_users                  — all users in the account (up to 200)
  get_user_by_id             — single user by ID
  search_users               — filtered user search
  get_user_hierarchy         — full reporting chain under a manager
  get_user_checkin_history   — field check-in records for a user
  get_user_availability      — working hours and available slots

LISTS (4 tools)
  get_lists                  — all lists (static + dynamic)
  get_leads_in_list          — paginated leads in a list
  get_lead_list_memberships  — lists a lead belongs to
  get_list_lead_count        — count of leads in a list without fetching them

ANALYTICS (4 tools — require Elasticsearch)
  get_lead_distribution      — leads by owner/stage with aggregation
  get_leads_not_contacted    — leads without any qualifying activity in a date range
  get_leads_no_active_tasks  — leads with no pending tasks
  get_leads_pending_tasks    — leads with overdue or pending tasks
"#;
