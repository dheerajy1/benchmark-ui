// Run history: paginated list with filters and comparison selection

mod api;
mod models;
mod run_row;
mod selection;
mod view;

pub use api::fetch_recent_runs;
pub use models::{RunMetrics, RunSummary};
pub use view::History;
