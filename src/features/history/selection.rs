// Run selection for the compare action, at most two runs, the oldest selection is dropped first

pub const MAX_SELECTED: usize = 2;

pub fn toggle(selected: &mut Vec<String>, run_id: String) {
    if let Some(position) = selected.iter().position(|id| *id == run_id) {
        selected.remove(position);
        return;
    }

    if selected.len() >= MAX_SELECTED {
        selected.remove(0);
    }

    selected.push(run_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_and_deselects() {
        let mut selected = Vec::new();

        toggle(&mut selected, "a".to_string());
        toggle(&mut selected, "b".to_string());
        assert_eq!(selected, vec!["a", "b"]);

        toggle(&mut selected, "a".to_string());
        assert_eq!(selected, vec!["b"]);
    }

    #[test]
    fn keeps_only_the_two_newest_selections() {
        let mut selected = Vec::new();

        for id in ["a", "b", "c"] {
            toggle(&mut selected, id.to_string());
        }

        assert_eq!(selected, vec!["b", "c"]);
    }
}
