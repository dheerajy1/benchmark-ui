use std::{cell::RefCell, collections::HashMap};

const KEY_PREFIX: &str = "benchmark.cancel.";

// Where a run's cancel token is kept, by run id. Never a URL, an event or a log line.
pub fn token_key(run_id: &str) -> String {
    format!("{KEY_PREFIX}{run_id}")
}

// What the API issues: 128 random bits as 32 hex digits. Anything else is not a token.
pub fn is_valid_token(token: &str) -> bool {
    token.len() == 32 && token.chars().all(|character| character.is_ascii_hexdigit())
}

// A place that keeps strings by key for the life of a browser tab
pub trait TokenStore {
    fn get(&self, key: &str) -> Option<String>;

    // False when the value could not be kept
    fn set(&self, key: &str, value: &str) -> bool;

    fn remove(&self, key: &str);
}

thread_local! {
    // Kept next to the tab's storage, so cancelling still works inside this page load when
    // the storage is blocked. A reload empties it, the storage is what survives a reload.
    static MEMORY: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

// Keeps the token of a run, in the store and in memory. Answers whether the store kept it.
pub fn save_token(store: &impl TokenStore, run_id: &str, token: &str) -> bool {
    if !is_valid_token(token) {
        return false;
    }

    let key = token_key(run_id);

    MEMORY.with(|memory| {
        memory.borrow_mut().insert(key.clone(), token.to_string());
    });

    store.set(&key, token)
}

// The token of a run: the store first, then memory. A stored value of the wrong shape is ignored.
pub fn load_token(store: &impl TokenStore, run_id: &str) -> Option<String> {
    let key = token_key(run_id);

    store
        .get(&key)
        .filter(|token| is_valid_token(token))
        .or_else(|| MEMORY.with(|memory| memory.borrow().get(&key).cloned()))
        .filter(|token| is_valid_token(token))
}

// Removes the token everywhere, called when a run reaches its end
pub fn forget_token(store: &impl TokenStore, run_id: &str) {
    let key = token_key(run_id);

    MEMORY.with(|memory| {
        memory.borrow_mut().remove(&key);
    });

    store.remove(&key);
}

// The browser's sessionStorage: private to the tab, gone when the tab closes. Every call is
// safe when the storage is missing or blocked, it just keeps nothing.
pub struct SessionTokenStore;

impl SessionTokenStore {
    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.session_storage().ok().flatten()
    }
}

impl TokenStore for SessionTokenStore {
    fn get(&self, key: &str) -> Option<String> {
        Self::storage()?.get_item(key).ok().flatten()
    }

    fn set(&self, key: &str, value: &str) -> bool {
        Self::storage().is_some_and(|storage| storage.set_item(key, value).is_ok())
    }

    fn remove(&self, key: &str) {
        if let Some(storage) = Self::storage() {
            let _ = storage.remove_item(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "3f9a0c1b7d2e48a6b5c4d3e2f1a09b8c";
    const OTHER: &str = "00112233445566778899aabbccddeeff";

    struct FakeStore {
        data: RefCell<HashMap<String, String>>,
        available: bool,
    }

    impl FakeStore {
        fn new(available: bool) -> Self {
            Self {
                data: RefCell::new(HashMap::new()),
                available,
            }
        }
    }

    impl TokenStore for FakeStore {
        fn get(&self, key: &str) -> Option<String> {
            if !self.available {
                return None;
            }

            self.data.borrow().get(key).cloned()
        }

        fn set(&self, key: &str, value: &str) -> bool {
            if !self.available {
                return false;
            }

            self.data
                .borrow_mut()
                .insert(key.to_string(), value.to_string());

            true
        }

        fn remove(&self, key: &str) {
            self.data.borrow_mut().remove(key);
        }
    }

    #[test]
    fn the_key_belongs_to_the_run() {
        assert_eq!(token_key("abc"), "benchmark.cancel.abc");
        assert_ne!(token_key("abc"), token_key("abd"));
    }

    #[test]
    fn only_thirty_two_hex_digits_are_a_token() {
        assert!(is_valid_token(TOKEN));
        assert!(is_valid_token("ABCDEF0123456789abcdef0123456789"));

        for wrong in [
            "",
            "short",
            "3f9a0c1b7d2e48a6b5c4d3e2f1a09b8",
            "3f9a0c1b7d2e48a6b5c4d3e2f1a09b8cc",
            "3f9a0c1b7d2e48a6b5c4d3e2f1a09b8g",
            "3f9a0c1b-7d2e-48a6-b5c4-d3e2f1a09b8c",
        ] {
            assert!(!is_valid_token(wrong), "{wrong}");
        }
    }

    #[test]
    fn a_saved_token_is_found_again() {
        let store = FakeStore::new(true);

        assert!(save_token(&store, "run-saved", TOKEN));
        assert_eq!(load_token(&store, "run-saved"), Some(TOKEN.to_string()));
    }

    #[test]
    fn a_token_survives_a_reload_through_the_store_alone() {
        let store = FakeStore::new(true);

        store.set(&token_key("run-reloaded"), TOKEN);

        // Nothing in memory, as after a reload
        assert_eq!(load_token(&store, "run-reloaded"), Some(TOKEN.to_string()));
    }

    #[test]
    fn something_that_is_not_a_token_is_never_saved() {
        let store = FakeStore::new(true);

        assert!(!save_token(&store, "run-bad", "not a token"));
        assert_eq!(load_token(&store, "run-bad"), None);
        assert!(store.data.borrow().is_empty());
    }

    #[test]
    fn a_stored_value_of_the_wrong_shape_is_ignored() {
        let store = FakeStore::new(true);

        store.set(&token_key("run-junk"), "<script>");

        assert_eq!(load_token(&store, "run-junk"), None);
    }

    #[test]
    fn blocked_storage_falls_back_to_memory_for_this_page_load() {
        let store = FakeStore::new(false);

        assert!(!save_token(&store, "run-blocked", TOKEN));
        assert_eq!(load_token(&store, "run-blocked"), Some(TOKEN.to_string()));
    }

    #[test]
    fn tokens_of_different_runs_are_independent() {
        let store = FakeStore::new(true);

        save_token(&store, "run-one", TOKEN);
        save_token(&store, "run-two", OTHER);

        assert_eq!(load_token(&store, "run-one"), Some(TOKEN.to_string()));
        assert_eq!(load_token(&store, "run-two"), Some(OTHER.to_string()));
        assert_eq!(load_token(&store, "run-three"), None);
    }

    #[test]
    fn forgetting_removes_the_token_from_the_store_and_from_memory() {
        let store = FakeStore::new(true);

        save_token(&store, "run-forgotten", TOKEN);
        forget_token(&store, "run-forgotten");

        assert_eq!(load_token(&store, "run-forgotten"), None);
        assert!(store.data.borrow().is_empty());

        // Also when the store was blocked and only memory had it
        let blocked = FakeStore::new(false);

        save_token(&blocked, "run-forgotten-blocked", TOKEN);
        forget_token(&blocked, "run-forgotten-blocked");

        assert_eq!(load_token(&blocked, "run-forgotten-blocked"), None);
    }
}
