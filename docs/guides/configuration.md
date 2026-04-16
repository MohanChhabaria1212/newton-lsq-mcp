# Configuration Guide

## Getting your API keys

1. Log in to your LeadSquared account
2. Go to **My Account → Settings → API and Webhooks**
3. Copy your **Access Key** and **Secret Key**

LSQ recommends using admin credentials for full team-wide data access. If you use non-admin keys, some tools (e.g. `get_users`, `get_user_hierarchy`) may return limited results.

## Running configure

```bash
lsq-mcp configure
```

The wizard will:
1. Prompt for your LSQ email address
2. Prompt for your Access Key
3. Prompt for your Secret Key
4. Prompt for your LSQ portal URL (e.g. `https://app.in21.leadsquared.com/leads`) — the URL you open in your browser. The API host is derived automatically from this.
5. Validate credentials against the LSQ API
6. Display the matched account (name, email, role) and ask you to confirm
7. Save credentials to `~/.lsq-mcp/credentials.json` (permissions: `0600`) only after confirmation

Nothing is saved if validation fails or you decline the confirmation prompt — you can safely re-run configure as many times as needed.

## Checking current config

```bash
lsq-mcp status
```

Shows the configured host and a masked version of your Access Key. The Secret Key is never displayed.

## Updating credentials

Re-run `lsq-mcp configure`. The new credentials replace the old ones only if validation succeeds.

## Credential file location

Credentials are stored at `~/.lsq-mcp/credentials.json`. You can override this location by setting the `LSQ_MCP_HOME` environment variable:

```bash
export LSQ_MCP_HOME=/custom/path
lsq-mcp configure
```

## Regional hosts

You don't need to know your API host. During `lsq-mcp configure`, paste the URL you normally open in your browser (e.g. `https://app.in21.leadsquared.com/leads`) and the correct API host is derived automatically.

Supported portal URL patterns:

| Browser URL | Derived API host |
|---|---|
| `app.leadsquared.com` | `api.leadsquared.com` |
| `app.in21.leadsquared.com` | `api-in21.leadsquared.com` |
| `app-us.leadsquared.com` | `api-us.leadsquared.com` |
| `app-au.leadsquared.com` | `api-au.leadsquared.com` |

Any `app.{cluster}.leadsquared.com` or `app-{region}.leadsquared.com` URL is handled automatically.

## MCP client setup

### Claude Desktop

Edit `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "lsq": {
      "command": "lsq-mcp"
    }
  }
}
```

Restart Claude Desktop after saving.

### Claude Code

```bash
claude mcp add lsq -- lsq-mcp
```
