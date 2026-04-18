# LeadSquared MCP — Progress Log

Running log of implementation progress. Updated after each module or phase is completed. Each entry captures: what was built, any implementation surprises, deviations from the spec, and what comes next.

---

## How to resume a session

1. Read `2026-04-14-lsq-mcp-design.md` — the full design spec
2. Read `decisions-log.md` — why key decisions were made
3. Read this file from the latest entry upward — what has been done and what's next
4. Check `Cargo.toml` and `src/lib.rs` to verify current module state

---

## 2026-04-14 — Design Phase Complete

**What was done:**
- Full design spec written and approved (see `2026-04-14-lsq-mcp-design.md`)
- All edge cases identified and incorporated into the spec
- Decisions log created
- Skeleton project at `/Documents/codebase/newton-lsq-mcp` confirmed — `Cargo.toml` and `lib.rs` already exist

**Key facts about the skeleton:**
- `Cargo.toml` already has correct dependencies: rmcp v1.2, tokio, reqwest/rustls, serde, schemars, thiserror, anyhow, tracing, dirs, open
- `lib.rs` already declares all module names: auth, client, config, error, login, metadata, models, server, tools
- `main.rs` exists but is a stub (empty `fn main()`)
- No tool files exist yet — `src/tools/` directory does not exist

**Next step:** Write and execute implementation plan.

---

---

## 2026-04-15 — Implementation Complete (Tasks 1–13)

All 13 implementation tasks from the plan are complete. The server compiles and all module tests pass.

---

### Task 1: Cargo.toml + error.rs + config.rs

**What was done:**
- `Cargo.toml`: added `query` feature to reqwest, credential encryption deps (`aes-gcm`, `rand`, `base64`), release profile (`strip`, `lto`, `codegen-units=1`).
- `src/error.rs`: `LsqError` enum with `Api`, `Unauthorized`, `HostUnreachable`, `RateLimitExhausted`, `Auth`, `Configure`, `Io`, `Json` variants; `lsq_error()` 4-part message builder.
- `src/config.rs`: `api_base()`, `analytics_base()`, `credentials_path()` (honours `LSQ_MCP_HOME`), `keyfile_path()`, `output_dir()`, `cleanup_output_dir()`, `MAX_OUTPUT_FILES = 100`.

**Deviations from spec:** `cleanup_output_dir` and `MAX_OUTPUT_FILES` were added here (not in the original plan) to support the output file auto-threshold feature decided during task 6.

---

### Task 2: auth.rs — Credential Load/Save/Delete

**What was done:**
- `Credentials` struct: `access_key`, `secret_key`, `host`, optional `user_name`, `user_email`, `user_role`.
- `load_credentials()` / `save_credentials()` / `delete_credentials()`.
- **AES-256-GCM encryption added (not in original spec):** credentials are encrypted at rest. Key stored at `~/.lsq-mcp/.key` (0o600). Credentials stored as `{v:2, n:<base64 nonce>, ct:<base64 ciphertext>}`. v1 plaintext files are silently re-encrypted on load.

**Surprise:** The original spec used plaintext JSON. Encryption was added after recognising that cloud sync (iCloud/Dropbox) routinely picks up dotfiles. See decisions log: "Credential Encryption: AES-256-GCM at Rest".

---

### Task 3: login.rs — Interactive Configure Flow

**What was done:**
- Interactive stdin prompts for host, access key, secret key.
- Validates credentials with a live `get_lead_metadata` call before saving.
- Displays connected user name, email, and role on success.
- `lsq-mcp status` command reads credentials and shows current account.

---

### Task 4: client.rs — HTTP Layer with Retry and Caching

**What was done:**
- `LsqClient` struct with `get`, `get_url`, `get_with_params`, `post`, `post_analytics`, `post_url` methods.
- Auth via query params (`accessKey`/`secretKey`) appended to every URL before the retry loop.
- 429 retry: reads `Retry-After` header (default: exponential backoff 1s→2s→4s), max 3 retries, surfaces `LsqError::RateLimitExhausted` only on exhaustion.
- 401: immediate `LsqError::Unauthorized`.
- In-memory caches for `lead_metadata`, `activity_types`, `opportunity_types`, `task_types`, `products` via `Arc<RwLock<Option<Value>>>`.
- `#[cfg(test)]` constructor `new_for_testing(creds, server_uri)` routes all HTTP to a local mock server.

**Deviation from spec:** Auth uses query params, not headers. See decisions log: "HTTP Auth: Switched to Query Params".

---

### Task 5: models.rs — Tool Parameter Structs

**What was done:**
- All parameter structs with `JsonSchema + Deserialize`: `SearchLeadsParams`, `LeadIdParam`, `LeadEmailParam`, `LeadPhoneParam`, `GetLeadsByIdsParams`, `QuickSearchLeadsParams`, `LeadOwnerParams`, `RecentlyModifiedLeadsParams`, `OpportunityIdParam`, `OpportunityMetadataParams`, `SearchOpportunitiesParams`, `GetOpportunitiesByLeadFieldParams`, `IsOpportunityEnabledParams`, `ActivityIdParam`, `RecentlyModifiedActivitiesParams`, `ActivitiesByLeadParams`, `SalesActivitiesByLeadParams`, `TasksByLeadParams`, `TasksByOwnerParams`, `AppointmentParams`, `GetUsersParams`, `UserIdParam`, `SearchUsersParams`, `UserHierarchyParams`, `CheckInHistoryParams`, `AvailabilityParams`, `GetLeadsInListParams`, `ListIdParam`, `LeadListMembershipsParam`, `LeadOwnerParams`, analytics params.
- `PaginationParams` helper with `page_index()` and `page_size()` (0-based index, capped at 100).

---

### Task 6: server.rs + main.rs + tools/mod.rs + instructions.rs

**What was done:**
- `LsqMcpServer` with `tool_router`, `ensure_client()` (re-reads credentials on every tool call), `get_client()`.
- `validated_output_path()` (now `pub(crate)`): rejects `..` components, strips directory prefix, always resolves inside `~/.lsq-mcp/output/`.
- `success_json()` and `success_json_opt()`: unified response helpers; `success_json_opt` writes to file if `output_file` given or response > 100 KB.
- `api_error()` helper.
- `check_auth()`: wraps tool results with credential-error messages.
- `main.rs`: `configure | status | (default: serve)` CLI dispatch.
- `src/tools/instructions.rs`: static `INSTRUCTIONS` constant with the full AI session guide including `DATA TRUST WARNING` and `STRICT CONSTRAINT` sections.

---

### Task 7: tools/leads.rs — 11 Lead Tools

**Planned (7):** `get_lead_metadata`, `search_leads`, `get_lead_by_id`, `get_lead_by_email`, `get_lead_by_phone`, `get_lead_notes`, `get_lead_activities`.

**Added during implementation (4):** `quick_search_leads`, `get_leads_by_ids`, `get_lead_owner`, `get_recently_modified_leads`.

**Notes:**
- `search_leads` supports a single filter condition (`lookup_name`/`lookup_value`/`operator`). LSQ's `/Leads.Get` endpoint does not support multi-condition arrays on the same call.
- `has_more` is inferred from `count == page_size` since the endpoint does not return a total count.
- `build_paginated_response()` helper is unit-tested.

---

### Task 8: tools/opportunities.rs — 8 Opportunity Tools

**Planned (5):** `get_opportunity_types`, `get_opportunity_metadata`, `get_opportunity_by_id`, `get_opportunities_by_lead`, `search_opportunities`.

**Added during implementation (3):** `is_opportunity_enabled`, `get_opportunities_by_lead_field`, `get_activities_of_opportunity`.

**Notes:**
- `search_opportunities` uses `AdvancedSearch` as a JSON-encoded string (not a raw object) in the POST body — LSQ's API requires this.
- `get_activities_of_opportunity` path is unconfirmed; flagged in `docs/reference/tools.md`.

---

### Task 9: tools/activities.rs + sales.rs — 9 Tools Total

**Activities planned (2):** `get_activity_types`, `get_activities_by_lead`.
**Activities added (4):** `get_activity_details`, `get_activity_owner`, `get_activity_settings`, `get_recently_modified_activities`.
**Sales planned (3):** `get_products`, `get_sales_activity_types`, `get_sales_activities_by_lead`.

**Notes:**
- `get_activities_by_lead` sends `leadId` as a query param on the POST URL — LSQ requires this.
- LSQ caps activity pages at 25 regardless of `page_size`; the implementation enforces `.min(25)`.
- `get_activity_owner`, `get_activity_settings`, `get_recently_modified_activities` paths are unconfirmed; flagged in docs.

---

### Task 10: tools/tasks.rs — 5 Task Tools

`get_task_types`, `get_tasks_by_lead`, `get_tasks_by_owner`, `get_appointments`, `get_todos`.

**Notes:** `get_appointments` and `get_todos` share the same `AppointmentParams` struct (both accept `user_id` or `email`). `get_task_types` uses a best-effort endpoint path — flagged in `client.rs`.

---

### Task 11: tools/users.rs — 6 User Tools

`get_users`, `get_user_by_id`, `search_users`, `get_user_hierarchy`, `get_user_checkin_history`, `get_user_availability`.

**Notes:**
- `get_users` accepts optional `output_file` (large accounts can return 200+ user records).
- `get_user_availability` dispatches to two different endpoints: `ByUserId` when `user_id` given, `ByUserSearchCriteria` when only `email` given; returns `INVALID_PARAMS` if neither provided.
- `get_user_checkin_history` POSTs a body with `UserIds` array and optional `FromDate`/`ToDate`.

---

### Task 12: tools/lists.rs — 4 List Tools

`get_lists`, `get_leads_in_list`, `get_lead_list_memberships`, `get_list_lead_count`.

---

### Task 13: tools/analytics.rs — 4 Analytics Tools

`get_lead_distribution`, `get_leads_not_contacted`, `get_leads_no_active_tasks`, `get_leads_pending_tasks`.

**Notes:** All four use `post_analytics()` which routes to the analytics base URL (no `/v2` prefix) and appends `responseformat=json`. All four take a raw `filters` JSON object passed through directly to the LSQ Analytics API.

---

## 2026-04-17 — Integration Test Infrastructure

**What was done:**
- `Cargo.toml`: added `wiremock = "0.6"` and `tempfile = "3"` to `[dev-dependencies]`.
- `src/client.rs`: added `#[cfg(test)]` fields `test_base_url` / `test_analytics_base_url` to `LsqClient`; added `new_for_testing(creds, server_uri)` constructor that routes v2 calls to `{server_uri}/v2` and analytics calls to `{server_uri}`.
- `src/server.rs`: changed `validated_output_path` visibility from `fn` to `pub(crate) fn` for direct testing.
- `src/lib.rs`: added `pub(crate) static ENV_MUTEX: std::sync::Mutex<()>` (serialises tests that mutate `LSQ_MCP_HOME`) and `#[cfg(test)] mod integration_tests`.

**Next:** Write `src/integration_tests.rs` — the full integration test suite.

---

## 2026-04-17 — Integration Tests Complete (Task 14)

**What was done:**
- `src/integration_tests.rs`: 63 integration tests written and passing. All 82 tests in the suite pass (0 failures).

**Test coverage by area:**
- **Leads (14):** `search_leads` (no filter, filter, empty, has_more true/false, full page), `get_lead_by_id`, `get_lead_by_email`, `get_lead_by_phone`, `get_lead_notes`, `get_lead_activities` (query param on POST URL), `get_leads_by_ids`, `get_lead_owner`, `quick_search_leads`, `get_recently_modified_leads`, metadata caching
- **Opportunities (5):** `get_opportunity_types` (happy + cached), `get_opportunity_by_id`, `get_opportunities_by_lead`, `search_opportunities`
- **Activities (4):** `get_activity_types` (happy + cached), `get_activities_by_lead` (query param on POST URL), `get_recently_modified_activities`
- **Sales (3):** `get_products` (happy + cached), `get_sales_activities_by_lead`
- **Tasks (6):** `get_task_types` (happy + cached), `get_tasks_by_lead`, `get_tasks_by_owner` (body JSON), `get_appointments`, `get_todos`
- **Users (4):** `get_users`, `get_user_by_id`, `search_users` (body JSON), `get_user_hierarchy`
- **Lists (4):** `get_lists`, `get_leads_in_list` (pagination params), `get_lead_list_memberships`, `get_list_lead_count`
- **Analytics (4):** `get_lead_distribution`, `get_leads_not_contacted`, `get_leads_no_active_tasks`, `get_leads_pending_tasks` — all verified against analytics URL (no /v2 prefix)
- **HTTP error handling (4):** 401 → Unauthorized error data, 500 → error data, 429 retries once then succeeds (2 requests), 429 exhausted → RateLimitExhausted (4 requests total)
- **File output (3):** explicit `output_file` written to output dir, auto-threshold triggers at >100KB (200 leads × ~600 chars), directory prefix stripped from `output_file`
- **Path security (4):** plain filename resolves inside output dir, `..` traversal rejected, embedded `..` rejected, directory prefix stripped
- **Cleanup (3):** no-op when dir missing, no-op when under limit, oldest files pruned when over `MAX_OUTPUT_FILES`

**Key implementation decisions for tests:**
- `result_json()` helper: serialises `CallToolResult` via `serde_json::to_value`, extracts `content[0]["text"]`, parses as JSON — avoids needing exact rmcp Content enum API
- wiremock mock order matters: 429 mock registered FIRST with `up_to_n_times(1)`, 200 mock registered second — wiremock tries first-registered first
- `ENV_MUTEX` with `unwrap_or_else(|p| p.into_inner())` recovers from poisoned mutex (prior test panic)
- Auto-threshold test: 200 leads, each with two 300-char strings ≈ 126KB pretty-printed → exceeds 100KB threshold

**Next step:** Task 15 — npm distribution wrapper (`npm/package.json` + `npm/run.js`)

---

## 2026-04-18 — npm Distribution Wrapper Complete (Task 15)

**What was done:**
- `npm/package.json`: minimal manifest — `name: lsq-mcp`, `bin.lsq-mcp: run.js`, `os: ["darwin"]`, `cpu: ["arm64"]`. Version is `0.0.0` in repo; overwritten at publish time by the release workflow.
- `npm/run.js`: 10-line Node wrapper — `execFileSync`s the bundled `lsq-mcp` binary (placed next to `run.js` by CI), passes `process.argv.slice(2)` with `stdio: inherit`, sets `process.exitCode` on failure.
- `.github/workflows/release.yml`: triggered on `v*` tag push. Two jobs:
  1. `build` — runs on `macos-latest`, cross-compiles to `aarch64-apple-darwin`, uploads binary as artifact.
  2. `publish-npm` — downloads artifact into `npm/`, `chmod +x`, copies `README.md`, sets version from tag (strips `v` prefix via `${GITHUB_REF_NAME#v}`), runs `npm publish --access public`.

**Pattern source:** Directly mirrors `newton-mcp` (`/Documents/codebase/newton-mcp`) — same repo, same team. No download-on-install logic; binary is bundled in the package. Simple and proven.

**To release:** Push a `v<semver>` tag (e.g., `git tag v0.1.0 && git push origin v0.1.0`). The workflow handles the rest. Requires `NPM_TOKEN` secret set in GitHub repo settings.

**Next step:** Project is complete for v1. All 15 tasks done.

<!-- Append new entries below this line as modules are completed -->
