// Series colors, each variant maps to design tokens defined in tailwind.css
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChartTone {
    #[default]
    Blue,
    Green,
    Amber,
    Cyan,
    Violet,
}

impl ChartTone {
    pub fn stroke_class(self) -> &'static str {
        match self {
            Self::Blue => "stroke-(color:--clr-chart-1)",
            Self::Green => "stroke-(color:--clr-chart-2)",
            Self::Amber => "stroke-(color:--clr-chart-3)",
            Self::Cyan => "stroke-(color:--clr-chart-4)",
            Self::Violet => "stroke-(color:--clr-chart-5)",
        }
    }

    pub fn fill_class(self) -> &'static str {
        match self {
            Self::Blue => "fill-(--clr-chart-1)",
            Self::Green => "fill-(--clr-chart-2)",
            Self::Amber => "fill-(--clr-chart-3)",
            Self::Cyan => "fill-(--clr-chart-4)",
            Self::Violet => "fill-(--clr-chart-5)",
        }
    }

    pub fn soft_fill_class(self) -> &'static str {
        match self {
            Self::Blue => "fill-(--clr-chart-1-soft)",
            Self::Green => "fill-(--clr-chart-2-soft)",
            Self::Amber => "fill-(--clr-chart-3-soft)",
            Self::Cyan => "fill-(--clr-chart-4-soft)",
            Self::Violet => "fill-(--clr-chart-5-soft)",
        }
    }

    pub fn swatch_class(self) -> &'static str {
        match self {
            Self::Blue => "bg-(--clr-chart-1)",
            Self::Green => "bg-(--clr-chart-2)",
            Self::Amber => "bg-(--clr-chart-3)",
            Self::Cyan => "bg-(--clr-chart-4)",
            Self::Violet => "bg-(--clr-chart-5)",
        }
    }
}
