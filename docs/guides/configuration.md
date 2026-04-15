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
1. Prompt for your Access Key
2. Prompt for your Secret Key
3. Prompt for your API host (press Enter for the default `api.leadsquared.com`)
4. Make a test API call to validate the credentials
5. Display your connected account email
6. Save credentials to `~/.lsq-mcp/credentials.json` (permissions: `0600`)

Nothing is saved if validation fails — you can safely re-run configure as many times as needed.

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

| Region | Host |
|---|---|
| India / Global (default) | `api.leadsquared.com` |
| US | `api-us.leadsquared.com` |
| AU | `api-au.leadsquared.com` |

Enter the correct host for your account region during `lsq-mcp configure`.

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
