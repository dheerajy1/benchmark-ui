use dioxus::prelude::*;

// Runtime families that get their own color, anything else is neutral
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeColor {
    Rust,
    Bun,
    Node,
    Python,
    Go,
    Other,
}

fn runtime_color(runtime: &str) -> RuntimeColor {
    match runtime.trim().to_ascii_lowercase().as_str() {
        "rust" => RuntimeColor::Rust,
        "bun" | "typescript" => RuntimeColor::Bun,
        "node" | "node.js" | "nodejs" => RuntimeColor::Node,
        "python" => RuntimeColor::Python,
        "go" => RuntimeColor::Go,
        _ => RuntimeColor::Other,
    }
}

fn runtime_class(color: RuntimeColor) -> &'static str {
    match color {
        RuntimeColor::Rust => "bg-(--clr-runtime-rust-soft) text-(color:--clr-runtime-rust)",
        RuntimeColor::Bun => "bg-(--clr-runtime-blue-soft) text-(color:--clr-runtime-blue)",
        RuntimeColor::Node => "bg-(--clr-runtime-green-soft) text-(color:--clr-runtime-green)",
        RuntimeColor::Python => "bg-(--clr-runtime-violet-soft) text-(color:--clr-runtime-violet)",
        RuntimeColor::Go => "bg-(--clr-runtime-cyan-soft) text-(color:--clr-runtime-cyan)",
        RuntimeColor::Other => "bg-(--clr-surface-hover) text-(color:--clr-text-muted)",
    }
}

// Small colored tag with the runtime name (Rust orange, Bun blue, Node green)
#[component]
pub fn RuntimeTag(#[props(into)] runtime: String) -> Element {
    let class = runtime_class(runtime_color(&runtime));

    rsx! {
        span { class: "inline-flex items-center rounded-(--round-full) px-(--space-2) py-px text-(length:--fs-xs) font-(weight:--fw-medium) whitespace-nowrap {class}",
            "{runtime}"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_runtimes_ignoring_case() {
        assert_eq!(runtime_color("Rust"), RuntimeColor::Rust);
        assert_eq!(runtime_color(" BUN "), RuntimeColor::Bun);
        assert_eq!(runtime_color("Node.js"), RuntimeColor::Node);
    }

    #[test]
    fn unknown_runtime_is_neutral() {
        assert_eq!(runtime_color("Unknown"), RuntimeColor::Other);
        assert_eq!(runtime_color(""), RuntimeColor::Other);
    }
}
