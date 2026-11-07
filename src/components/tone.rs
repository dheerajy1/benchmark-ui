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
    // Soft background with readable text, used by pills and badges
    pub fn badge_class(self) -> &'static str {
        match self {
            Self::Neutral => "bg-(--clr-surface-hover) text-(color:--clr-text-muted)",
            Self::Accent => "bg-(--clr-accent-soft) text-(color:--clr-accent)",
            Self::Success => "bg-(--clr-success-pill) text-(color:--clr-success-text)",
            Self::Danger => "bg-(--clr-danger-soft) text-(color:--clr-danger-text)",
        }
    }

    pub fn text_class(self) -> &'static str {
        match self {
            Self::Neutral => "text-(color:--clr-text-muted)",
            Self::Accent => "text-(color:--clr-accent)",
            Self::Success => "text-(color:--clr-success-text)",
            Self::Danger => "text-(color:--clr-danger-text)",
        }
    }
}
