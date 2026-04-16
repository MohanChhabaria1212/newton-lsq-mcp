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

## 2026-04-14 — HTTP Auth: Headers over Query Params

**Decision:** Send `x-LSQ-AccessKey` and `x-LSQ-SecretKey` as HTTP headers, not query string parameters.

**Why:** LSQ explicitly recommends headers for security. Headers are not stored in server logs, browser history, or proxy caches. Service CRM endpoints mandate headers — query params would fail for those endpoints.

**Rejected:** Query params — LSQ documents them as an option but recommends against them. Using headers is strictly safer and more compatible.

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

## 2026-04-14 — Module Scope: Core CRM + Analytics (36 tools)

**Decision:** v1 includes Leads, Opportunities, Activities, Sales Activities, Tasks, Users, Lists, and Analytics (4 endpoints). Telephony, Email Marketing, Landing Pages, Portal API, Service CRM, and Async API are deferred.

**Why:** The included modules cover 100% of CRM data queries for all three user types (sales reps, managers, developers/admins). Excluded modules are either write-only (Async API), separate product lines (Service CRM, Portal API), or have low AI utility (Telephony's 2 endpoints retrieve an SSO key and an owner phone number — not useful in conversation).

---

## 2026-04-14 — Multiple Accounts: Single Profile in v1

**Decision:** v1 supports one credential set per machine. Running `lsq-mcp configure` overwrites the current credentials. Named profiles (`--profile`) are deferred to v2.

**Why:** Keeps the configure flow simple for the majority of users who have one LSQ account. Named profiles add complexity (file naming, profile selection flags) that is premature for v1.
