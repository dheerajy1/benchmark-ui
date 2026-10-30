// Reusable SVG charts, drawn with Dioxus elements and no charting dependency

mod bar_chart;
mod frame;
mod geometry;
mod line_chart;
mod palette;
mod scale;

pub use bar_chart::{BarChart, BarGroup, BarSeries};
pub use line_chart::{SeriesPoint, TrendChart};
pub use palette::ChartTone;
