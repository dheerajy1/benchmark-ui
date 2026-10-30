// Run detail: the full stored result of one benchmark run

mod api;
mod models;
mod sections;
mod view;

pub use api::fetch_run;
pub use models::BenchmarkResult;
pub use view::RunDetail;
