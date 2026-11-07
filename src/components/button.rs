// Visual weight of a button or button-like link
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
}

// 36px high, radius 10, a 0.97 scale on press and a soft focus halo
const BASE: &str = "inline-flex h-(--control-height) cursor-pointer items-center justify-center gap-(--space-2) rounded-(--round-md) px-(--space-4) text-(length:--fs-sm) font-(weight:--fw-medium) whitespace-nowrap transition-[background-color,box-shadow,transform,color,border-color] duration-200 ease-(--ease-out) active:scale-[0.97] disabled:cursor-not-allowed disabled:opacity-50 disabled:active:scale-100 focus-visible:outline-none focus-visible:shadow-(--halo-focus)";

// Class string for `button` and `Link` elements so both look identical
pub fn button_class(variant: ButtonVariant) -> String {
    let variant_class = match variant {
        ButtonVariant::Primary => {
            "bg-(--clr-primary) text-(color:--clr-primary-contrast) hover:bg-(--clr-primary-hover) hover:shadow-(--shadow-primary)"
        }
        ButtonVariant::Secondary => {
            "border border-(color:--clr-border) bg-(--clr-surface) text-(color:--clr-text) hover:bg-(--clr-surface-hover)"
        }
    };

    format!("{BASE} {variant_class}")
}
