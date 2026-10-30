// Semantic color intent shared by badges, tiles and deltas
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tone {
    #[default]
    Neutral,
    Accent,
    Success,
    Danger,
}

impl Tone {
    pub fn badge_class(self) -> &'static str {
        match self {
            Self::Neutral => "bg-(--clr-surface-raised) text-(color:--clr-text-muted)",
            Self::Accent => "bg-(--clr-accent-soft) text-(color:--clr-accent)",
            Self::Success => "bg-(--clr-success-soft) text-(color:--clr-success)",
            Self::Danger => "bg-(--clr-danger-soft) text-(color:--clr-danger)",
        }
    }

    pub fn text_class(self) -> &'static str {
        match self {
            Self::Neutral => "text-(color:--clr-text-muted)",
            Self::Accent => "text-(color:--clr-accent)",
            Self::Success => "text-(color:--clr-success)",
            Self::Danger => "text-(color:--clr-danger)",
        }
    }

    pub fn dot_class(self) -> &'static str {
        match self {
            Self::Neutral => "bg-(--clr-text-faint)",
            Self::Accent => "bg-(--clr-accent)",
            Self::Success => "bg-(--clr-success)",
            Self::Danger => "bg-(--clr-danger)",
        }
    }
}
