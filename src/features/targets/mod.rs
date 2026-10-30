// Saved targets defined on the server: the projects a run can start from by id

mod api;
mod models;

pub use api::fetch_targets;
pub use models::{TargetView, TargetsView};
