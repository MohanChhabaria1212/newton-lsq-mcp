# LeadSquared MCP Server — Design Spec
**Date:** 2026-04-14
**Status:** Approved

---

## Overview

A read-only MCP (Model Context Protocol) server that exposes LeadSquared CRM data to AI assistants (Claude Desktop, Claude Code, and any MCP-compatible client). Any LSQ user installs it, runs a one-time setup command, and their AI tools can then query leads, opportunities, analytics, tasks, users, and more — in plain language.

**v1 scope:** Read-only. Covers Core CRM + Analytics: Leads, Opportunities, Activities, Sales Activities, Tasks, Users, Lists, and all 4 Analytics endpoints.

---

## Tech Stack

| Concern | Choice | Rationale |
|---|---|---|
| Language | Rust 2024 edition | Single static binary, minimal RAM per connection, no runtime dependency |
| MCP SDK | `rmcp` v1.2 (`server`, `macros`, `transport-io`) | Official Anthropic Rust MCP SDK |
| Async runtime | `tokio` v1 (rt-multi-thread) | Industry-standard async runtime for Rust |
| HTTP client | `reqwest` v0.13 + `rustls` | TLS without an OpenSSL system dependency |
| Serialization | `serde` + `serde_json` + `schemars` | JSON + JSON Schema for MCP tool parameter validation |
| Error handling | `thiserror` v2 + `anyhow` v1 | Typed errors with ergonomic propagation |
| Logging | `tracing` + `tracing-subscriber` | Structured, async-aware logging to stderr |
| Transport | stdio | Standard for locally-installed MCP servers; works with all MCP clients |
| Distribution | npm package wrapping the Rust binary | Zero-dependency install via `npx` for non-Rust users |

### Release profile

```toml
[profile.release]
strip = true
lto = true
codegen-units = 1
```

Produces a small, optimised binary with no debug symbols.

---

## Authentication & Configuration

### Design principle

Keys are entered once via an interactive CLI command and stored locally. No browser flows, no OAuth, no JSON editing. Any user who can copy-paste from their LSQ settings page can set this up in under a minute.

Works for all account types:
- **Individual users** — configure with your own LSQ keys; access is automatically scoped to your LSQ permissions
- **Team members** — each person installs their own copy and configures with their personal keys; LSQ's permission model enforces what each person can see
- **Admin/shared deployments** — configure with admin credentials to get full team-wide visibility (all leads, all owners, full analytics, user hierarchies)

### CLI commands

```
lsq-mcp configure    Interactive setup: prompts for access key, secret key,
                     and API host. Saves to ~/.lsq-mcp/credentials.json.
                     Safe to re-run at any time to update credentials.

lsq-mcp status       Prints current config with keys masked (first 4 chars only).

lsq-mcp              (no args) Starts the MCP server over stdio.
```

### First-run experience

```
$ lsq-mcp configure

LeadSquared MCP Setup
─────────────────────
Find your API keys at: LSQ Portal → My Account → Settings → API and Webhooks
LSQ recommends admin credentials for full team-wide access.

Enter Access Key : ak_••••••••••••••••
Enter Secret Key : sk_••••••••••••••••
Enter API Host   [api.leadsquared.com]: ← press Enter to accept default

Verifying credentials...

✓ Connected as: Mohan Chhabaria (mohan@company.com)
  Role: Admin
  Credentials saved to ~/.lsq-mcp/credentials.json
  Start your MCP client to begin using lsq-mcp.
```

If verification fails:
```
✗ Invalid credentials — LSQ returned 401 Unauthorized.
  Nothing was saved.
  Double-check your keys at: LSQ Portal → My Account → Settings → API and Webhooks
  If your keys are correct, verify the API host matches your account region.
```

### Credential validation

After the user enters their keys, `configure` makes a lightweight verification call to `GET /v2/UserManagement.svc/GetByAccessKey` (or equivalent user-identity endpoint) before writing anything to disk. This:

1. Confirms the keys are valid before saving
2. Surfaces the connected user's **name, email, and role** so they can confirm it is the right account
3. Works identically for individual accounts, team member accounts, and admin accounts — whatever LSQ returns for those credentials is displayed
4. Rejects and discards invalid keys immediately with a clear error, so the credentials file is never written in a broken state

### Credential storage

`~/.lsq-mcp/credentials.json`, written with `0o600` permissions (owner-read only). The parent directory is created with `0o700`.

```json
{
  "access_key": "...",
  "secret_key": "...",
  "host": "api.leadsquared.com"
}
```

### API host by region

| Region | Host |
|---|---|
| US (default) | `api.leadsquared.com` |
| India — Mumbai | `api.in21.leadsquared.com` |
| India — Hyderabad | `api.in22.leadsquared.com` |
| Singapore | `api.sg21.leadsquared.com` |
| EU / Ireland | `api.eu21.leadsquared.com` |
| Canada | `api.ca21.leadsquared.com` |

Users who do not know their region can press Enter to accept the default. If requests fail with 401, the error message will suggest verifying the host.

### HTTP authentication

All API requests use headers (LSQ's recommended method; required for Service CRM endpoints):

```
x-LSQ-AccessKey: <access_key>
x-LSQ-SecretKey: <secret_key>
```

### Server startup behaviour

Credentials are loaded from disk at startup. If the file is missing or malformed, the server exits immediately with:

```
Error: No credentials found.
Run 'lsq-mcp configure' to set up your LSQ API keys.
```

---

## Module Structure

```
src/
  main.rs            CLI entry: configure | status | (serve)
  lib.rs             Module declarations
  auth.rs            Credential load / save / delete, file permission enforcement
  client.rs          LsqClient — reqwest wrapper, header injection, timeout, caching
  config.rs          Default host, VERSION constant, credential file path helper
  error.rs           LsqError enum + structured error message builder
  login.rs           Interactive configure flow (stdin → credential file)
  metadata.rs        Background version check against a remote config endpoint
  models.rs          LSQ API response structs (serde Deserialize)
  server.rs          LsqMcpServer, tool router, ensure_client(), check_auth()
  tools/
    mod.rs
    instructions.rs  Static usage guidance (no API call)
    leads.rs
    opportunities.rs
    activities.rs
    sales.rs
    tasks.rs
    users.rs
    lists.rs
    analytics.rs
```

---

## Tool Catalogue (36 tools, all read-only)

### Leads — 7 tools

| Tool | Description |
|---|---|
| `get_lead_metadata` | All lead field schemas, types, and picklist values. Call this first to understand what fields are available for filtering. |
| `search_leads` | Advanced search with filters on any lead field: stage, owner, date range, engagement score, custom fields. Paginated. |
| `get_lead_by_id` | Full lead record by ProspectID. |
| `get_lead_by_email` | Lookup a lead by email address. |
| `get_lead_by_phone` | Lookup a lead by phone number. |
| `get_lead_notes` | All notes attached to a lead. |
| `get_lead_activities` | Full activity history for a lead (calls, emails, meetings, custom events). |

### Opportunities — 5 tools

| Tool | Description |
|---|---|
| `get_opportunity_types` | All opportunity types configured in the account. |
| `get_opportunity_metadata` | Field schema for a given opportunity type. Requires opportunity type ID from `get_opportunity_types`. |
| `get_opportunity_by_id` | Single opportunity record. |
| `get_opportunities_by_lead` | All opportunities attached to a specific lead. |
| `search_opportunities` | Advanced opportunity search with filters. |

### Activities — 2 tools

| Tool | Description |
|---|---|
| `get_activity_types` | All activity types in the account (system, sales, and custom). |
| `get_activities_by_lead` | Chronological activity log for a lead. |

### Sales Activities — 3 tools

| Tool | Description |
|---|---|
| `get_products` | All products configured in the account. |
| `get_sales_activity_types` | Sales activity settings and type definitions. |
| `get_sales_activities_by_lead` | Sales transactions for a lead: product, revenue, SKU, date, owner. |

### Tasks — 5 tools

| Tool | Description |
|---|---|
| `get_task_types` | All task type names and configurations. |
| `get_tasks_by_lead` | All tasks attached to a lead. |
| `get_tasks_by_owner` | All tasks assigned to a specific user. |
| `get_appointments` | Appointments filtered by user ID, email, or search criteria. |
| `get_todos` | To-do items filtered by user ID, email, or search criteria. |

### Users — 6 tools

| Tool | Description |
|---|---|
| `get_users` | All users in the account. |
| `get_user_by_id` | Single user details. |
| `search_users` | Advanced user search by criteria. |
| `get_user_hierarchy` | Reporting chain under a given manager. |
| `get_user_checkin_history` | Historical check-in records for one or more users. |
| `get_user_availability` | Working hours and available appointment slots for a user. |

### Lists — 4 tools

| Tool | Description |
|---|---|
| `get_lists` | All lists in the account. |
| `get_leads_in_list` | All leads belonging to a list. |
| `get_lead_list_memberships` | Which lists a specific lead belongs to. |
| `get_list_lead_count` | Count of leads in a list. |

### Analytics — 4 tools

| Tool | Description |
|---|---|
| `get_lead_distribution` | Leads distributed by owner or stage with Count / Average / Sum aggregation. Supports date range and lead field filters. |
| `get_leads_not_contacted` | Leads where specified activities have not been posted within a time window. Supports user, lead, and activity filters. |
| `get_leads_no_active_tasks` | Leads with no pending or active tasks. Supports user, lead, and task type filters. |
| `get_leads_pending_tasks` | Leads with pending or overdue tasks. Supports Pending / Overdue / PendingAndOverdue status filter. |

### Utility — 1 tool

| Tool | Description |
|---|---|
| `get_instructions` | Describes all available tools and recommends call sequences. Static — no API call. |

---

## Data Flow

```
MCP Client (Claude Desktop / Claude Code / any MCP client)
    │
    │  stdio  —  MCP protocol (JSON-RPC over stdin/stdout)
    ▼
LsqMcpServer  (#[tool_router])
    │
    ├── ensure_client()
    │     Load ~/.lsq-mcp/credentials.json
    │     Build LsqClient if not already initialised
    │     Return structured error if credentials missing / invalid
    │
    ├── tool handler (e.g. search_leads)
    │     Validate parameters via schemars-generated JSON Schema
    │     Delegate to tools/<module>.rs function
    │
    ├── LsqClient
    │     Inject x-LSQ-AccessKey / x-LSQ-SecretKey headers
    │     30-second request timeout
    │     Transparent retry on HTTP 429 (see Rate Limiting section)
    │     Return LsqError::Unauthorized on HTTP 401
    │     Return LsqError::HostUnreachable on connection error
    │     In-memory cache for stable data (lead metadata, activity types)
    │     Reload credentials from disk on every ensure_client() call
    │
    └── check_auth()
          On Unauthorized → clear client, return re-configure message
          On success → success_json(&result)  (pretty-printed JSON)
```

---

## In-Memory Caching

Certain LSQ data changes rarely and is expensive to re-fetch on every tool call. `LsqClient` caches the following for the lifetime of the server process:

| Data | Cache key | Rationale |
|---|---|---|
| Lead field metadata | singleton | Field schemas don't change between requests |
| Activity types | singleton | Type list is stable within a session |
| Opportunity types | singleton | Same reason |
| Task types | singleton | Same reason |
| Products | singleton | Product catalogue rarely changes |

Cache is implemented as `Arc<RwLock<Option<Value>>>` per cached item — the same pattern used for the session client. First call populates it; subsequent calls read without hitting the API.

---

## Error Handling

### Error variants (`LsqError`)

| Variant | Cause |
|---|---|
| `Api(reqwest::Error)` | HTTP or network failure |
| `Unauthorized` | LSQ returned HTTP 401 |
| `HostUnreachable(String)` | DNS/connection failure — likely wrong host |
| `FeatureNotEnabled(String)` | LSQ feature not available on this account |
| `ElasticsearchNotEnabled` | Analytics endpoint requires Elasticsearch add-on |
| `RateLimitExhausted` | 429 persisted after all retries |
| `Auth(String)` | Credential file missing, unreadable, or malformed |
| `Configure(String)` | Invalid value provided during configure |
| `Io(io::Error)` | File system error |
| `Json(serde_json::Error)` | Failed to deserialise LSQ response |

### User-facing error format

All errors returned to the MCP client follow a structured 4-part format that gives the AI enough context to help the user resolve the issue:

```
Error: <what went wrong>
Reason: <why it happened>
Solution: <immediate action to take>
Alternative: <fallback if the solution does not work>
```

Example for an expired key:
```
Error: LeadSquared API request failed with 401 Unauthorized.
Reason: Your access key or secret key is invalid or has been revoked.
Solution: Run 'lsq-mcp configure' to enter new credentials.
Alternative: Verify your keys at LSQ Portal → My Account → Settings → API and Webhooks, then re-run configure.
```

### 401 handling

On a 401 response, the server clears the cached client and returns the above message. The next tool call re-checks credentials from disk, so if the user runs `lsq-mcp configure` in a separate terminal and retries, it works without restarting the server.

### Wrong host handling

A connection error (DNS failure, timeout, refused) is distinct from a 401 and gets its own message:

```
Error: Could not reach api.wronghost.leadsquared.com.
Reason: The API host may be incorrect for your account region.
Solution: Run 'lsq-mcp configure' and enter the correct host for your region.
Alternative: Check your region in the LSQ portal. Regional hosts are listed in the README.
```

### Rate limiting — transparent retry

429 responses are handled entirely inside `LsqClient` and are never surfaced to the user unless all retries are exhausted.

**Retry strategy:**
1. On HTTP 429, read the `Retry-After` header. If present, wait exactly that many seconds.
2. If no `Retry-After` header, use exponential backoff: wait 1s, then 2s, then 4s.
3. Maximum **3 retries** per request.
4. Log each retry attempt at `DEBUG` level only — nothing visible to the user.
5. If all 3 retries fail, return a single clean message:

```
Error: LeadSquared is temporarily rate-limiting requests.
Reason: Too many API calls were made in a short period.
Solution: Wait a moment and try again.
Alternative: Reduce the frequency of tool calls if this recurs.
```

### Elasticsearch requirement

Three analytics tools require Elasticsearch to be enabled on the LSQ account:
- `get_leads_not_contacted`
- `get_leads_no_active_tasks`
- `get_leads_pending_tasks`

If the LSQ API returns an error indicating Elasticsearch is not enabled, the server returns:

```
Error: This analytics tool requires Elasticsearch to be enabled on your LSQ account.
Reason: The Leads Not Contacted / No Active Tasks / Pending Tasks APIs depend on LSQ's Elasticsearch feature.
Solution: Contact LSQ support to enable Elasticsearch for your account.
Alternative: Use search_leads with manual filters as a partial substitute.
```

### Feature availability

LSQ accounts have different modules enabled (Opportunities, Sales Activities, Lists). If a tool calls an endpoint for a feature that is not enabled, the server detects the specific LSQ error response and returns:

```
Error: The [Opportunities / Sales Activities / Lists] feature is not enabled on your LSQ account.
Reason: This module requires activation in your LSQ plan.
Solution: Contact your LSQ account manager to enable this feature.
Alternative: Skip this tool and use lead-level tools instead.
```

### Credential reload on running server

`ensure_client()` reloads credentials from disk on every call rather than caching the credential object indefinitely. This means if the user runs `lsq-mcp configure` while the server is running (to update keys), the new credentials take effect on the very next tool call — no server restart required.

### Log sanitisation

Credentials must never appear in log output. `LsqClient` logs only masked versions:
- Access key: first 4 characters + `****` (e.g. `ak_1****`)
- Secret key: never logged, even masked
- Full URLs logged without query parameters if credentials were ever passed as params (they are not in our implementation — headers only)

---

## Response Format

All tools return responses via `success_json(&value)` — a pretty-printed JSON string wrapped in a `CallToolResult`. Raw LSQ responses are never passed through directly. Each tool module contains `build_*` helper functions that reshape, flatten, and normalise the raw response before returning it. This keeps `server.rs` thin and makes each tool's output independently testable.

### Pagination

All list/search tools are paginated to protect the AI's context window and avoid overwhelming responses.

| Parameter | Default | Maximum |
|---|---|---|
| `page` | 1 | unbounded |
| `page_size` | 25 | 100 |

Every paginated response includes:
```json
{
  "results": [...],
  "total_count": 1500,
  "page": 1,
  "page_size": 25,
  "has_more": true
}
```

No tool auto-paginates silently. The AI is responsible for requesting subsequent pages when `has_more` is true.

### Dates and timezones

LSQ stores all timestamps in UTC. Tool parameters that accept dates require UTC datetime in `YYYY-MM-DD HH:MM:SS` format. Tool descriptions call this out explicitly. Responses include timestamps as returned by LSQ (UTC) with a note in the `get_instructions` tool about timezone handling.

### Multiple LSQ accounts

v1 supports one credential set per machine. Running `lsq-mcp configure` overwrites the existing credentials. Users who work across multiple LSQ accounts (e.g. consultants) should run `lsq-mcp configure` to switch between them. Named profiles (`--profile`) are a planned v2 feature.

---

## Tool Description Quality

Tool descriptions in the `#[tool]` attribute serve double duty: they guide the AI on *when* to call each tool and *in what order*. Descriptions follow this pattern:

1. **What the tool does** — one sentence
2. **When to use it** — trigger phrases a user might say
3. **Dependencies** — if another tool must be called first, say so explicitly
4. **Output** — what key fields the caller should expect

Example for `search_leads`:
> "Search leads using filters on any lead field (stage, owner, date range, engagement score, custom fields). Use when the user asks to find, list, or filter leads. Call `get_lead_metadata` first if you need to know available field names or picklist values — especially for custom fields, which vary by account. All date parameters must be UTC in `YYYY-MM-DD HH:MM:SS` format. Returns a paginated list (default 25 per page); check `has_more` and increment `page` to retrieve further results."

### Custom fields

LSQ accounts have custom lead fields with account-specific names (e.g. `mx_Lead_Source_Detail`). The AI cannot know these names without calling `get_lead_metadata` first. Tools that accept field-based filters (`search_leads`, `search_opportunities`, analytics tools) explicitly state this dependency in their descriptions. `get_lead_metadata` is also listed as the recommended first call in `get_instructions`.

---

## Testing

### Unit tests
Each `tools/<module>.rs` file contains unit tests for its `build_*` response-shaping functions. Fixture JSON is embedded inline. No network calls.

### Integration tests
`tests/` directory contains integration tests that hit a real LSQ sandbox account. Credentials are read from environment variables (`LSQ_ACCESS_KEY`, `LSQ_SECRET_KEY`, `LSQ_HOST`) so they can run in CI without a credentials file on disk.

---

## Distribution

The binary is wrapped in an npm package (in the `npm/` directory) following the pattern used by other Rust-based CLI tools distributed via npm:

- `npm/package.json` — declares the binary and supported platforms
- `npm/run.js` — downloads the correct pre-built binary for the current OS/arch on first run

Users install via:
```
npx -y lsq-mcp
```

Or globally:
```
npm install -g lsq-mcp
```

Initial target platforms: `darwin-arm64`. Expand to `darwin-x64`, `linux-x64`, `linux-arm64` in subsequent releases.

---

## Out of Scope — v1

| Area | Reason deferred |
|---|---|
| Write operations | Adds risk surface; read-only v1 proves value first |
| Async API (`x-api-key`) | Write-only endpoints; nothing to read |
| Service CRM (tickets) | Different product line, different user base |
| Portal API | Customer-facing portals, not sales CRM |
| Email marketing / landing pages | Content tooling, not CRM data |
| Telephony | Very narrow surface (SSO key + owner phone lookup); low AI value |
| Webhooks / Lapps / Batch jobs | Developer platform, not data access |
