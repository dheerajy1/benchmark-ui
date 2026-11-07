use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

// How long a toast stays on screen
const TOAST_MS: u32 = 2_200;

// A failed small action stays longer, it may offer `Try again`
const ERROR_TOAST_MS: u32 = 6_000;

#[derive(Clone, PartialEq)]
struct ToastData {
    id: u64,
    message: String,
    // Set for a failed action: red dot and an optional `Try again`
    failed: bool,
    retry: Option<Callback<()>>,
}

// Handle to show toasts from any component below `ToastProvider`
#[derive(Clone, Copy)]
pub struct Toasts {
    current: Signal<Option<ToastData>>,
    next_id: Signal<u64>,
}

impl Toasts {
    // Short confirmation such as `Copied`
    pub fn show(self, message: impl Into<String>) {
        let mut current = self.current;
        let mut next_id = self.next_id;
        let id = next_id() + 1;

        next_id.set(id);
        current.set(Some(ToastData {
            id,
            message: message.into(),
            failed: false,
            retry: None,
        }));
    }

    // A small action that failed, for example saving a picture. `retry` adds a `Try again` link.
    pub fn show_error(self, message: impl Into<String>, retry: Option<Callback<()>>) {
        let mut current = self.current;
        let mut next_id = self.next_id;
        let id = next_id() + 1;

        next_id.set(id);
        current.set(Some(ToastData {
            id,
            message: message.into(),
            failed: true,
            retry,
        }));
    }
}

// Gives the toast handle to every child and draws the dark toast at the bottom center
#[component]
pub fn ToastProvider(children: Element) -> Element {
    let current = use_signal(|| None::<ToastData>);
    let next_id = use_signal(|| 0u64);
    use_context_provider(|| Toasts { current, next_id });
    let mut current = current;

    // One timer per toast, owned by the provider so it survives the caller unmounting
    use_effect(move || {
        if let Some(toast) = current() {
            let id = toast.id;
            let stays = if toast.failed {
                ERROR_TOAST_MS
            } else {
                TOAST_MS
            };

            spawn(async move {
                TimeoutFuture::new(stays).await;

                // A newer toast has its own timer, only clear our own
                let still_ours = matches!(&*current.peek(), Some(toast) if toast.id == id);
                if still_ours {
                    current.set(None);
                }
            });
        }
    });

    rsx! {
        {children}
        if let Some(toast) = current() {
            div {
                key: "{toast.id}",
                class: "toast-in fixed bottom-(--space-6) left-1/2 z-60 flex -translate-x-1/2 items-center gap-(--space-3) rounded-(--round-md) bg-(--clr-ink) px-(--space-4) py-(--space-3) text-(length:--fs-sm) text-(color:--clr-ink-contrast) shadow-(--shadow-popover)",
                role: "status",
                "aria-live": "polite",
                if toast.failed {
                    span { class: "size-2 shrink-0 rounded-(--round-full) bg-(--clr-danger)" }
                }
                span { "{toast.message}" }
                if let Some(retry) = toast.retry {
                    button {
                        r#type: "button",
                        class: "cursor-pointer font-(weight:--fw-medium) underline underline-offset-2",
                        onclick: move |_| {
                            current.set(None);
                            retry.call(());
                        },
                        "Try again"
                    }
                }
            }
        }
    }
}

// Toast handle, call it inside a component below `ToastProvider`
pub fn use_toasts() -> Toasts {
    use_context::<Toasts>()
}
