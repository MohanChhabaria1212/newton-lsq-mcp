use schemars::JsonSchema;
use serde::Deserialize;

// ── Pagination ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PaginationParams {
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 100.
    pub page_size: Option<u32>,
}

impl PaginationParams {
    pub fn page_index(&self) -> u32 {
        self.page.unwrap_or(1).saturating_sub(1)
    }
    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(25).min(100)
    }
}

// ── Lead params ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchLeadsParams {
    /// Field name (LSQ schema name) to filter on, e.g. "ProspectStage", "EmailAddress", "CreatedOn", "OwnerId".
    /// Call get_lead_metadata first to discover valid field schema names (especially custom fields starting with mx_).
    /// Leave blank (omit) to retrieve all leads without filtering.
    pub lookup_name: Option<String>,
    /// Value to match. For dates use UTC format: "YYYY-MM-DD HH:MM:SS".
    pub lookup_value: Option<String>,
    /// SQL comparison operator: =, LIKE, >, <, <=, >=, <>. Default: =
    /// Use LIKE for partial string match, = for exact match.
    pub operator: Option<String>,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 1000.
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LeadIdParam {
    /// The LeadSquared ProspectID (GUID) of the lead.
    pub lead_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LeadEmailParam {
    /// Email address of the lead to look up.
    pub email: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LeadPhoneParam {
    /// Phone number of the lead to look up.
    pub phone: String,
}

/// Bulk-fetch leads by a list of ProspectIDs.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetLeadsByIdsParams {
    /// List of ProspectID GUIDs to fetch (max 10,000).
    pub lead_ids: Vec<String>,
    /// Comma-separated field names to include. Leave blank for all fields.
    pub columns: Option<String>,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 1000.
    pub page_size: Option<u32>,
}

/// Full-text quick search across name, email, phone, company, city, country.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct QuickSearchLeadsParams {
    /// Search term (searches FirstName, LastName, EmailAddress, Phone, Mobile, Company, City, Country).
    pub key: String,
}

/// Get the owner of a lead by looking up via any unique lead field.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LeadOwnerParams {
    /// Lead field name to search on (e.g. "EmailAddress", "LeadId", "Phone").
    pub lead_identifier: String,
    /// Value of the field.
    pub value: String,
}

/// Get leads modified within a date range.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RecentlyModifiedLeadsParams {
    /// Start of modification window (UTC, YYYY-MM-DD HH:MM:SS).
    pub from_date: String,
    /// End of modification window (UTC, YYYY-MM-DD HH:MM:SS).
    pub to_date: String,
    /// Comma-separated field names to return. Leave blank for all fields.
    pub columns: Option<String>,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 100. Maximum: 1000.
    pub page_size: Option<u32>,
}

// ── Opportunity params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OpportunityIdParam {
    /// The LeadSquared Opportunity ID.
    pub opportunity_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OpportunityMetadataParams {
    /// Opportunity type ID from get_opportunity_types.
    pub opportunity_type_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchOpportunitiesParams {
    /// Opportunity type event code from get_opportunity_types (e.g. 12005).
    pub opportunity_type_code: Option<i64>,
    /// Filter conditions as JSON. Format: {"GrpConOp":"And","Conditions":[{"Attribute":"FieldName","Operator":"eq","Value":"..."}]}
    pub advanced_search: Option<serde_json::Value>,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 100.
    pub page_size: Option<u32>,
}

/// Get opportunities by matching a unique lead field value.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetOpportunitiesByLeadFieldParams {
    /// Lead field name to search on (e.g. "Mobile", "EmailAddress").
    pub lookup_name: String,
    /// Value to match.
    pub lookup_value: String,
    /// SQL operator: =, LIKE, >, <, <=, >=, <>. Default: =
    pub operator: Option<String>,
    /// Comma-separated field names to include in response.
    pub columns: Option<String>,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 100.
    pub page_size: Option<u32>,
}

/// Check whether the Opportunity feature is enabled for an organisation.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct IsOpportunityEnabledParams {
    /// Organisation ID from your LSQ account.
    pub org_id: String,
}

// ── Activity params ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ActivityIdParam {
    /// The LeadSquared Activity ID (GUID).
    pub activity_id: String,
}

/// Get activities modified within a date range (unconfirmed path — verify if endpoint returns 404).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RecentlyModifiedActivitiesParams {
    /// Start of modification window (UTC, YYYY-MM-DD HH:MM:SS).
    pub from_date: String,
    /// End of modification window (UTC, YYYY-MM-DD HH:MM:SS).
    pub to_date: String,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25.
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ActivitiesByLeadParams {
    /// The LeadSquared ProspectID of the lead.
    pub lead_id: String,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 100.
    pub page_size: Option<u32>,
}

// ── Sales activity params ─────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SalesActivitiesByLeadParams {
    /// The LeadSquared ProspectID of the lead.
    pub lead_id: String,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 100.
    pub page_size: Option<u32>,
}

// ── Task params ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskIdParam {
    /// The LeadSquared Task ID.
    pub task_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TasksByLeadParams {
    /// The LeadSquared ProspectID of the lead.
    pub lead_id: String,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 100.
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TasksByOwnerParams {
    /// User ID of the task owner.
    pub owner_id: String,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 100.
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AppointmentParams {
    /// User ID to filter appointments for.
    pub user_id: Option<String>,
    /// User email to filter appointments for.
    pub email: Option<String>,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 100.
    pub page_size: Option<u32>,
}

// ── User params ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UserIdParam {
    /// The LeadSquared User ID.
    pub user_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchUsersParams {
    /// JSON array of filter conditions on user fields.
    pub filters: Option<serde_json::Value>,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 100.
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UserHierarchyParams {
    /// Manager's User ID. Returns all users in their reporting chain.
    pub manager_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckInHistoryParams {
    /// User ID to retrieve check-in history for.
    pub user_id: String,
    /// From date (UTC, YYYY-MM-DD HH:MM:SS).
    pub from_date: Option<String>,
    /// To date (UTC, YYYY-MM-DD HH:MM:SS).
    pub to_date: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AvailabilityParams {
    /// User ID to check availability for.
    pub user_id: Option<String>,
    /// User email to check availability for.
    pub email: Option<String>,
}

// ── List params ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListIdParam {
    /// The LeadSquared List ID.
    pub list_id: String,
    /// Page number (1-based). Default: 1.
    pub page: Option<u32>,
    /// Results per page. Default: 25. Maximum: 100.
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LeadListMembershipsParam {
    /// The LeadSquared ProspectID of the lead.
    pub lead_id: String,
}

// ── Analytics params ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LeadDistributionParams {
    /// JSON filter body following LSQ Lead Distribution API schema.
    /// Supports UserFilter, LeadFilters, DateFilter, and Aggregate fields.
    /// All dates must be UTC in YYYY-MM-DD HH:MM:SS format.
    pub filters: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LeadsNotContactedParams {
    /// JSON filter body following LSQ Leads Not Contacted API schema.
    /// Supports UserFilter, LeadFilters, ActivityFilters, DateFilter.
    /// All dates must be UTC in YYYY-MM-DD HH:MM:SS format.
    pub filters: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LeadsNoActiveTasksParams {
    /// JSON filter body following LSQ Leads With No Active Tasks API schema.
    /// Supports UserFilter, LeadFilters, TaskFilters, DateFilter.
    pub filters: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LeadsPendingTasksParams {
    /// JSON filter body following LSQ Leads With Pending Tasks API schema.
    /// Supports UserFilter, LeadFilters, TaskFilters (Pending/Overdue/PendingAndOverdue), DateFilter.
    pub filters: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_defaults() {
        let p = PaginationParams { page: None, page_size: None };
        assert_eq!(p.page_index(), 0);
        assert_eq!(p.page_size(), 25);
    }

    #[test]
    fn pagination_caps_at_100() {
        let p = PaginationParams { page: Some(1), page_size: Some(500) };
        assert_eq!(p.page_size(), 100);
    }

    #[test]
    fn pagination_page_index_is_zero_based() {
        let p = PaginationParams { page: Some(3), page_size: Some(10) };
        assert_eq!(p.page_index(), 2);
    }
}
