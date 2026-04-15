# Tools Reference

All 36 tools exposed by lsq-mcp. Call `get_instructions` first — it returns this same information in-session with recommended call sequences.

---

## get_instructions

Returns descriptions of all tools, call sequences, and usage notes. No API call required. Always call this first in a new session.

**Parameters:** none

---

## Leads

### get_lead_metadata
Get all lead field schemas, types, and picklist values for the account. **Call this first before any lead search** — custom field names vary per account. Results cached for the session.

**Parameters:** none

### search_leads
Search leads using filter conditions on any field.

| Parameter | Type | Description |
|---|---|---|
| `filters` | JSON array | Array of `{"Attribute":"FieldName","Operator":"eq","Value":"..."}` conditions |
| `page` | integer | Page number (1-based, default: 1) |
| `page_size` | integer | Results per page (default: 25, max: 100) |

### get_lead_by_id
Get full lead details by ProspectID (GUID).

| Parameter | Type | Description |
|---|---|---|
| `lead_id` | string | LeadSquared ProspectID |

### get_lead_by_email
Look up a lead by email address.

| Parameter | Type | Description |
|---|---|---|
| `email` | string | Email address |

### get_lead_by_phone
Look up a lead by phone number.

| Parameter | Type | Description |
|---|---|---|
| `phone` | string | Phone number |

### get_lead_notes
Get all notes attached to a lead.

| Parameter | Type | Description |
|---|---|---|
| `lead_id` | string | LeadSquared ProspectID |

### get_lead_activities
Get the full activity history for a lead.

| Parameter | Type | Description |
|---|---|---|
| `lead_id` | string | LeadSquared ProspectID |

---

## Opportunities

### get_opportunity_types
Get all opportunity types available on the account. Cached.

**Parameters:** none

### get_opportunity_metadata
Get field schema for a specific opportunity type.

| Parameter | Type | Description |
|---|---|---|
| `opportunity_type_id` | string | ID from get_opportunity_types |

### get_opportunity_by_id
Get a single opportunity by ID.

| Parameter | Type | Description |
|---|---|---|
| `opportunity_id` | string | Opportunity ID |

### get_opportunities_by_lead
Get all opportunities for a lead.

| Parameter | Type | Description |
|---|---|---|
| `lead_id` | string | LeadSquared ProspectID |

### search_opportunities
Search opportunities with filter conditions.

| Parameter | Type | Description |
|---|---|---|
| `filters` | JSON array | Filter conditions |
| `page` | integer | Page (default: 1) |
| `page_size` | integer | Per page (default: 25, max: 100) |

---

## Activities

### get_activity_types
Get all activity type definitions — names, IDs, field schemas. Cached.

**Parameters:** none

### get_activities_by_lead
Get paginated activity log for a lead.

| Parameter | Type | Description |
|---|---|---|
| `lead_id` | string | LeadSquared ProspectID |
| `page` | integer | Page (default: 1) |
| `page_size` | integer | Per page (default: 25, max: 100) |

---

## Sales Activities

### get_products
Get the product catalogue. Cached.

**Parameters:** none

### get_sales_activity_types
Get all sales activity type configurations.

**Parameters:** none

### get_sales_activities_by_lead
Get sales activity (transaction) records for a lead.

| Parameter | Type | Description |
|---|---|---|
| `lead_id` | string | LeadSquared ProspectID |
| `page` | integer | Page (default: 1) |
| `page_size` | integer | Per page (default: 25, max: 100) |

---

## Tasks

### get_task_types
Get all task type definitions. Cached.

**Parameters:** none

### get_tasks_by_lead
Get paginated tasks for a lead.

| Parameter | Type | Description |
|---|---|---|
| `lead_id` | string | LeadSquared ProspectID |
| `page` | integer | Page (default: 1) |
| `page_size` | integer | Per page (default: 25, max: 100) |

### get_tasks_by_owner
Get tasks assigned to a user.

| Parameter | Type | Description |
|---|---|---|
| `owner_id` | string | User ID |
| `page` | integer | Page (default: 1) |
| `page_size` | integer | Per page (default: 25, max: 100) |

### get_appointments
Get appointment tasks for a user.

| Parameter | Type | Description |
|---|---|---|
| `user_id` | string | User ID (optional if email provided) |
| `email` | string | User email (optional if user_id provided) |
| `page` | integer | Page (default: 1) |
| `page_size` | integer | Per page (default: 25, max: 100) |

### get_todos
Get to-do tasks for a user.

| Parameter | Type | Description |
|---|---|---|
| `user_id` | string | User ID (optional if email provided) |
| `email` | string | User email (optional if user_id provided) |
| `page` | integer | Page (default: 1) |
| `page_size` | integer | Per page (default: 25, max: 100) |

---

## Users

### get_users
Get all users in the account (up to 200). For larger accounts use `search_users`.

**Parameters:** none

### get_user_by_id
Get a single user by ID.

| Parameter | Type | Description |
|---|---|---|
| `user_id` | string | User ID |

### search_users
Search users with filter conditions.

| Parameter | Type | Description |
|---|---|---|
| `filters` | JSON array | Filter conditions |
| `page` | integer | Page (default: 1) |
| `page_size` | integer | Per page (default: 25, max: 100) |

### get_user_hierarchy
Get all users in a manager's reporting chain.

| Parameter | Type | Description |
|---|---|---|
| `manager_id` | string | Manager's user ID |

### get_user_checkin_history
Get field check-in history for a user.

| Parameter | Type | Description |
|---|---|---|
| `user_id` | string | User ID |
| `from_date` | string | From date UTC (YYYY-MM-DD HH:MM:SS, optional) |
| `to_date` | string | To date UTC (YYYY-MM-DD HH:MM:SS, optional) |

### get_user_availability
Get working hours and availability for a user.

| Parameter | Type | Description |
|---|---|---|
| `user_id` | string | User ID (optional if email provided) |
| `email` | string | User email (optional if user_id provided) |

---

## Lists

### get_lists
Get all lists (static and dynamic) in the account.

**Parameters:** none

### get_leads_in_list
Get paginated leads in a list.

| Parameter | Type | Description |
|---|---|---|
| `list_id` | string | List ID from get_lists |
| `page` | integer | Page (default: 1) |
| `page_size` | integer | Per page (default: 25, max: 100) |

### get_lead_list_memberships
Get all lists a lead belongs to.

| Parameter | Type | Description |
|---|---|---|
| `lead_id` | string | LeadSquared ProspectID |

### get_list_lead_count
Get the count of leads in a list without fetching them.

| Parameter | Type | Description |
|---|---|---|
| `list_id` | string | List ID |

---

## Analytics *(requires Elasticsearch)*

These four tools require Elasticsearch to be enabled on your LSQ account. Contact LSQ support if you receive an Elasticsearch error.

### get_lead_distribution
Lead count aggregated by owner, stage, or other dimensions.

| Parameter | Type | Description |
|---|---|---|
| `filters` | JSON object | LSQ Lead Distribution API filter body |

### get_leads_not_contacted
Leads with no qualifying activity in a date range.

| Parameter | Type | Description |
|---|---|---|
| `filters` | JSON object | LSQ Leads Not Contacted API filter body |

### get_leads_no_active_tasks
Leads with no pending tasks.

| Parameter | Type | Description |
|---|---|---|
| `filters` | JSON object | LSQ Leads With No Active Tasks API filter body |

### get_leads_pending_tasks
Leads with overdue or pending tasks.

| Parameter | Type | Description |
|---|---|---|
| `filters` | JSON object | LSQ Leads With Pending Tasks API filter body (use `TaskFilters.Status`: `Pending`/`Overdue`/`PendingAndOverdue`) |
