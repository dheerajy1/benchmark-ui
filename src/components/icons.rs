use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
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
        IconKind::Dashboard => &[
            "M3 3h7v9H3z",
            "M14 3h7v5h-7z",
            "M14 12h7v9h-7z",
            "M3 16h7v5H3z",
        ],
        IconKind::History => &["M3 12a9 9 0 1 0 3-6.7", "M3 4v5h5", "M12 7v5l3 2"],
        IconKind::Compare => &["M7 7h13", "M16 3l4 4-4 4", "M17 17H4", "M8 13l-4 4 4 4"],
        IconKind::Play => &["M6 4l14 8-14 8z"],
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
    #[props(into, default = "size-(--icon-md)".to_string())] class: String,
) -> Element {
    rsx! {
        svg {
            class: "shrink-0 {class}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            for d in paths(kind) {
                path { d: *d }
            }
        }
    }
}
