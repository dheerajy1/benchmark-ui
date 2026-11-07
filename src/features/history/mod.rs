// Run history: paginated list with filters and comparison selection

mod api;
mod columns;
mod columns_menu;
mod filters;
mod models;
mod run_row;
mod selection;
mod view;

pub use api::{fetch_page, fetch_recent_runs, HistoryFilters};
pub use columns::Column;
pub use models::{runtime_label, RunSummary};
pub use view::History;
