use super::models::OverviewData;
use crate::features::connection::{get_json, ApiError};

// GET /api/v1/benchmarks/overview for one project (a stored target path). Without a project the
// API shows the project of the newest run.
pub async fn fetch_overview(project: &str) -> Result<OverviewData, ApiError> {
    get_json("/benchmarks/overview", &query(project)).await
}

fn query(project: &str) -> Vec<(&'static str, String)> {
    let project = project.trim();

    if project.is_empty() {
        Vec::new()
    } else {
        vec![("target_path", project.to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sends_the_project_as_target_path() {
        assert_eq!(
            query(" /srv/shop "),
            vec![("target_path", "/srv/shop".to_string())]
        );
    }

    #[test]
    fn sends_nothing_without_a_project() {
        assert!(query("  ").is_empty());
    }
}
