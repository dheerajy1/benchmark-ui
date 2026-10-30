use dioxus::prelude::*;

use super::{
    palette::ChartTone,
    scale::{Domain, LinearScale},
};
use crate::format::ValueFormat;

// Charts are drawn in two layers. The marks (lines, areas, bars) are SVG in a 0 to 100 box
// that stretches over the plot area, so they fill any width. Every piece of text (axis labels,
// tooltips) is plain HTML laid over the SVG, so it keeps one pixel size at any chart width.

// Vertical scale from a value domain to a percentage of the plot height, 0 is the top edge
pub fn y_scale(domain: &Domain) -> LinearScale {
    LinearScale::new(domain.min, domain.max, 100.0, 0.0)
}

// Where a label sits under its position on the x axis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XAlign {
    Start,
    Center,
    End,
}

// Axes, dashed grid lines and the plot area that `children` is drawn into.
// The plot has half a label of space above and below, so tick labels line up with their lines.
#[component]
pub fn PlotFrame(
    domain: Domain,
    format: ValueFormat,
    x_labels: Vec<(String, XAlign)>,
    children: Element,
) -> Element {
    let tick_labels: Vec<String> = domain
        .ticks
        .iter()
        .map(|tick| format.format_axis(*tick))
        .collect();
    let line_count = tick_labels.len();

    let x_items: Vec<(String, &'static str)> = x_labels
        .into_iter()
        .map(|(text, align)| {
            let justify = match align {
                XAlign::Start => "justify-start",
                XAlign::Center => "justify-center",
                XAlign::End => "justify-end",
            };

            (text, justify)
        })
        .collect();

    rsx! {
        div { class: "flex w-full flex-col",
            div { class: "flex h-(--chart-height)",
                div { class: "flex w-14 shrink-0 flex-col-reverse justify-between",
                    for (index , label) in tick_labels.into_iter().enumerate() {
                        span {
                            key: "{index}",
                            class: "h-4 pr-(--space-2) text-right text-(length:--fs-2xs) leading-4 whitespace-nowrap tabular-nums text-(color:--clr-text-faint)",
                            "{label}"
                        }
                    }
                }
                div { class: "relative min-w-0 flex-1",
                    div { class: "absolute inset-x-0 top-2 bottom-2 flex flex-col-reverse justify-between",
                        for index in 0..line_count {
                            div {
                                key: "{index}",
                                class: "h-0 w-full border-t border-dashed border-(color:--clr-border)",
                            }
                        }
                    }
                    div { class: "absolute inset-x-0 top-2 bottom-2", {children} }
                }
            }
            div { class: "flex pt-(--space-2) pl-14",
                for (index , (text , justify)) in x_items.into_iter().enumerate() {
                    div { key: "{index}", class: "flex min-w-0 flex-1 {justify}",
                        span { class: "text-(length:--fs-2xs) whitespace-nowrap text-(color:--clr-text-faint)",
                            "{text}"
                        }
                    }
                }
            }
        }
    }
}

// Transparent columns over the plot that drive the hover tooltip and keyboard focus.
// `labels` holds one accessible description per column.
#[component]
pub fn HoverZones(labels: Vec<String>, active: Signal<Option<usize>>) -> Element {
    let mut active = active;
    let current = active();

    let zones: Vec<(usize, String, &'static str)> = labels
        .into_iter()
        .enumerate()
        .map(|(index, label)| {
            let zone_class = if current == Some(index) {
                "h-full min-w-0 flex-1 bg-(--clr-hover-wash)"
            } else {
                "h-full min-w-0 flex-1"
            };

            (index, label, zone_class)
        })
        .collect();

    rsx! {
        div { class: "absolute inset-0 flex",
            for (index , label , zone_class) in zones {
                div {
                    key: "{index}",
                    class: "{zone_class}",
                    "tabindex": "0",
                    "role": "img",
                    "aria-label": "{label}",
                    onmouseenter: move |_| active.set(Some(index)),
                    onmouseleave: move |_| active.set(None),
                    onfocus: move |_| active.set(Some(index)),
                    onblur: move |_| active.set(None),
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
        ul { class: "flex flex-wrap items-center gap-(--space-4) pb-(--space-3)",
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

// Tooltip pinned to the top of the plot at `anchor_percent` of its width, it flips to the
// left of the anchor on the right half so it never leaves the chart
#[component]
pub fn ChartTooltip(anchor_percent: f64, heading: String, lines: Vec<TooltipLine>) -> Element {
    let left = format!("{anchor_percent:.2}%");
    let placement = if anchor_percent > 55.0 {
        "-translate-x-full -ml-2"
    } else {
        "ml-2"
    };

    let rows: Vec<(String, String, Option<&'static str>)> = lines
        .into_iter()
        .map(|line| {
            (
                line.label,
                line.value,
                line.swatch.map(ChartTone::swatch_class),
            )
        })
        .collect();

    rsx! {
        div {
            class: "pointer-events-none absolute top-0 z-10 min-w-32 rounded-(--round-md) border border-(color:--clr-border-strong) bg-(--clr-surface-raised) px-(--space-3) py-(--space-2) shadow-(--shadow-popover) {placement}",
            style: "left: {left}",
            p { class: "pb-(--space-1) text-(length:--fs-2xs) whitespace-nowrap text-(color:--clr-text-muted)",
                "{heading}"
            }
            for (label , value , swatch) in rows {
                div { class: "flex items-center justify-between gap-(--space-4) text-(length:--fs-xs) whitespace-nowrap",
                    span { class: "flex items-center gap-(--space-2) text-(color:--clr-text-muted)",
                        if let Some(swatch_class) = swatch {
                            span { class: "inline-block size-(--dot-size) rounded-(--round-full) {swatch_class}" }
                        }
                        "{label}"
                    }
                    span { class: "font-(weight:--fw-medium) tabular-nums text-(color:--clr-text)",
                        "{value}"
                    }
                }
            }
        }
    }
}
