# LSQ API Endpoints Reference

All endpoints used by lsq-mcp. Edit this file to update URLs without touching source code — then update the corresponding source file listed in the **Source** column.

Base URL: `https://{host}/v2` (e.g. `https://api.leadsquared.com/v2`)

Auth: **all** endpoints use `?accessKey=&secretKey=` query params. No headers required.

---

## Auth / Credential Validation

| Source file | Method | Path | Notes |
|---|---|---|---|
| `src/login.rs` | GET | `/UserManagement.svc/Users.Get` | Used only during `lsq-mcp configure` to validate keys. Returns array of user objects. |

---

## Leads

| Source file | Method | Path | Notes |
|---|---|---|---|
| `src/client.rs` (cached) | GET | `/LeadManagement.svc/LeadsMetaData.Get` | Returns all field schema for leads. Cached per session. |
| `src/tools/leads.rs` | POST | `/LeadManagement.svc/Leads.Get` | Search leads. Body: `{ Parameter: { LookupName, LookupValue, SqlOperator }, Paging: { PageIndex (1-based), PageSize } }`. Empty `Parameter: {}` returns all leads. Response: direct array of lead objects (no wrapper). SqlOperator: `=`, `LIKE`, `>`, `<`, `<=`, `>=`, `<>`. |
| `src/tools/leads.rs` | GET | `/LeadManagement.svc/Leads.GetByQuickSearch?key={term}` | Full-text search across name, email, phone, company, city, country. Returns array. |
| `src/tools/leads.rs` | POST | `/LeadManagement.svc/Leads/Retrieve/ByIds` | Bulk fetch by ProspectID list. Body: `{ SearchParameters: { LeadIds: [...] }, Columns: { Include_CSV }, Paging: { PageIndex (1-based), PageSize } }`. Response: `{ RecordCount, Leads: [...] }`. |
| `src/tools/leads.rs` | GET | `/LeadManagement.svc/LeadOwner.Get?LeadIdentifier={field}&value={val}` | Get owner details of a lead. LeadIdentifier is the field name (e.g. EmailAddress, LeadId). Response: array of user objects. |
| `src/tools/leads.rs` | POST | `/LeadManagement.svc/Leads.RecentlyModified` | Leads modified in a date range. Body: `{ Parameter: { FromDate, ToDate }, Columns: { Include_CSV }, Paging: { PageIndex (1-based), PageSize } }`. Response: `{ RecordCount, Leads: [{ LeadPropertyList: [...] }] }`. |
| `src/tools/leads.rs` | GET | `/LeadManagement.svc/Leads.GetById?id={leadId}` | Fetch single lead by ProspectID. |
| `src/tools/leads.rs` | GET | `/LeadManagement.svc/Leads.GetByEmailaddress?emailaddress={email}` | Lookup lead by email. |
| `src/tools/leads.rs` | GET | `/LeadManagement.svc/RetrieveLeadByPhoneNumber?phone={phone}` | Lookup lead by phone. |
| `src/tools/leads.rs` | POST | `/LeadManagement.svc/RetrieveNote` | Get notes for a lead. Body: `{ Parameter: { RelatedId: "{leadId}", RelatedEntityTypeId: 1 } }`. |
| `src/tools/leads.rs` | POST | `/ProspectActivity.svc/Retrieve?leadId={leadId}` | Full activity history (all types). Body: `{ Paging: { PageIndex, PageSize } }`. |

---

## Opportunities

| Source file | Method | Path | Notes |
|---|---|---|---|
| `src/client.rs` (cached) | GET | `/OpportunityManagement.svc/GetOpportunityTypes` | Returns all opportunity type definitions. Cached per session. |
| `src/tools/opportunities.rs` | GET | `/OpportunityManagement.svc/GetOpportunityTypeMetadata?code={typeId}` | Field schema for a specific opportunity type. |
| `src/tools/opportunities.rs` | GET | `/OpportunityManagement.svc/GetOpportunityDetails?OpportunityId={id}` | Fetch single opportunity by ID. |
| `src/tools/opportunities.rs` | POST | `/OpportunityManagement.svc/GetOpportunitiesOfLead?leadId={leadId}` | All opportunities for a lead. Body: `{}` (empty triggers all types). |
| `src/tools/opportunities.rs` | GET | `/OpportunityManagement.svc/IsOpportunityEnabled?orgId={id}` | Check if opportunity feature is enabled. Response: `{ OpportunityManagement: "Enabled" }`. |
| `src/tools/opportunities.rs` | POST | `/OpportunityManagement.svc/GetOpportunitiesByUniqueLeadField` | Opportunities for leads matching a unique field. Body: `{ Parameter: { LookupName, LookupValue, SqlOperator }, Columns: { Include_CSV }, Paging }`. |
| `src/tools/opportunities.rs` | POST | `/OpportunityManagement.svc/Retrieve/BySearchParameter` | Search opportunities. Body: `{ OpportunityEventCode, AdvancedSearch: "<stringified JSON>", Paging: { PageIndex (1-based), PageSize } }`. Response: `{ RecordCount, List: [...] }`. Admin only. |
| `src/tools/opportunities.rs` | GET | `/OpportunityManagement.svc/GetActivitiesOfOpportunity?opportunityId={id}` | Activities logged on an opportunity. **Path unconfirmed** — update if wrong. |

---

## Activities

| Source file | Method | Path | Notes |
|---|---|---|---|
| `src/client.rs` (cached) | GET | `/ProspectActivity.svc/ActivityTypes.Get` | Returns all activity type definitions. Cached per session. |
| `src/tools/activities.rs` | POST | `/ProspectActivity.svc/Retrieve?leadId={leadId}` | Paginated activity log for a lead. Body: `{ Paging: { PageIndex, PageSize } }`. Max 25/page (LSQ cap). |
| `src/tools/activities.rs` | GET | `/ProspectActivity.svc/GetActivityDetails?activityId={id}` | Full details of a single activity including all field values. |
| `src/tools/activities.rs` | GET | `/ProspectActivity.svc/ActivityOwner.Get?activityId={id}` | Owner of an activity. **Path unconfirmed** — update if wrong. |
| `src/tools/activities.rs` | GET | `/ProspectActivity.svc/CustomActivity/GetActivitySetting` | Custom activity type settings/schema. **Path unconfirmed** — update if wrong. |
| `src/tools/activities.rs` | POST | `/ProspectActivity.svc/RetrieveRecentlyModified` | Activities modified in a date range. Body: `{ Parameter: { FromDate, ToDate }, Paging }`. **Path unconfirmed** — update if wrong. |

---

## Sales Activities

| Source file | Method | Path | Notes |
|---|---|---|---|
| `src/client.rs` (cached) | GET | `/SalesActivity.svc/Product/GetAll` | Product catalogue. **Path unconfirmed in docs** — update if wrong. Cached per session. |
| `src/tools/sales.rs` | GET | `/SalesActivity.svc/RetrieveSetting` | Sales activity type configurations. |
| `src/tools/sales.rs` | GET | `/SalesActivity.svc/RetrieveByLeadId?leadId={leadId}&pageIndex={n}&pageSize={n}` | Sales transactions for a lead. |

---

## Tasks

| Source file | Method | Path | Notes |
|---|---|---|---|
| `src/client.rs` (cached) | GET | `/Task.svc/TaskType/GetAll` | Task type definitions. **Path unconfirmed in docs** — update if wrong. Cached per session. |
| `src/tools/tasks.rs` | GET | `/LeadManagement.svc/RetrieveTaskByLeadId?leadId={leadId}&pageIndex={n}&pageSize={n}` | Tasks for a lead. |
| `src/tools/tasks.rs` | POST | `/Task.svc/Retrieve` | Tasks by owner. Body: `{ UserId: "{ownerId}", PageIndex, PageSize }`. |
| `src/tools/tasks.rs` | GET | `/Task.svc/RetrieveAppointments?pageIndex={n}&pageSize={n}&userId={id}` | Appointment-type tasks. **Path unconfirmed** — may be a filter on `Task.svc/Retrieve`. |
| `src/tools/tasks.rs` | GET | `/Task.svc/RetrieveToDos?pageIndex={n}&pageSize={n}&userId={id}` | To-do type tasks. **Path unconfirmed** — may be a filter on `Task.svc/Retrieve`. |

---

## Users

| Source file | Method | Path | Notes |
|---|---|---|---|
| `src/tools/users.rs` | GET | `/UserManagement.svc/Users.Get` | All users (up to 200). |
| `src/tools/users.rs` | GET | `/UserManagement.svc/User/Retrieve/ByUserId?userId={id}` | Single user by ID. |
| `src/tools/users.rs` | POST | `/UserManagement.svc/User/AdvancedSearch` | Search users. Body: `{ Filters: [...], Paging: { PageIndex, PageSize } }`. |
| `src/tools/users.rs` | GET | `/UserManagement.svc/ReportingHierarchy/RetrieveAllReportingUsers?UserId={id}` | Full reporting chain under a manager. |
| `src/tools/users.rs` | POST | `/UserManagement.svc/User/GetCheckinCheckoutHistory` | Check-in history. Body: `{ UserIds: ["{id}"], FromDate: "YYYY-MM-DD HH:MM:SS", ToDate: "..." }`. |
| `src/tools/users.rs` | POST | `/Task.svc/RetrieveAvailableSlots/ByUserId` | Availability by user ID. Body: `{ UserIds: ["{id}"] }`. |
| `src/tools/users.rs` | POST | `/Task.svc/RetrieveAvailableSlots/ByUserSearchCriteria` | Availability by email. Body: `{ EmailAddress: "{email}" }`. |

---

## Lists

| Source file | Method | Path | Notes |
|---|---|---|---|
| `src/tools/lists.rs` | GET | `/LeadManagement.svc/Lists.Get` | All lists (static + dynamic). |
| `src/tools/lists.rs` | GET | `/LeadManagement.svc/List.GetLeads?listId={id}&pageIndex={n}&pageSize={n}` | Lead IDs in a list. |
| `src/tools/lists.rs` | GET | `/List.svc/GetByLeadId?leadId={id}` | Lists a lead belongs to. **Path unconfirmed** — update if wrong. |
| `src/tools/lists.rs` | GET | `/List.svc/GetLeadCount?listId={id}` | Count of leads in a list. **Path unconfirmed** — update if wrong. |

---

## Analytics (Elasticsearch required)

Analytics endpoints use a different base URL (`https://{host}`) and auth via query params, not headers.

| Source file | Method | Path | Notes |
|---|---|---|---|
| `src/tools/analytics.rs` | POST | `/Leads/LeadDistribution/FilterByLeadField` | Lead distribution by field/owner. Body: LSQ LeadDistribution filter schema. |
| `src/tools/analytics.rs` | POST | `/v2/Leads/NotContacted/FilterByLeadField` | Leads not contacted in date range. |
| `src/tools/analytics.rs` | POST | `/v2/Leads/NoActiveTasks/FilterByLeadField` | Leads with no active tasks. |
| `src/tools/analytics.rs` | POST | `/v2/Leads/PendingTasks/FilterByLeadField` | Leads with pending/overdue tasks. |

---

## Unconfirmed Paths

These paths were not found in the official LSQ API docs. Best-effort guesses — verify and update here + source if incorrect.

| Path | Used in | Status |
|---|---|---|
| `/Task.svc/TaskType/GetAll` | `src/client.rs` | Unconfirmed |
| `/SalesActivity.svc/Product/GetAll` | `src/client.rs` | Unconfirmed |
| `/Task.svc/RetrieveAppointments` | `src/tools/tasks.rs` | Unconfirmed |
| `/Task.svc/RetrieveToDos` | `src/tools/tasks.rs` | Unconfirmed |
| `/List.svc/GetByLeadId` | `src/tools/lists.rs` | Unconfirmed |
| `/List.svc/GetLeadCount` | `src/tools/lists.rs` | Unconfirmed |
| `/OpportunityManagement.svc/GetActivitiesOfOpportunity` | `src/tools/opportunities.rs` | Unconfirmed |
| `/ProspectActivity.svc/ActivityOwner.Get` | `src/tools/activities.rs` | Unconfirmed |
| `/ProspectActivity.svc/CustomActivity/GetActivitySetting` | `src/tools/activities.rs` | Unconfirmed |
| `/ProspectActivity.svc/RetrieveRecentlyModified` | `src/tools/activities.rs` | Unconfirmed |

---

## Official LSQ API Docs

- Lead Management: https://apidocs.leadsquared.com/lead-management/
- Opportunity Management: https://apidocs.leadsquared.com/opportunity-management/
- Activity Management: https://apidocs.leadsquared.com/activity-management/
- Sales Activity Management: https://apidocs.leadsquared.com/sales-activity-management/
- Task Management: https://apidocs.leadsquared.com/task-management/
- User Management: https://apidocs.leadsquared.com/user-management/
- List Management: https://apidocs.leadsquared.com/list-management/
- Analytics API: https://apidocs.leadsquared.com/analytics/
