# LeadSquared MCP — Decisions Log

Running log of every non-obvious decision made during design and implementation. Append-only — never edit past entries. Each entry answers: what was decided, why, and what was considered and rejected.

---

## 2026-04-14 — Project Scope: Read-Only for v1

**Decision:** v1 is read-only. No write operations (create lead, log activity, etc.).

**Why:** Proves value quickly, eliminates risk of accidental data mutation via an AI tool, and the read surface alone is large enough to be genuinely useful to all three user types (sales reps, managers, developers).

**Rejected:** Full read-write from day one. Risk of AI misuse outweighs the benefit at this stage.

---

## 2026-04-14 — Transport: stdio over HTTP

**Decision:** Use stdio MCP transport, not HTTP Streamable.

**Why:** Each user installs the binary locally and runs it on their own machine with their own LSQ credentials. stdio is the standard for locally-installed MCP servers and is supported by all MCP clients (Claude Desktop, Claude Code, etc.). No hosting infrastructure required.

**Rejected:** HTTP Streamable hosted server. Would require infrastructure, multi-tenant session management, and a way to securely pass per-user credentials to a shared server. Unnecessary complexity given that LSQ credentials are per-user and the binary is lightweight enough to run locally.

---

## 2026-04-14 — Auth: Interactive CLI Configure + Credential File

**Decision:** `lsq-mcp configure` prompts for keys interactively and saves to `~/.lsq-mcp/credentials.json` with `0o600` permissions. Validates credentials with a live API call before saving. Displays connected user name, email, and role on success.

**Why:** Simpler than env vars for non-technical users. More user-friendly than editing JSON config files. The validation step guarantees the file is never written in a broken state. Showing the user their account details confirms they used the right keys.

**Rejected:** Environment variables only — requires editing MCP client config JSON, not accessible to all users. OAuth device flow — unnecessary since LSQ already issues static API keys.

---

## 2026-04-14 — HTTP Auth: Headers over Query Params (design intent)

**Design intent:** Send `x-LSQ-AccessKey` and `x-LSQ-SecretKey` as HTTP headers, not query string parameters.

**Why headers were preferred in design:** LSQ recommends headers for security. Headers are not stored in server logs or proxy caches. Service CRM endpoints mandate headers.

> ⚠️ **REVERSED during implementation (2026-04-15) — see entry below.**

---

## 2026-04-15 — HTTP Auth: Switched to Query Params

**Decision:** Reversed the header auth design. The implementation (`src/client.rs`) sends `accessKey` and `secretKey` as URL query parameters on every request.

**Why the switch:** Testing against the live LSQ API revealed that the standard v2 endpoints (`/LeadManagement.svc/...`, `/ProspectActivity.svc/...`, etc.) do not accept `x-LSQ-AccessKey` / `x-LSQ-SecretKey` headers — they only recognise `accessKey`/`secretKey` as query parameters. The header approach would have silently failed against real LSQ accounts.

**Security mitigation:** The URL (which contains the keys as query params) is built once before the retry loop and is **never logged**. All `tracing::debug!` statements in `client.rs` that reference rate-limit retries log only timing information — not the URL. This prevents credential leakage at `RUST_LOG=debug`. See `docs/reference/security.md §5`.

**Rejected alternative kept in reserve:** If LSQ's Service CRM or other modules require header auth in a future v2, a per-request header injection path can be added to `LsqClient` without changing the retry loop.

---

## 2026-04-14 — Rate Limiting: Transparent Retry

**Decision:** 429 responses are retried automatically inside `LsqClient`. Strategy: read `Retry-After` header if present, else exponential backoff (1s → 2s → 4s). Max 3 retries. Only surfaced to the user if all 3 retries fail.

**Why:** The user (and the AI) should never need to know about rate limits or retry logic. This is a plumbing concern that belongs in the HTTP layer, not in tool responses. Transparent retry gives a much better UX with zero cost to the caller.

**Rejected:** Surfacing 429 directly as a tool error — forces the AI to reason about retry timing, adds noise to every session, and provides no value to end users.

---

## 2026-04-14 — Pagination: Default 25, Max 100, Always Include Metadata

**Decision:** All list/search tools default to page_size=25, max 100. Every paginated response includes `total_count`, `page`, `page_size`, and `has_more`. No silent auto-pagination.

**Why:** Protects the AI's context window from being flooded by large datasets. LSQ accounts can have tens of thousands of leads. `has_more` + `total_count` gives the AI everything it needs to decide whether to fetch more pages or ask the user to narrow their query.

**Rejected:** Unlimited response size — would routinely overflow AI context windows. Auto-pagination — silently fetching all pages could produce multi-megabyte responses and cause unpredictable behaviour.

---

## 2026-04-14 — Caching: In-Memory for Stable Data

**Decision:** Lead metadata, activity types, opportunity types, task types, and products are cached in memory for the lifetime of the server process using `Arc<RwLock<Option<Value>>>`.

**Why:** These datasets change rarely (field schemas, type lists, product catalogues). Caching eliminates redundant API calls on every tool invocation, reduces latency, and avoids rate limit consumption on high-frequency operations like checking field names before a search.

**Rejected:** No caching — would hit the LSQ API on every tool call even for data that never changes within a session. Redis or external cache — overkill for a local process; in-memory is sufficient and has no external dependency.

---

## 2026-04-14 — Credential Reload: On Every ensure_client() Call

**Decision:** `ensure_client()` re-reads credentials from disk on every invocation rather than caching the credential object indefinitely.

**Why:** A user who runs `lsq-mcp configure` to update expired or rotated keys while the server is running should not need to restart the server. Disk reads are cheap relative to network round-trips and this eliminates an entire class of "stale credential" bugs.

**Rejected:** Cache credentials at startup only — requires server restart after any credential change, which is a bad UX for a background process.

---

## 2026-04-14 — Module Scope: Core CRM + Analytics (36 tools planned)

**Decision:** v1 includes Leads, Opportunities, Activities, Sales Activities, Tasks, Users, Lists, and Analytics (4 endpoints). Telephony, Email Marketing, Landing Pages, Portal API, Service CRM, and Async API are deferred.

**Why:** The included modules cover 100% of CRM data queries for all three user types (sales reps, managers, developers/admins). Excluded modules are either write-only (Async API), separate product lines (Service CRM, Portal API), or have low AI utility (Telephony's 2 endpoints retrieve an SSO key and an owner phone number — not useful in conversation).

> **Updated 2026-04-15:** Final implementation exposed 48 tools, not 36. 12 additional utility tools were added during module implementation — see entry below.

---

## 2026-04-14 — Multiple Accounts: Single Profile in v1

**Decision:** v1 supports one credential set per machine. Running `lsq-mcp configure` overwrites the current credentials. Named profiles (`--profile`) are deferred to v2.

**Why:** Keeps the configure flow simple for the majority of users who have one LSQ account. Named profiles add complexity (file naming, profile selection flags) that is premature for v1.

---

## 2026-04-15 — Credential Encryption: AES-256-GCM at Rest

**Decision:** Credentials are encrypted on disk using AES-256-GCM. A 32-byte random key is stored at `~/.lsq-mcp/.key` (0o600). The credentials file stores `{v, n, ct}` (version, nonce, ciphertext). A fresh random 96-bit nonce is generated on every write.

**Why:** The original spec stored credentials as plaintext JSON (0o600). During implementation it became clear that plaintext is easily exfiltrated by cloud sync (iCloud, Dropbox) picking up `~/.lsq-mcp/credentials.json`. Encryption at rest mitigates passive exfiltration while keeping the UX identical (no passphrase required).

**Rejected:** Keychain / OS secure enclave — adds platform-specific dependencies and complicates the Linux/Docker use case. The two-file model (key + ciphertext) is sufficient for the threat model since an attacker needs both files.

**v1 → v2 migration:** Plaintext credentials files from pre-encryption installs are silently re-encrypted to v2 on next load. No user action required.

---

## 2026-04-15 — Tool Count Expanded: 36 → 48

**Decision:** 12 additional tools were added during module implementation beyond the original 36 planned:

| Module | Added tools |
|---|---|
| Leads | `quick_search_leads`, `get_leads_by_ids`, `get_lead_owner`, `get_recently_modified_leads` |
| Opportunities | `is_opportunity_enabled`, `get_opportunities_by_lead_field`, `get_activities_of_opportunity` |
| Activities | `get_activity_details`, `get_activity_owner`, `get_activity_settings`, `get_recently_modified_activities` |

**Why:** These endpoints were discovered in LSQ's API documentation while implementing the planned tools and added incrementally. Each fills a real gap: `quick_search_leads` for identity lookups, `get_recently_modified_*` for sync workflows, `is_opportunity_enabled` for pre-flight checks.

**Note:** Three of the activity endpoints (`get_activity_owner`, `get_activity_settings`, `get_recently_modified_activities`) and `get_activities_of_opportunity` have unconfirmed paths — added as best-effort and flagged in `docs/reference/tools.md`.

---

## 2026-04-15 — Output File Support + Auto-Threshold

**Decision:** All paginated/list tools accept an optional `output_file` parameter. Any tool response exceeding 100 KB is automatically written to `~/.lsq-mcp/output/` regardless of whether `output_file` was specified.

**Why:** LSQ accounts can have thousands of leads. Without file output, large responses either overflow the AI's context window or get truncated. Auto-threshold (100 KB) means the AI never needs to manually manage output for large result sets — it just receives a file path.

**Security:** `output_file` values are validated before any write: `..` components are rejected, and only the filename component is used (directory prefix stripped). The AI cannot write outside `~/.lsq-mcp/output/`. After every write the directory is capped at 100 files (oldest deleted). See `src/server.rs: validated_output_path()` and `src/config.rs: cleanup_output_dir()`.

---

## 2026-04-15 — Integration Tests in src/ Not tests/

**Decision:** Integration tests live in `src/integration_tests.rs` (registered as `#[cfg(test)] mod integration_tests` in `lib.rs`), not in the `tests/` directory.

**Why:** The `tests/` directory compiles the crate in library mode with `#[cfg(test)]` disabled. This means `LsqClient::new_for_testing` (a `#[cfg(test)]`-only constructor) and `pub(crate)` items like `validated_output_path` are inaccessible from `tests/`. Moving tests into `src/` gives them full access to test-only and crate-private items.

**Rejected:** `tests/` with a re-export shim — adds unnecessary complexity and surface area.
