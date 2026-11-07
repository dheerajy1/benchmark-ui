use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use super::{
    driver::{follow, FeedState},
    header::{HeaderCard, LiveHud, TabTitle},
    last_run::fetch_last_run,
    outcome::Outcome,
    phase::Charts,
    pipeline::{Pipeline, UpNext},
    state::{log_text, LiveState, Terminal},
    token::{forget_token, load_token, SessionTokenStore},
};
use crate::{
    app::Route,
    components::{
        button_class, copy_text, use_toasts, ButtonVariant, Card, CardHeader, ErrorKind,
        ErrorState, Icon, IconKind, LoadingPanel, PageHeader,
    },
    features::{
        run_detail::WorkspacesSection,
        submission::{fetch_stage_averages, live_total_ms},
    },
    format::{format_duration_ms, short_id},
};

const BANNER_WARNING: &str =
    "border-(color:--clr-warning) bg-(--clr-warning-soft) text-(color:--clr-warning)";
const BANNER_INFO: &str = "border-(color:--clr-info) bg-(--clr-info-soft) text-(color:--clr-info)";
const BANNER_DANGER: &str =
    "border-(color:--clr-danger) bg-(--clr-danger-soft) text-(color:--clr-danger)";

// Live view of one run, the page of /runs/live/:run_id
#[component]
pub fn LiveRun(run_id: String) -> Element {
    // A different run id remounts the page, so no state leaks from one run into another
    rsx! {
        LivePage { key: "{run_id}", run_id: run_id.clone() }
    }
}

#[component]
fn LivePage(run_id: String) -> Element {
    let initial_id = run_id.clone();
    let state = use_signal(move || LiveState::new(initial_id.clone()));
    let feed = use_signal(|| FeedState::Connecting);
    let mut clock = use_signal(|| 0u64);
    let restart = use_signal(|| 0u32);

    // The right to cancel this run, kept by the tab that started it
    let token_run_id = run_id.clone();
    let token = use_signal(move || load_token(&SessionTokenStore, &token_run_id));

    // Once the run has ended its token is of no use, so it is removed
    let forget_run_id = run_id.clone();

    use_effect(move || {
        if state.read().is_terminal() {
            forget_token(&SessionTokenStore, &forget_run_id);
        }
    });

    // Local time in milliseconds, it only fills the time between two events
    use_future(move || async move {
        loop {
            TimeoutFuture::new(1_000).await;

            clock += 1_000;
        }
    });

    // Follows the run until it ends. Changing `restart` starts it again from the last event.
    let follow_id = run_id.clone();
    use_future(move || {
        let _restart = restart();
        let run_id = follow_id.clone();

        async move { follow(run_id, state, feed, clock).await }
    });

    // The usual time of this project's runs, for the time that is left and the tab title
    let plan_path = use_memo(move || state.read().plan().map(|plan| plan.path.clone()));
    let averages = use_resource(move || {
        let path = plan_path();

        async move {
            match path {
                Some(path) => fetch_stage_averages(&path).await.ok(),
                None => None,
            }
        }
    });
    let estimate_ms = use_memo(move || {
        let known = averages().flatten()?;
        let snapshot = state.read();
        let plan = snapshot.plan()?;

        live_total_ms(plan.workload.is_some(), plan.workspaces > 0, &known)
    });

    // The newest finished run of the project, for the dashed lines of the charts
    let last_run = use_resource(move || {
        let path = plan_path();

        async move {
            match path {
                Some(path) => fetch_last_run(&path).await,
                None => None,
            }
        }
    });

    // The charts stop at this snapshot while the view is paused
    let mut frozen = use_signal(|| None::<LiveState>);

    use_effect(move || {
        if state.read().is_terminal() && frozen.peek().is_some() {
            frozen.set(None);
        }
    });

    let short = short_id(&run_id);
    let secondary = button_class(ButtonVariant::Secondary);

    let waiting = {
        let snapshot = state.read();

        !snapshot.has_events() && !snapshot.is_terminal()
    };
    let lost = matches!(feed(), FeedState::Lost);

    let body: Element = if lost {
        rsx! {
            ErrorState {
                title: "This run is no longer available",
                message: "The API no longer has this run live and it was not stored. It may have failed, or the API may have restarted while it ran.",
                kind: ErrorKind::Grey,
                retryable: false,
                actions: Some(rsx! {
                    Link { to: Route::new_benchmark(), class: "{secondary}", "New benchmark" }
                    Link { to: Route::runs(), class: "{secondary}", "All runs" }
                }),
                on_retry: move |_| {},
            }
        }
    } else if waiting && matches!(feed(), FeedState::Connecting) {
        rsx! {
            LoadingPanel { label: "Connecting to the live run" }
        }
    } else {
        rsx! {
            div { class: "flex flex-col gap-(--space-4)",
                HeaderCard {
                    state,
                    feed,
                    clock,
                    token,
                    estimate_ms: estimate_ms(),
                    paused: frozen.read().is_some(),
                    on_pause: move |_| {
                        if frozen.peek().is_some() {
                            frozen.set(None);
                        } else {
                            frozen.set(Some(state.peek().clone()));
                        }
                    },
                }
                FeedBanner { feed, restart }
                if !state.read().is_terminal() && !state.read().is_recovered() {
                    LiveHud { state }
                }
                Outcome {
                    state,
                    last_run_id: last_run().flatten().map(|last| last.run_id),
                }
                if !state.read().is_recovered() {
                    div { class: "grid grid-cols-1 gap-(--space-4) lg:grid-cols-[minmax(0,24rem)_minmax(0,1fr)] lg:items-start",
                        div { class: "flex min-w-0 flex-col gap-(--space-4)",
                            Pipeline {
                                state,
                                clock,
                                averages: averages().flatten(),
                            }
                            UpNext { state, averages: averages().flatten() }
                        }
                        Charts {
                            state,
                            frozen,
                            last_run: last_run().flatten(),
                            averages: averages().flatten(),
                            clock,
                        }
                    }
                }
                WorkspaceResults { state }
                if !state.read().is_recovered() {
                    EventLog { state }
                }
            }
        }
    };

    rsx! {
        TabTitle {
            state,
            clock,
            estimate_ms: estimate_ms(),
            short: short.clone(),
        }
        PageHeader {
            title: "Live run {short}",
            description: "A benchmark that is running now. Its steps and measurements update as they happen, and the result is stored when it finishes.",
            Link { to: Route::runs(), class: "{secondary}", "All runs" }
        }
        {body}
    }
}

// How the page is getting its events, silent while the live stream works
#[component]
fn FeedBanner(feed: Signal<FeedState>, restart: Signal<u32>) -> Element {
    let mut restart = restart;
    let secondary = button_class(ButtonVariant::Secondary);

    let (class, text, can_retry) = match feed() {
        FeedState::Reconnecting { attempt, reason } => (
            BANNER_WARNING,
            format!("Lost the connection to the API ({reason}). The run keeps going on the server, reconnecting, attempt {attempt}. Nothing is missed: the stream resumes after the last event."),
            false,
        ),
        FeedState::Polling => (
            BANNER_INFO,
            "A live connection is not possible from here, so this page checks for news every 2 seconds instead.".to_string(),
            false,
        ),
        FeedState::TooManyViewers => (
            BANNER_WARNING,
            "This run already has the most viewers it allows. Trying again shortly.".to_string(),
            false,
        ),
        FeedState::Offline(detail) => (
            BANNER_DANGER,
            format!("The API cannot be reached. {detail}"),
            true,
        ),
        FeedState::Connecting | FeedState::Live | FeedState::Finished | FeedState::Lost => {
            return rsx! {};
        }
    };

    rsx! {
        div {
            class: "flex flex-wrap items-center justify-between gap-(--space-3) rounded-(--round-lg) border px-(--space-4) py-(--space-3) text-(length:--fs-sm) {class}",
            role: "status",
            span { class: "min-w-0 break-words", "{text}" }
            if can_retry {
                button {
                    r#type: "button",
                    class: "{secondary}",
                    onclick: move |_| restart += 1,
                    "Try again"
                }
            }
        }
    }
}

// The final numbers of every workspace mode, once the run is stored
#[component]
fn WorkspaceResults(state: Signal<LiveState>) -> Element {
    let workspaces = match state.read().terminal() {
        Some(Terminal::Completed(Some(result))) => result.metrics.workspaces.clone(),
        _ => Vec::new(),
    };

    rsx! {
        WorkspacesSection { workspaces }
    }
}

// What happened, oldest first. The list stays pinned to its bottom, so the newest line is always
// in view without scrolling.
#[component]
fn EventLog(state: Signal<LiveState>) -> Element {
    let toasts = use_toasts();
    let (lines, copy) = {
        let snapshot = state.read();

        // Newest first in the page, the reversed column shows it last
        let lines: Vec<(String, String, &'static str)> = snapshot
            .log()
            .iter()
            .rev()
            .map(|line| {
                (
                    format!("+{}", format_duration_ms(line.t_ms as f64)),
                    line.text.clone(),
                    line.tone.text_class(),
                )
            })
            .collect();

        (lines, log_text(snapshot.log()))
    };

    rsx! {
        Card {
            CardHeader {
                title: "Events",
                subtitle: "Oldest first, times are since the run was accepted.",
                trailing: Some(rsx! {
                    button {
                        r#type: "button",
                        class: "inline-flex cursor-pointer items-center gap-(--space-2) text-(length:--fs-sm) text-(color:--clr-text-muted) transition-colors hover:text-(color:--clr-accent)",
                        onclick: move |_| {
                            copy_text(&copy);
                            toasts.show("Copied the event log");
                        },
                        Icon { kind: IconKind::Copy, class: "size-(--icon-sm)" }
                        "Copy log"
                    }
                }),
            }
            ol { class: "flex max-h-72 flex-col-reverse gap-(--space-1) overflow-y-auto px-(--space-4) py-(--space-3)",
                for (index , (time , text , tone_class)) in lines.into_iter().enumerate() {
                    li {
                        key: "{index}",
                        class: "flex items-baseline gap-(--space-3) text-(length:--fs-sm)",
                        span { class: "w-20 shrink-0 font-(family-name:--font-code) text-(length:--fs-xs) tabular-nums text-(color:--clr-text-faint)",
                            "{time}"
                        }
                        span { class: "min-w-0 break-words {tone_class}", "{text}" }
                    }
                }
            }
        }
    }
}
