use dioxus::prelude::*;

use super::{
    palette::ChartTone,
    scale::{Domain, LinearScale},
};
use crate::format::ValueFormat;

// Chart geometry in SVG user units, the viewBox scales it to the available width
pub const WIDTH: f64 = 480.0;
pub const HEIGHT: f64 = 220.0;
pub const MARGIN_LEFT: f64 = 60.0;
pub const MARGIN_RIGHT: f64 = 16.0;
pub const MARGIN_TOP: f64 = 16.0;
pub const MARGIN_BOTTOM: f64 = 30.0;

pub fn inner_width() -> f64 {
    WIDTH - MARGIN_LEFT - MARGIN_RIGHT
}

pub fn inner_height() -> f64 {
    HEIGHT - MARGIN_TOP - MARGIN_BOTTOM
}

pub fn baseline_y() -> f64 {
    HEIGHT - MARGIN_BOTTOM
}

pub fn view_box() -> String {
    format!("0 0 {WIDTH} {HEIGHT}")
}

// Vertical scale from a value domain to pixel rows, larger values are higher up
pub fn y_scale(domain: &Domain) -> LinearScale {
    LinearScale::new(domain.min, domain.max, baseline_y(), MARGIN_TOP)
}

// Horizontal gridlines with their value labels
#[component]
pub fn ChartGrid(domain: Domain, format: ValueFormat) -> Element {
    let scale = y_scale(&domain);
    let line_end = WIDTH - MARGIN_RIGHT;
    let label_x = MARGIN_LEFT - 8.0;

    let rows: Vec<(f64, String)> = domain
        .ticks
        .iter()
        .map(|tick| (scale.map(*tick), format.format_axis(*tick)))
        .collect();

    rsx! {
        g {
            for (y , label) in rows {
                line {
                    x1: "{MARGIN_LEFT}",
                    x2: "{line_end}",
                    y1: "{y:.2}",
                    y2: "{y:.2}",
                    class: "stroke-(color:--clr-border)",
                }
                text {
                    x: "{label_x}",
                    y: "{y:.2}",
                    dy: "0.32em",
                    text_anchor: "end",
                    class: "fill-(--clr-text-faint) text-(length:--fs-xs) tabular-nums",
                    "{label}"
                }
            }
        }
    }
}

// Shown instead of a chart when no point has a value
#[component]
pub fn ChartEmpty(#[props(into)] message: String) -> Element {
    rsx! {
        div { class: "flex h-(--chart-height) items-center justify-center rounded-(--round-md) border border-dashed border-(color:--clr-border-strong) px-(--space-4) text-center text-(length:--fs-sm) text-(color:--clr-text-faint)",
            "{message}"
        }
    }
}

// Color key above a multi-series chart
#[component]
pub fn ChartLegend(entries: Vec<(String, ChartTone)>) -> Element {
    let items: Vec<(String, &'static str)> = entries
        .into_iter()
        .map(|(name, tone)| (name, tone.swatch_class()))
        .collect();

    rsx! {
        ul { class: "flex flex-wrap items-center gap-(--space-4) pb-(--space-2)",
            for (name , swatch) in items {
                li { class: "flex items-center gap-(--space-2) text-(length:--fs-xs) text-(color:--clr-text-muted)",
                    span { class: "inline-block size-(--dot-size) rounded-(--round-full) {swatch}" }
                    "{name}"
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TooltipLine {
    pub label: String,
    pub value: String,
    pub swatch: Option<ChartTone>,
}

// Tooltip drawn inside the SVG so it scales with the chart and needs no inline styles
#[component]
pub fn SvgTooltip(anchor_x: f64, heading: String, lines: Vec<TooltipLine>) -> Element {
    // Approximate glyph width used to size the box, SVG cannot measure text before layout
    const CHAR_WIDTH: f64 = 6.8;
    const PADDING: f64 = 10.0;
    const HEADER_HEIGHT: f64 = 24.0;
    const ROW_HEIGHT: f64 = 16.0;
    const SWATCH_INDENT: f64 = 14.0;
    const MIN_WIDTH: f64 = 96.0;
    const OFFSET: f64 = 12.0;

    let widest_row = lines
        .iter()
        .map(|line| line.label.chars().count() + line.value.chars().count() + 3)
        .max()
        .unwrap_or(0);
    let widest = widest_row.max(heading.chars().count());

    let width = (widest as f64 * CHAR_WIDTH + PADDING * 2.0 + SWATCH_INDENT).max(MIN_WIDTH);
    let height = HEADER_HEIGHT + ROW_HEIGHT * lines.len() as f64 + PADDING - 2.0;

    let mut left = anchor_x + OFFSET;
    if left + width > WIDTH - MARGIN_RIGHT {
        left = anchor_x - OFFSET - width;
    }
    let left = left.max(2.0);
    let top = MARGIN_TOP;

    let heading_x = left + PADDING;
    let heading_y = top + 17.0;
    let value_x = left + width - PADDING;

    let rows: Vec<TooltipRow> = lines
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            let row_y = top + HEADER_HEIGHT + 4.0 + ROW_HEIGHT * index as f64;
            let label_x = if line.swatch.is_some() {
                heading_x + SWATCH_INDENT
            } else {
                heading_x
            };

            TooltipRow {
                y: format!("{row_y:.2}"),
                label_x: format!("{label_x:.2}"),
                swatch_x: format!("{:.2}", heading_x + 4.0),
                swatch_y: format!("{:.2}", row_y - 4.0),
                swatch_class: line.swatch.map(ChartTone::fill_class),
                label: line.label,
                value: line.value,
            }
        })
        .collect();

    rsx! {
        g { class: "pointer-events-none",
            rect {
                x: "{left:.2}",
                y: "{top:.2}",
                width: "{width:.2}",
                height: "{height:.2}",
                rx: "6",
                class: "fill-(--clr-surface-raised) stroke-(color:--clr-border-strong)",
            }
            text {
                x: "{heading_x:.2}",
                y: "{heading_y:.2}",
                class: "fill-(--clr-text-muted) text-(length:--fs-xs)",
                "{heading}"
            }
            for row in rows {
                if let Some(swatch_class) = row.swatch_class {
                    circle {
                        cx: "{row.swatch_x}",
                        cy: "{row.swatch_y}",
                        r: "4",
                        class: "{swatch_class}",
                    }
                }
                text {
                    x: "{row.label_x}",
                    y: "{row.y}",
                    class: "fill-(--clr-text-muted) text-(length:--fs-xs)",
                    "{row.label}"
                }
                text {
                    x: "{value_x:.2}",
                    y: "{row.y}",
                    text_anchor: "end",
                    class: "fill-(--clr-text) text-(length:--fs-xs) tabular-nums",
                    "{row.value}"
                }
            }
        }
    }
}

// Tooltip row with every coordinate already formatted for the SVG attributes
struct TooltipRow {
    y: String,
    label_x: String,
    swatch_x: String,
    swatch_y: String,
    swatch_class: Option<&'static str>,
    label: String,
    value: String,
}
