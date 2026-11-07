use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    Gauge,
    Dashboard,
    History,
    Compare,
    Play,
    Alert,
    Refresh,
    Swap,
    Panel,
    Search,
    Plus,
    ChevronRight,
    ArrowLeft,
    Check,
    Settings,
    Folder,
    Filter,
    Copy,
    Plug,
    Columns,
    Grip,
    Close,
    Dots,
    Lock,
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
        IconKind::Alert => &[
            "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z",
            "M12 8v4",
            "M12 16h.01",
        ],
        IconKind::Plug => &[
            "M12 22v-5",
            "M9 8V2",
            "M15 8V2",
            "M18 8v5a4 4 0 0 1-4 4h-4a4 4 0 0 1-4-4V8z",
        ],
        IconKind::Columns => &[
            "M4 4h16a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1z",
            "M9 4v16",
            "M15 4v16",
        ],
        IconKind::Grip => &[
            "M9 6h.01",
            "M9 12h.01",
            "M9 18h.01",
            "M15 6h.01",
            "M15 12h.01",
            "M15 18h.01",
        ],
        IconKind::Lock => &[
            "M5 11h14a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1z",
            "M8 11V7a4 4 0 0 1 8 0v4",
        ],
        IconKind::Dots => &["M5 12h.01", "M12 12h.01", "M19 12h.01"],
        IconKind::Close => &["M6 6l12 12", "M18 6L6 18"],
        IconKind::Refresh => &["M21 12a9 9 0 1 1-3-6.7", "M21 4v5h-5"],
        IconKind::Swap => &["M8 3L4 7l4 4", "M4 7h16", "M16 21l4-4-4-4", "M20 17H4"],
        IconKind::Panel => &[
            "M6 4h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z",
            "M9 4v16",
        ],
        IconKind::Search => &["M11 3a8 8 0 1 0 0 16 8 8 0 0 0 0-16z", "M21 21l-4.3-4.3"],
        IconKind::Plus => &["M5 12h14", "M12 5v14"],
        IconKind::ChevronRight => &["M9 6l6 6-6 6"],
        IconKind::ArrowLeft => &["M19 12H5", "M12 19l-7-7 7-7"],
        IconKind::Check => &["M20 6L9 17l-5-5"],
        IconKind::Settings => &[
            "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z",
            "M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6z",
        ],
        IconKind::Folder => &[
            "M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2z",
        ],
        IconKind::Filter => &["M3 6h18", "M7 12h10", "M10 18h4"],
        IconKind::Copy => &[
            "M10 8h10a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H10a2 2 0 0 1-2-2V10a2 2 0 0 1 2-2z",
            "M4 16a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2",
        ],
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
