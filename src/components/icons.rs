use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    Gauge,
    Dashboard,
    History,
    Compare,
    Play,
    Menu,
    Alert,
    Refresh,
    Swap,
}

// Stroke paths on a 24 by 24 grid
fn paths(kind: IconKind) -> &'static [&'static str] {
    match kind {
        IconKind::Gauge => &["M12 14l4-4", "M3.34 19a10 10 0 1 1 17.32 0"],
        IconKind::Dashboard => &[
            "M4 3h5a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z",
            "M15 3h5a1 1 0 0 1 1 1v3a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z",
            "M15 12h5a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1v-7a1 1 0 0 1 1-1z",
            "M4 16h5a1 1 0 0 1 1 1v3a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1v-3a1 1 0 0 1 1-1z",
        ],
        IconKind::History => &[
            "M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8",
            "M3 3v5h5",
            "M12 7v5l4 2",
        ],
        IconKind::Compare => &[
            "M18 15a3 3 0 1 0 0 6 3 3 0 1 0 0-6z",
            "M6 3a3 3 0 1 0 0 6 3 3 0 1 0 0-6z",
            "M13 6h3a2 2 0 0 1 2 2v7",
            "M11 18H8a2 2 0 0 1-2-2V9",
        ],
        IconKind::Play => &["M6 3l14 9-14 9V3z"],
        IconKind::Menu => &["M4 6h16", "M4 12h16", "M4 18h16"],
        IconKind::Alert => &[
            "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z",
            "M12 8v4",
            "M12 16h.01",
        ],
        IconKind::Refresh => &["M21 12a9 9 0 1 1-3-6.7", "M21 4v5h-5"],
        IconKind::Swap => &["M8 3L4 7l4 4", "M4 7h16", "M16 21l4-4-4-4", "M20 17H4"],
    }
}

// Decorative inline icon, always hidden from assistive technology
#[component]
pub fn Icon(
    kind: IconKind,
    #[props(into, default = "size-(--icon-sm)".to_string())] class: String,
) -> Element {
    rsx! {
        svg {
            class: "shrink-0 {class}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            for d in paths(kind) {
                path { d: *d }
            }
        }
    }
}
