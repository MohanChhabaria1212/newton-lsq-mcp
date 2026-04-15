# lsq-mcp

A read-only MCP (Model Context Protocol) server for LeadSquared CRM. Exposes LSQ data — leads, opportunities, activities, tasks, users, lists, and analytics — to AI assistants like Claude Desktop and Claude Code.

## What it does

Connect your LSQ account once with `lsq-mcp configure`, then ask your AI assistant natural-language questions about your CRM data:

- "Show me all leads in the 'Contacted' stage assigned to Priya"
- "What opportunities does lead john@acme.com have open?"
- "Which users have no active tasks this week?"
- "Give me the lead distribution by owner for this month"

## Quick start

### 1. Install

**Via npm (recommended):**
```bash
npm install -g lsq-mcp
```

**Via cargo:**
```bash
cargo install lsq-mcp
```

### 2. Configure

```bash
lsq-mcp configure
```

You'll be prompted for your LSQ Access Key and Secret Key. Find them at:
**LSQ Portal → My Account → Settings → API and Webhooks**

The setup validates your credentials live and shows your connected account before saving.

### 3. Add to your MCP client

**Claude Desktop** (`~/Library/Application Support/Claude/claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "lsq": {
      "command": "lsq-mcp"
    }
  }
}
```

**Claude Code:**
```bash
claude mcp add lsq -- lsq-mcp
```

## Commands

| Command | Description |
|---|---|
| `lsq-mcp` | Start MCP server (used by MCP clients) |
| `lsq-mcp configure` | Set up or update LSQ API credentials |
| `lsq-mcp status` | Show current configuration |

## Available tools (36 total)

| Module | Tools |
|---|---|
| **Leads** | `get_lead_metadata`, `search_leads`, `get_lead_by_id`, `get_lead_by_email`, `get_lead_by_phone`, `get_lead_notes`, `get_lead_activities` |
| **Opportunities** | `get_opportunity_types`, `get_opportunity_metadata`, `get_opportunity_by_id`, `get_opportunities_by_lead`, `search_opportunities` |
| **Activities** | `get_activity_types`, `get_activities_by_lead` |
| **Sales Activities** | `get_products`, `get_sales_activity_types`, `get_sales_activities_by_lead` |
| **Tasks** | `get_task_types`, `get_tasks_by_lead`, `get_tasks_by_owner`, `get_appointments`, `get_todos` |
| **Users** | `get_users`, `get_user_by_id`, `search_users`, `get_user_hierarchy`, `get_user_checkin_history`, `get_user_availability` |
| **Lists** | `get_lists`, `get_leads_in_list`, `get_lead_list_memberships`, `get_list_lead_count` |
| **Analytics** *(requires Elasticsearch)* | `get_lead_distribution`, `get_leads_not_contacted`, `get_leads_no_active_tasks`, `get_leads_pending_tasks` |

Call `get_instructions` first — it describes all tools, recommended call sequences, and date format requirements.

## Regional hosts

By default `lsq-mcp` connects to `api.leadsquared.com` (India/global). If your account is on a different region, enter the correct host during `lsq-mcp configure`:

| Region | Host |
|---|---|
| India / Global | `api.leadsquared.com` |
| US | `api-us.leadsquared.com` |
| AU | `api-au.leadsquared.com` |

## Requirements

- LeadSquared account with API access
- Admin credentials recommended for full team-wide data access

## Docs

- [Configuration guide](docs/guides/configuration.md)
- [Tools reference](docs/reference/tools.md)
- [Date & filter formats](docs/guides/filters-and-dates.md)

## License

MIT
