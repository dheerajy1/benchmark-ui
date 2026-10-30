// Visual weight of a button or button-like link
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
}

const BASE: &str = "inline-flex h-(--control-height) cursor-pointer items-center justify-center gap-(--space-2) rounded-(--round-md) px-(--space-3) text-(length:--fs-sm) font-(weight:--fw-medium) whitespace-nowrap transition-colors disabled:cursor-not-allowed disabled:opacity-50";

// Class string for `button` and `Link` elements so both look identical
pub fn button_class(variant: ButtonVariant) -> String {
    let variant_class = match variant {
        ButtonVariant::Primary => {
            "bg-(--clr-primary) text-(color:--clr-primary-contrast) hover:bg-(--clr-primary-hover)"
        }
        ButtonVariant::Secondary => {
            "border border-(color:--clr-border-strong) bg-(--clr-surface) text-(color:--clr-text) hover:bg-(--clr-surface-hover)"
        }
        ButtonVariant::Ghost => {
            "text-(color:--clr-text-muted) hover:bg-(--clr-surface-hover) hover:text-(color:--clr-text)"
        }
    };

    format!("{BASE} {variant_class}")
}
