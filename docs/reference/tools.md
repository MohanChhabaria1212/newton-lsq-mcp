# Tools Reference

All 36 tools exposed by lsq-mcp. Each tool maps to a LeadSquared API endpoint.

> **Tip:** Call `get_instructions` at the start of every session — it summarises this page and provides recommended call sequences.

---

## get_instructions

Returns an in-session summary of all tools, recommended call sequences, date format requirements, and pagination notes. No API call is made — the response is static. Always call this first.

**Parameters:** none

**When to use:** At the start of every conversation before calling any other tool.

---

## Leads

Lead tools cover the core of LSQ — finding leads, reading their data, and retrieving related records.

### get_lead_metadata

Returns the complete field schema for leads on this account: field names, display labels, data types, and picklist values. Results are cached in memory for the server session (no repeated API calls).

**Parameters:** none

**When to use:** Call this first before any `search_leads` call. LSQ accounts have account-specific custom fields with unpredictable names — this tells you what fields are available to filter on.

**Response:** Array of field objects with `SchemaName`, `DisplayName`, `DataType`, `PickListValues` (for dropdown fields), etc.

---

### search_leads

Searches leads using one or more filter conditions. Supports any field returned by `get_lead_metadata`, including custom fields. Returns paginated results.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `filters` | JSON array | No | Array of filter conditions (see below). If omitted, returns all leads. |
| `page` | integer | No | Page number (1-based). Default: 1 |
| `page_size` | integer | No | Results per page. Default: 25, max: 100 |

**Filter format:**
```json
[
  {"Attribute": "LeadStage", "Operator": "eq", "Value": "Contacted"},
  {"Attribute": "CreatedOn", "Operator": "gte", "Value": "2024-01-01 00:00:00"}
]
```

**Operators:** `eq`, `neq`, `gt`, `lt`, `gte`, `lte`, `contains`, `startswith`

**Response:**
```json
{
  "results": [...],
  "total_count": 450,
  "page": 1,
  "page_size": 25,
  "has_more": true
}
```

When `has_more` is `true`, increment `page` to retrieve the next batch.

---

### get_lead_by_id

Returns the full lead record for a given ProspectID. Use when you already have a specific lead ID from a prior search result.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `lead_id` | string | Yes | LeadSquared ProspectID (GUID, e.g. `"a1b2c3d4-..."`) |

---

### get_lead_by_email

Looks up a lead by email address. Returns the lead record if found, or an empty response if no match.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `email` | string | Yes | Email address of the lead |

---

### get_lead_by_phone

Looks up a lead by phone number. Returns the lead record if found.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `phone` | string | Yes | Phone number (try with and without country code if no match) |

---

### get_lead_notes

Returns all notes (comments/remarks) attached to a lead, in chronological order. Notes are free-text entries added by sales reps.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `lead_id` | string | Yes | LeadSquared ProspectID |

---

### get_lead_activities

Returns the complete activity history for a lead — every interaction logged in LSQ (calls, emails, meetings, form fills, etc.). All activity types are returned together; filter by `ActivityEvent` name if you need a specific type.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `lead_id` | string | Yes | LeadSquared ProspectID |

**Tip:** Call `get_activity_types` first to see what activity type names are available on your account.

---

## Opportunities

Opportunity tools require the Opportunities module to be enabled on your LSQ plan.

### get_opportunity_types

Returns all opportunity types configured on the account (e.g. "Sales", "Renewal", "Upsell"). Each type has its own custom field schema. Cached for the session.

**Parameters:** none

**When to use:** Call before `get_opportunity_metadata` to get valid type IDs, or before `search_opportunities` to understand what types exist.

---

### get_opportunity_metadata

Returns the complete field schema for a specific opportunity type — field names, types, and picklist values.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `opportunity_type_id` | string | Yes | Opportunity type ID from `get_opportunity_types` |

---

### get_opportunity_by_id

Returns a single opportunity record by its ID.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `opportunity_id` | string | Yes | Opportunity ID |

---

### get_opportunities_by_lead

Returns all opportunities associated with a lead, across all opportunity types.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `lead_id` | string | Yes | LeadSquared ProspectID |

---

### search_opportunities

Searches opportunities using filter conditions. Returns paginated results.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `filters` | JSON array | No | Filter conditions (same format as `search_leads`) |
| `page` | integer | No | Page (default: 1) |
| `page_size` | integer | No | Per page (default: 25, max: 100) |

**Tip:** Call `get_opportunity_metadata` first to discover valid field names for the opportunity type you want to filter on.

---

## Activities

### get_activity_types

Returns all activity type definitions configured on the account — names, IDs, and the custom fields attached to each type. Cached for the session.

**Parameters:** none

**When to use:** Call this before filtering `get_activities_by_lead` results by activity type. Activity type names vary per account (e.g. "Phone Call" vs "Call").

---

### get_activities_by_lead

Returns a paginated activity log for a lead. Includes all activity types mixed together.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `lead_id` | string | Yes | LeadSquared ProspectID |
| `page` | integer | No | Page (default: 1) |
| `page_size` | integer | No | Per page (default: 25, max: 100) |

---

## Sales Activities

Sales Activity tools require the Sales Activities module to be enabled on your LSQ plan.

### get_products

Returns the product catalogue — product names, IDs, and prices. Cached for the session.

**Parameters:** none

---

### get_sales_activity_types

Returns all sales activity type configurations for the account.

**Parameters:** none

---

### get_sales_activities_by_lead

Returns paginated sales transaction records for a lead — what products were sold, at what price, and when.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `lead_id` | string | Yes | LeadSquared ProspectID |
| `page` | integer | No | Page (default: 1) |
| `page_size` | integer | No | Per page (default: 25, max: 100) |

---

## Tasks

### get_task_types

Returns all task type definitions (e.g. "Call", "Meeting", "Follow-up"). Cached for the session.

**Parameters:** none

---

### get_tasks_by_lead

Returns paginated tasks associated with a lead.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `lead_id` | string | Yes | LeadSquared ProspectID |
| `page` | integer | No | Page (default: 1) |
| `page_size` | integer | No | Per page (default: 25, max: 100) |

---

### get_tasks_by_owner

Returns paginated tasks assigned to a specific user. Useful for "what tasks does [person] have this week?" queries.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `owner_id` | string | Yes | User ID (get from `get_users`) |
| `page` | integer | No | Page (default: 1) |
| `page_size` | integer | No | Per page (default: 25, max: 100) |

---

### get_appointments

Returns appointment-type tasks (scheduled meetings, calls) for a user.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `user_id` | string | No | User ID (required if `email` not given) |
| `email` | string | No | User email (required if `user_id` not given) |
| `page` | integer | No | Page (default: 1) |
| `page_size` | integer | No | Per page (default: 25, max: 100) |

---

### get_todos

Returns to-do type tasks (follow-ups, reminders) for a user.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `user_id` | string | No | User ID (required if `email` not given) |
| `email` | string | No | User email (required if `user_id` not given) |
| `page` | integer | No | Page (default: 1) |
| `page_size` | integer | No | Per page (default: 25, max: 100) |

---

## Users

### get_users

Returns all users in the account — names, emails, roles, and team details. Returns up to 200 users. For accounts with more users, use `search_users` with filters.

**Parameters:** none

**When to use:** Use to resolve user names to user IDs, or to get a team roster for manager queries.

---

### get_user_by_id

Returns detailed information about a single user.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `user_id` | string | Yes | User ID |

---

### search_users

Searches users using filter conditions. More flexible than `get_users` for large accounts.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `filters` | JSON array | No | Filter conditions |
| `page` | integer | No | Page (default: 1) |
| `page_size` | integer | No | Per page (default: 25, max: 100) |

---

### get_user_hierarchy

Returns all users in a manager's reporting chain — direct reports and their direct reports, recursively.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `manager_id` | string | Yes | Manager's user ID |

**When to use:** "Show me everyone under [manager name]" — first call `get_users` to find the manager's user ID.

---

### get_user_checkin_history

Returns field check-in records for a user. LSQ field sales reps check in from their mobile app when visiting prospects.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `user_id` | string | Yes | User ID |
| `from_date` | string | No | Start of date range, UTC (`YYYY-MM-DD HH:MM:SS`) |
| `to_date` | string | No | End of date range, UTC (`YYYY-MM-DD HH:MM:SS`) |

---

### get_user_availability

Returns working hours and available time slots for a user.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `user_id` | string | No | User ID (required if `email` not given) |
| `email` | string | No | User email (required if `user_id` not given) |

---

## Lists

Lists require the Lists module to be enabled on your LSQ plan.

### get_lists

Returns all lists in the account — both static (manually curated) and dynamic (rule-based). Includes list name, ID, type, and lead count.

**Parameters:** none

---

### get_leads_in_list

Returns paginated leads belonging to a specific list.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `list_id` | string | Yes | List ID from `get_lists` |
| `page` | integer | No | Page (default: 1) |
| `page_size` | integer | No | Per page (default: 25, max: 100) |

---

### get_lead_list_memberships

Returns all lists that a specific lead belongs to.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `lead_id` | string | Yes | LeadSquared ProspectID |

**When to use:** "Which lists is this lead in?" — useful for understanding a lead's segment membership.

---

### get_list_lead_count

Returns only the count of leads in a list — no lead data. Faster than `get_leads_in_list` when you only need the number.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `list_id` | string | Yes | List ID from `get_lists` |

---

## Analytics

> **Requirement:** These four tools require **Elasticsearch to be enabled** on your LSQ account. Contact LSQ support if you receive an Elasticsearch error. All four use the LSQ Analytics API which requires a different filter schema than the search tools above.

### get_lead_distribution

Returns leads grouped and counted by a dimension — owner, stage, source, or any lead field. Use this for "how many leads does each rep have in each stage?" style queries.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `filters` | JSON object | Yes | LSQ Lead Distribution API filter body. Supports `UserFilter`, `LeadFilters`, `DateFilter`, and `Aggregate` fields. All dates UTC. |

**Example filter body:**
```json
{
  "DateFilter": {
    "FromDate": "2024-01-01 00:00:00",
    "ToDate": "2024-01-31 23:59:59"
  },
  "Aggregate": "Stage"
}
```

---

### get_leads_not_contacted

Returns leads that have not had any qualifying activity (call, email, meeting) within a date range. Useful for identifying neglected leads.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `filters` | JSON object | Yes | LSQ Leads Not Contacted API filter body. Supports `UserFilter`, `LeadFilters`, `ActivityFilters`, `DateFilter`. |

---

### get_leads_no_active_tasks

Returns leads that have no pending/active tasks assigned to them.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `filters` | JSON object | Yes | LSQ Leads With No Active Tasks API filter body. Supports `UserFilter`, `LeadFilters`, `TaskFilters`, `DateFilter`. |

---

### get_leads_pending_tasks

Returns leads that have overdue or pending tasks.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `filters` | JSON object | Yes | LSQ Leads With Pending Tasks API filter body. Set `TaskFilters.Status` to `Pending`, `Overdue`, or `PendingAndOverdue`. |

**Example filter body:**
```json
{
  "TaskFilters": {
    "Status": "Overdue"
  },
  "DateFilter": {
    "FromDate": "2024-01-01 00:00:00",
    "ToDate": "2024-01-31 23:59:59"
  }
}
```
