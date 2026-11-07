// Reusable SVG charts, drawn with Dioxus elements and no charting dependency

mod bar_chart;
mod compare_trend;
mod diverging_bars;
mod frame;
mod geometry;
mod horizontal_bars;
mod line_chart;
mod palette;
mod range_bars;
mod scale;
mod sparkline;
mod stacked_bars;
mod trend_tile;

pub use bar_chart::{BarChart, BarGroup, BarSeries};
pub use compare_trend::{align_end, CompareSeries, CompareTrendChart};
pub use diverging_bars::{DivergeRow, DivergingBars};
pub use horizontal_bars::{BarRow, HorizontalBars};
pub use line_chart::{Reference, SeriesPoint, TrendChart};
pub use palette::ChartTone;
pub use range_bars::{RangeBars, RangeRow};
pub use sparkline::Sparkline;
pub use stacked_bars::{Segment, StackRow, StackedBars};
pub use trend_tile::{ago_labels, split_unit, TrendTile};
