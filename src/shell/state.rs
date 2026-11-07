use std::rc::Rc;

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

// Local storage key of the pinned state, kept in this browser only
const PIN_KEY: &str = "benchmark.sidebar.pinned";

// How long the peeking sidebar stays after the mouse leaves
const PEEK_HIDE_MS: u32 = 240;

// A first visit starts with the sidebar pinned
fn read_pinned() -> bool {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(PIN_KEY).ok().flatten())
        .is_none_or(|value| value != "0")
}

fn write_pinned(pinned: bool) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item(PIN_KEY, if pinned { "1" } else { "0" });
    }
}

// Shell state shared by the sidebar, the top bar and the keyboard shortcuts
#[derive(Clone, Copy)]
pub struct ShellState {
    pinned: Signal<bool>,
    peek: Signal<bool>,
    hold_open: Signal<bool>,
    leave_ticket: Signal<u64>,
    search_input: Signal<Option<Rc<MountedData>>>,
    recents_refresh: Signal<u64>,
}

// Creates the shell state and gives it to every component below
pub fn use_shell_provider() -> ShellState {
    let state = ShellState {
        pinned: use_signal(read_pinned),
        peek: use_signal(|| false),
        hold_open: use_signal(|| false),
        leave_ticket: use_signal(|| 0),
        search_input: use_signal(|| None),
        recents_refresh: use_signal(|| 0),
    };

    use_context_provider(|| state)
}

pub fn use_shell() -> ShellState {
    use_context::<ShellState>()
}

impl ShellState {
    pub fn is_pinned(self) -> bool {
        (self.pinned)()
    }

    // The floating sidebar is visible when it is pinned or peeking
    pub fn is_peeking(self) -> bool {
        (self.peek)()
    }

    // Counter that changes when the Recents list should load again
    pub fn recents_version(self) -> u64 {
        (self.recents_refresh)()
    }

    pub fn bump_recents(self) {
        let mut version = self.recents_refresh;
        let next = *version.peek() + 1;

        version.set(next);
    }

    pub fn toggle_pin(self) {
        let mut pinned = self.pinned;
        let mut peek = self.peek;
        let next = !*pinned.peek();

        pinned.set(next);
        peek.set(false);
        write_pinned(next);
    }

    // Mouse entered the button or the floating card
    pub fn peek_in(self) {
        let mut peek = self.peek;
        let mut ticket = self.leave_ticket;
        let next = *ticket.peek() + 1;

        ticket.set(next);
        peek.set(true);
    }

    // Mouse left: hide after a short delay unless the mouse came back or a menu holds it open
    pub fn peek_out(self) {
        let mut peek = self.peek;
        let mut ticket = self.leave_ticket;
        let hold_open = self.hold_open;
        let id = *ticket.peek() + 1;

        ticket.set(id);

        spawn(async move {
            TimeoutFuture::new(PEEK_HIDE_MS).await;

            if *ticket.peek() == id && !*hold_open.peek() {
                peek.set(false);
            }
        });
    }

    pub fn close_peek(self) {
        let mut peek = self.peek;

        peek.set(false);
    }

    // A menu inside the sidebar keeps the floating card open while it is open
    pub fn set_hold_open(self, hold: bool) {
        let mut hold_open = self.hold_open;

        if *hold_open.peek() != hold {
            hold_open.set(hold);
        }
    }

    pub fn register_search(self, input: Rc<MountedData>) {
        let mut search_input = self.search_input;

        search_input.set(Some(input));
    }

    pub fn focus_search(self) {
        let input = self.search_input.peek().clone();

        if let Some(input) = input {
            spawn(async move {
                let _ = input.set_focus(true).await;
            });
        }
    }
}
