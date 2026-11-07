use dioxus::prelude::*;

// Which corner of the trigger the panel is aligned to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopoverAlign {
    Left,
    #[default]
    Right,
}

// Floating panel next to a trigger. Put it inside a `relative` wrapper and render it only while
// open. A click outside or Esc calls `on_close`. `above` opens it upward, `rise` uses the quick
// 120ms fade and 4px rise of small menus instead of the fade and scale of larger panels.
#[component]
pub fn Popover(
    #[props(default)] align: PopoverAlign,
    #[props(default)] above: bool,
    #[props(default)] rise: bool,
    #[props(into, default = "w-72".to_string())] width: String,
    on_close: EventHandler<()>,
    children: Element,
) -> Element {
    let side_class = match (align, rise) {
        (PopoverAlign::Left, false) => "left-0 panel-in panel-in-left",
        (PopoverAlign::Right, false) => "right-0 panel-in panel-in-right",
        (PopoverAlign::Left, true) => "left-0 menu-in",
        (PopoverAlign::Right, true) => "right-0 menu-in",
    };
    let vertical_class = if above {
        "bottom-full mb-(--space-2)"
    } else {
        "top-full mt-(--space-2)"
    };

    rsx! {
        // Invisible layer that catches the outside click
        div {
            class: "fixed inset-0 z-30",
            onclick: move |_| on_close.call(()),
        }
        div {
            class: "absolute z-40 rounded-(--round-lg) border border-(color:--clr-border) bg-(--clr-surface) shadow-(--shadow-popover) focus:outline-none {vertical_class} {side_class} {width}",
            tabindex: "-1",
            onmounted: move |event| async move {
                let _ = event.set_focus(true).await;
            },
            onkeydown: move |event| {
                if event.key() == Key::Escape {
                    on_close.call(());
                }
            },
            {children}
        }
    }
}
