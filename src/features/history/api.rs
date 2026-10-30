use super::models::HistoryPage;
use crate::features::connection::{get_json, ApiError};

const PATH: &str = "/benchmarks";

// Runtime lookup codes accepted by the `runtime` filter, taken from the backend's seeded lookup data
pub const RUNTIMES: [(&str, &str); 5] = [
    ("bun", "Bun"),
    ("node", "Node.js"),
    ("python", "Python"),
    ("rust", "Rust"),
    ("go", "Go"),
];

// Page size of the list, the backend accepts 1 to 100
pub const PAGE_SIZE: u32 = 20;
const MAX_PAGE_SIZE: u32 = 100;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct HistoryFilters {
    pub runtime: String,
    pub target_path: String,
}

// Query string of GET /benchmarks, empty filters are left out
fn query_pairs(
    filters: &HistoryFilters,
    before: Option<&str>,
    limit: u32,
) -> Vec<(&'static str, String)> {
    let mut query = vec![("limit", limit.clamp(1, MAX_PAGE_SIZE).to_string())];

    if !filters.runtime.is_empty() {
        query.push(("runtime", filters.runtime.clone()));
    }

    let target_path = filters.target_path.trim();
    if !target_path.is_empty() {
        query.push(("target_path", target_path.to_string()));
    }

    if let Some(cursor) = before {
        query.push(("before", cursor.to_string()));
    }

    query
}

// GET /api/v1/benchmarks
pub async fn fetch_page(
    filters: &HistoryFilters,
    before: Option<&str>,
    limit: u32,
) -> Result<HistoryPage, ApiError> {
    get_json(PATH, &query_pairs(filters, before, limit)).await
}

// Newest runs without filters, used by the comparison pickers
pub async fn fetch_recent_runs() -> Result<HistoryPage, ApiError> {
    fetch_page(&HistoryFilters::default(), None, MAX_PAGE_SIZE).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omits_empty_filters() {
        let query = query_pairs(&HistoryFilters::default(), None, 20);

        assert_eq!(query, vec![("limit", "20".to_string())]);
    }

    #[test]
    fn includes_filters_and_cursor() {
        let filters = HistoryFilters {
            runtime: "bun".to_string(),
            target_path: "  /home/me/api  ".to_string(),
        };

        let query = query_pairs(&filters, Some("MTc5"), 50);

        assert_eq!(
            query,
            vec![
                ("limit", "50".to_string()),
                ("runtime", "bun".to_string()),
                ("target_path", "/home/me/api".to_string()),
                ("before", "MTc5".to_string()),
            ]
        );
    }

    #[test]
    fn clamps_the_limit_to_the_documented_range() {
        assert_eq!(query_pairs(&HistoryFilters::default(), None, 0)[0].1, "1");
        assert_eq!(
            query_pairs(&HistoryFilters::default(), None, 500)[0].1,
            "100"
        );
    }
}
