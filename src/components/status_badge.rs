use dioxus::prelude::*;

use super::{Badge, Tone};
use crate::format::humanize_code;

// Tone for the backend's status codes such as success, failed or no_build_command
fn status_tone(status: &str) -> Tone {
    match status {
        "success" | "completed" => Tone::Success,
        "failed" | "timeout" | "unhealthy" | "target_exited" => Tone::Danger,
        _ => Tone::Neutral,
    }
}

// Badge for a measurement status reported by the API
#[component]
pub fn StatusBadge(#[props(into)] status: String) -> Element {
    rsx! {
        Badge { label: humanize_code(&status), tone: status_tone(&status) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_status_codes_to_tones() {
        assert_eq!(status_tone("success"), Tone::Success);
        assert_eq!(status_tone("target_exited"), Tone::Danger);
        assert_eq!(status_tone("no_build_command"), Tone::Neutral);
    }
}
