# Filters and Date Formats

## Date format

All date parameters must be in **UTC** using this exact format:

```
YYYY-MM-DD HH:MM:SS
```

Examples:
- `2024-01-15 00:00:00` — start of January 15, 2024 UTC
- `2024-01-15 23:59:59` — end of January 15, 2024 UTC

> **Important:** LSQ stores all timestamps in UTC. If your users are in IST (+5:30), a lead created at 10:00 AM IST was created at 04:30 UTC. Adjust accordingly.

## Lead search filters

Used by `search_leads`. Pass as a JSON array of conditions:

```json
[
  {"Attribute": "FieldName", "Operator": "eq", "Value": "value"},
  {"Attribute": "AnotherField", "Operator": "gt", "Value": "2024-01-01 00:00:00"}
]
```

### Operators

| Operator | Meaning |
|---|---|
| `eq` | Equals |
| `neq` | Not equals |
| `gt` | Greater than |
| `lt` | Less than |
| `gte` | Greater than or equal |
| `lte` | Less than or equal |
| `contains` | Contains substring |
| `startswith` | Starts with |

### Getting field names

Call `get_lead_metadata` first — it returns all field names including custom fields specific to your account. Standard fields include `EmailAddress`, `Phone`, `FirstName`, `LastName`, `LeadStage`, `OwnerId`, `CreatedOn`, `ModifiedOn`.

### Example: Leads in a stage assigned to a user

```json
[
  {"Attribute": "LeadStage", "Operator": "eq", "Value": "Contacted"},
  {"Attribute": "OwnerId", "Operator": "eq", "Value": "user-guid-here"}
]
```

### Example: Leads created in a date range

```json
[
  {"Attribute": "CreatedOn", "Operator": "gte", "Value": "2024-01-01 00:00:00"},
  {"Attribute": "CreatedOn", "Operator": "lt", "Value": "2024-02-01 00:00:00"}
]
```

## Pagination

All list/search tools support `page` (1-based, default 1) and `page_size` (default 25, max 100).

Every paginated response includes:

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

## Analytics filters

Analytics tools (`get_lead_distribution`, `get_leads_not_contacted`, etc.) use a different filter schema defined by LSQ's analytics API. Pass the full filter body as the `filters` parameter. Refer to the [LSQ Analytics API documentation](https://apidocs.leadsquared.com) for the exact schema.

These tools require **Elasticsearch to be enabled** on your LSQ account. Contact LSQ support if you get an Elasticsearch error.
