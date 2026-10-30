// Series colors, each variant maps to design tokens defined in tailwind.css.
// Green and red are left to success and regression states, so no series uses them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChartTone {
    #[default]
    Blue,
    Amber,
    Cyan,
    Violet,
    Teal,
    Slate,
}

impl ChartTone {
    pub fn stroke_class(self) -> &'static str {
        match self {
            Self::Blue => "stroke-(color:--clr-chart-1)",
            Self::Amber => "stroke-(color:--clr-chart-3)",
            Self::Cyan => "stroke-(color:--clr-chart-4)",
            Self::Violet => "stroke-(color:--clr-chart-5)",
            Self::Teal => "stroke-(color:--clr-chart-6)",
            Self::Slate => "stroke-(color:--clr-chart-neutral)",
        }
    }

    pub fn fill_class(self) -> &'static str {
        match self {
            Self::Blue => "fill-(--clr-chart-1)",
            Self::Amber => "fill-(--clr-chart-3)",
            Self::Cyan => "fill-(--clr-chart-4)",
            Self::Violet => "fill-(--clr-chart-5)",
            Self::Teal => "fill-(--clr-chart-6)",
            Self::Slate => "fill-(--clr-chart-neutral)",
        }
    }

    pub fn soft_fill_class(self) -> &'static str {
        match self {
            Self::Blue => "fill-(--clr-chart-1-soft)",
            Self::Amber => "fill-(--clr-chart-3-soft)",
            Self::Cyan => "fill-(--clr-chart-4-soft)",
            Self::Violet => "fill-(--clr-chart-5-soft)",
            Self::Teal => "fill-(--clr-chart-6-soft)",
            Self::Slate => "fill-(--clr-chart-neutral-soft)",
        }
    }

    pub fn swatch_class(self) -> &'static str {
        match self {
            Self::Blue => "bg-(--clr-chart-1)",
            Self::Amber => "bg-(--clr-chart-3)",
            Self::Cyan => "bg-(--clr-chart-4)",
            Self::Violet => "bg-(--clr-chart-5)",
            Self::Teal => "bg-(--clr-chart-6)",
            Self::Slate => "bg-(--clr-chart-neutral)",
        }
    }
}
