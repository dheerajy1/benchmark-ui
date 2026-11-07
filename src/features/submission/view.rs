use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use super::{
    api::{submit_live, submit_target_live},
    checks::{all_ok, custom_checks, custom_lines, target_checks, target_lines},
    custom_form::CustomForm,
    estimate::{custom_plan, fetch_stage_averages, panel_parts, target_plan, StageAverages},
    form::{validate, FieldError, FormState},
    paste_modal::PasteModal,
    prefill::{prefill_for, Prefill},
    previous_runs::{cards, format_when, now_epoch_ms, PreviousRuns},
    run_panel::RunPanel,
    skeleton::TargetSkeleton,
    target_card::SavedTargetForm,
    target_run::{validate_target, TargetForm},
};
use crate::{
    app::Route,
    components::{
        button_class, ButtonVariant, ErrorKind, ErrorState, Icon, IconKind, PageHeader,
        SegmentSwitch,
    },
    features::{
        configurations::{
            fetch_configurations, fetch_entry, group, ConfigurationEntry, ConfigurationGroup,
        },
        connection::{ApiError, ApiErrorPanel},
        history::fetch_recent_runs,
        live::{active_run_id, save_token, SessionTokenStore},
        targets::{fetch_targets, TargetView, TargetsView},
    },
};

// Id of the form that the Run benchmark button in the side panel submits
const FORM_ID: &str = "new-benchmark-form";

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Editing,
    // The API is taking the request, it answers with the run id at once
    Running,
    Failed(ApiError),
}

// Data that is loaded from the API when the page opens
#[derive(Debug, Clone, PartialEq)]
enum Loaded<T> {
    Loading,
    Ready(T),
    Failed(ApiError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartMode {
    Target,
    Custom,
}

// Shown after a history row fills the form. It never repeats a header value, only names.
#[derive(Debug, Clone, PartialEq)]
struct Banner {
    text: String,
    notice: Option<String>,
    header_names: Vec<String>,
}

// The target with this id, while it is still in the targets file
fn target_by_id<'a>(view: &'a TargetsView, id: &str) -> Option<&'a TargetView> {
    view.targets.iter().find(|target| target.id == id)
}

// Saved target mode when the server has targets, otherwise custom mode
fn default_mode(view: &TargetsView) -> StartMode {
    if view.targets.is_empty() {
        StartMode::Custom
    } else {
        StartMode::Target
    }
}

// The signals a filled form writes to, copied into the effect and into the lookup of a run
#[derive(Clone, Copy)]
struct FillSignals {
    form: Signal<FormState>,
    reference: Signal<Option<FormState>>,
    errors: Signal<Vec<FieldError>>,
    target_form: Signal<Option<TargetForm>>,
    target_error: Signal<Option<String>>,
    start_mode: Signal<StartMode>,
    banner: Signal<Option<Banner>>,
}

// Fills the form from a run's settings and shows the banner. Nothing starts.
fn fill_from_entry(mut signals: FillSignals, entry: ConfigurationEntry, view: &TargetsView) {
    let short = crate::format::short_id(&entry.run_id);

    let text = if entry.created_at.is_empty() {
        format!("Filled from run {short}. Review and edit before starting.")
    } else {
        format!(
            "Filled from run {short} ({}). Review and edit before starting.",
            format_when(&entry.created_at)
        )
    };

    match prefill_for(&entry, view) {
        Prefill::Target { form: filled } => {
            signals.target_form.set(Some(filled));
            signals.target_error.set(None);
            signals.start_mode.set(StartMode::Target);
            signals.banner.set(Some(Banner {
                text,
                notice: None,
                header_names: Vec::new(),
            }));
        }
        Prefill::Custom {
            form: filled,
            notice,
            header_names,
        } => {
            signals.reference.set(Some((*filled).clone()));
            signals.form.set(*filled);
            signals.errors.set(Vec::new());
            signals.start_mode.set(StartMode::Custom);
            signals.banner.set(Some(Banner {
                text,
                notice,
                header_names,
            }));
        }
        Prefill::Unavailable { reason } => {
            signals.banner.set(Some(Banner {
                text: reason,
                notice: None,
                header_names: Vec::new(),
            }));
        }
    }
}

// The New Benchmark page opened from a run's "Use these settings" button
#[component]
pub fn SubmissionFrom(run_id: String) -> Element {
    rsx! {
        Submission { from_run: Some(run_id), target: String::new() }
    }
}

// Form that starts a live run, from a saved target or from a custom form, and then opens its
// live view
#[component]
pub fn Submission(#[props(default)] from_run: Option<String>, target: String) -> Element {
    let mut form = use_signal(FormState::default);
    let mut reference = use_signal(|| None::<FormState>);
    let mut errors = use_signal(Vec::<FieldError>::new);
    let mut phase = use_signal(|| Phase::Editing);
    let mut targets = use_signal(|| Loaded::<TargetsView>::Loading);
    let mut history = use_signal(|| Loaded::<Vec<ConfigurationGroup>>::Loading);
    let mut all_runs = use_signal(Vec::<ConfigurationEntry>::new);
    let mut start_mode = use_signal(|| StartMode::Custom);
    let mut target_form = use_signal(|| None::<TargetForm>);
    let mut target_error = use_signal(|| None::<String>);
    let mut banner = use_signal(|| None::<Banner>);
    let mut paste_open = use_signal(|| false);
    let mut flash = use_signal(|| false);
    let mut reload = use_signal(|| 0u32);
    let nav = use_navigator();

    // The runtime of each target path, taken from the newest runs, for the tags of the picker
    let recent = use_resource(|| async { fetch_recent_runs().await });

    // The project path of the run being set up, for the time estimate
    let wanted_path = use_memo(move || match (start_mode(), targets()) {
        (StartMode::Target, Loaded::Ready(view)) => target_form()
            .and_then(|filled| target_by_id(&view, &filled.target_id).map(|t| t.path.clone()))
            .unwrap_or_default(),
        _ => form().path.trim().to_string(),
    });

    // Usual stage times of that project. The request waits until typing pauses.
    let estimate = use_resource(move || {
        let path = wanted_path();

        async move {
            if path.is_empty() {
                return Ok(StageAverages::default());
            }

            TimeoutFuture::new(400).await;
            fetch_stage_averages(&path).await
        }
    });

    // Loads the saved targets and the run history when the page opens, and again after a retry.
    // `target` (from /new?target=id) picks the saved target to start with.
    let wanted_target = target.clone();
    use_effect(move || {
        let wanted_target = wanted_target.clone();
        let _ = reload();

        spawn(async move {
            match fetch_targets().await {
                Ok(view) => {
                    if view.targets.is_empty() {
                        start_mode.set(StartMode::Custom);
                    } else {
                        start_mode.set(StartMode::Target);

                        if target_form.peek().is_none() {
                            let first =
                                target_by_id(&view, &wanted_target).or(view.targets.first());
                            target_form.set(first.map(TargetForm::for_target));
                        }
                    }

                    targets.set(Loaded::Ready(view));
                }
                Err(error) => targets.set(Loaded::Failed(error)),
            }
        });

        spawn(async move {
            match fetch_configurations().await {
                Ok(runs) => {
                    history.set(Loaded::Ready(group(&runs)));
                    all_runs.set(runs);
                }
                Err(error) => history.set(Loaded::Failed(error)),
            }
        });
    });

    // Fills the form from the run that was opened with "Use these settings", once the targets and
    // the runs have loaded. It runs only once. A run outside the 200 most recent is looked up.
    let mut from_handled = use_signal(|| false);
    let from_id = from_run.clone();
    use_effect(move || {
        if from_handled() {
            return;
        }

        let Some(run_id) = from_id.clone() else {
            return;
        };

        let Loaded::Ready(view) = targets() else {
            return;
        };

        if !matches!(history(), Loaded::Ready(_)) {
            return;
        }

        from_handled.set(true);

        let signals = FillSignals {
            form,
            reference,
            errors,
            target_form,
            target_error,
            start_mode,
            banner,
        };

        match all_runs().into_iter().find(|entry| entry.run_id == run_id) {
            Some(entry) => fill_from_entry(signals, entry, &view),
            None => {
                spawn(async move {
                    match fetch_entry(&run_id).await {
                        Ok(entry) => fill_from_entry(signals, entry, &view),
                        Err(_) => {
                            let mut signals = signals;

                            signals.banner.set(Some(Banner {
                                text: format!(
                                    "Run {} could not be loaded, so its settings cannot be filled in.",
                                    crate::format::short_id(&run_id)
                                ),
                                notice: None,
                                header_names: Vec::new(),
                            }));
                        }
                    }
                });
            }
        }
    });

    let state = form();
    let current_errors = errors();
    let targets_state = targets();
    let history_state = history();
    let current_target = target_form();
    let target_message = target_error();
    let mode = start_mode();
    let primary = button_class(ButtonVariant::Primary);
    let secondary = button_class(ButtonVariant::Secondary);
    let running = matches!(phase(), Phase::Running);

    // Checks run live. Field errors show once something has been typed, so an empty form is
    // not covered in red.
    let live = validate(&state);
    let live_errors = live.as_ref().err().cloned().unwrap_or_default();
    let touched = state != FormState::default();
    let current_errors = if touched {
        live_errors.clone()
    } else {
        current_errors
    };
    let custom_check_list = custom_checks(&state, &live_errors);
    let custom_ready = live.is_ok() && all_ok(&custom_check_list);
    let target_check_list = target_checks(current_target.as_ref());
    let target_ready = all_ok(&target_check_list);
    let runtimes: Vec<(String, String)> = match recent() {
        Some(Ok(page)) => page
            .items
            .iter()
            .filter_map(|run| {
                run.runtime_code()
                    .map(|code| (run.target.path.clone(), code.to_string()))
            })
            .collect(),
        _ => Vec::new(),
    };

    // Custom runs are offered only when the server allows them
    let custom_allowed = matches!(&targets_state, Loaded::Ready(view) if view.custom_runs_allowed);

    // Custom mode is never shown on a server that refuses custom runs
    let effective_mode = match (mode, &targets_state) {
        (StartMode::Custom, Loaded::Ready(view)) if !view.custom_runs_allowed => StartMode::Target,
        (mode, _) => mode,
    };

    let mut submit_custom = move |_| {
        let request = match validate(&form.peek()) {
            Ok(request) => request,
            Err(found) => {
                errors.set(found);
                return;
            }
        };

        errors.set(Vec::new());
        phase.set(Phase::Running);

        spawn(async move {
            match submit_live(&request).await {
                Ok(accepted) => {
                    // Kept for this tab, so the live page can offer to cancel the run
                    let _ =
                        save_token(&SessionTokenStore, &accepted.run_id, &accepted.cancel_token);

                    let _ = nav.push(Route::LiveRun {
                        run_id: accepted.run_id,
                    });
                }
                Err(error) => phase.set(Phase::Failed(error)),
            }
        });
    };

    let mut submit_target = move |_| {
        let request = match target_form.peek().as_ref().map(validate_target) {
            Some(Ok(request)) => request,
            Some(Err(message)) => {
                target_error.set(Some(message));
                return;
            }
            None => {
                target_error.set(Some("Choose a saved target.".to_string()));
                return;
            }
        };

        target_error.set(None);
        phase.set(Phase::Running);

        spawn(async move {
            match submit_target_live(&request).await {
                Ok(accepted) => {
                    let _ =
                        save_token(&SessionTokenStore, &accepted.run_id, &accepted.cancel_token);

                    let _ = nav.push(Route::LiveRun {
                        run_id: accepted.run_id,
                    });
                }
                Err(error) => phase.set(Phase::Failed(error)),
            }
        });
    };

    // Fills the form from a history row. Nothing starts, the user reviews and presses Run.
    let mut apply_history = move |index: usize| {
        let group = match &*history.peek() {
            Loaded::Ready(groups) => groups.get(index).cloned(),
            _ => None,
        };

        let view = match &*targets.peek() {
            Loaded::Ready(view) => Some(view.clone()),
            _ => None,
        };

        let (Some(group), Some(view)) = (group, view) else {
            return;
        };

        let short_id: String = group.latest.run_id.chars().take(8).collect();
        let text = format!(
            "Filled from run {short_id} ({}). Review and edit before starting.",
            format_when(&group.latest.created_at)
        );

        match prefill_for(&group.latest, &view) {
            Prefill::Target { form: filled } => {
                target_form.set(Some(filled));
                target_error.set(None);
                errors.set(Vec::new());
                start_mode.set(StartMode::Target);
                banner.set(Some(Banner {
                    text,
                    notice: None,
                    header_names: Vec::new(),
                }));
            }
            Prefill::Custom {
                form: filled,
                notice,
                header_names,
            } => {
                reference.set(Some((*filled).clone()));
                form.set(*filled);
                errors.set(Vec::new());
                start_mode.set(StartMode::Custom);
                banner.set(Some(Banner {
                    text,
                    notice,
                    header_names,
                }));
            }
            Prefill::Unavailable { .. } => return,
        }

        // The fields that took a new value flash once
        flash.set(true);
        spawn(async move {
            TimeoutFuture::new(1_300).await;
            flash.set(false);
        });
    };

    // What the form was filled from, with the header names only
    let banner_view: Element = match banner() {
        Some(filled) => {
            let header_hint = filled.header_names.join(", ");

            rsx! {
                div {
                    class: "mb-(--space-4) flex flex-col gap-(--space-2) rounded-(--round-md) border border-(color:--clr-border) bg-(--clr-surface) p-(--space-3)",
                    role: "status",
                    p { class: "text-(length:--fs-sm) text-(color:--clr-text)", "{filled.text}" }
                    if let Some(notice) = &filled.notice {
                        p { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)", "{notice}" }
                    }
                    if !filled.header_names.is_empty() {
                        p { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)",
                            "That run sent these headers: {header_hint}. Values are never stored, enter them again."
                        }
                    }
                    div {
                        button {
                            r#type: "button",
                            class: "{secondary}",
                            onclick: move |_| {
                                form.set(FormState::default());
                                reference.set(None);
                                errors.set(Vec::new());
                                target_error.set(None);
                                banner.set(None);

                                if let Loaded::Ready(view) = &*targets.peek() {
                                    start_mode.set(default_mode(view));
                                }
                            },
                            "Clear"
                        }
                    }
                }
            }
        }
        None => rsx! {},
    };

    // The switch between saved targets and custom runs
    let switch_bar: Element = match &targets_state {
        Loaded::Ready(view) if !view.targets.is_empty() || custom_allowed => {
            let no_targets = view.targets.is_empty();
            let selected = usize::from(effective_mode == StartMode::Custom);

            rsx! {
                div { class: "flex flex-wrap items-center gap-(--space-3)",
                    SegmentSwitch {
                        label: "Start from",
                        options: vec![
                            ("Saved target", running || no_targets),
                            ("Custom", running || !custom_allowed),
                        ],
                        selected,
                        width_rem: 8.0,
                        locked: if custom_allowed { None } else { Some(1) },
                        lock_hint: "Custom runs are turned off on this server",
                        on_pick: move |index: usize| {
                            start_mode
                                .set(if index == 0 { StartMode::Target } else { StartMode::Custom });
                        },
                    }
                    if effective_mode == StartMode::Custom && custom_allowed {
                        button {
                            r#type: "button",
                            class: "{secondary}",
                            disabled: running,
                            onclick: move |_| paste_open.set(true),
                            Icon { kind: IconKind::Copy, class: "size-(--icon-sm)" }
                            "Paste JSON"
                        }
                    }
                }
            }
        }
        _ => rsx! {},
    };

    let body: Element = match phase() {
        Phase::Failed(error) => {
            // A busy API names the run that holds it, so it can be watched instead
            let active = active_run_id(&error);
            let refused_title = if matches!(error, ApiError::Network { .. }) {
                String::new()
            } else {
                "Couldn't start the run.".to_string()
            };

            rsx! {
                if let Some(run_id) = active {
                    ErrorState {
                        title: "Another benchmark is already running.",
                        message: "Only one run can go at a time. You can watch it instead.",
                        kind: ErrorKind::Amber,
                        retryable: false,
                        actions: Some(rsx! {
                            Link {
                                to: Route::LiveRun {
                                    run_id,
                                },
                                class: "{primary}",
                                "Watch the run"
                            }
                        }),
                        on_retry: move |_| {},
                    }
                } else {
                    ApiErrorPanel {
                        title: refused_title,
                        error,
                        on_retry: move |_| phase.set(Phase::Editing),
                    }
                }
            }
        }
        Phase::Editing | Phase::Running => match (effective_mode, &targets_state) {
            (_, Loaded::Loading) => rsx! {
                TargetSkeleton {}
            },
            (_, Loaded::Failed(error)) => rsx! {
                ApiErrorPanel {
                    error: error.clone(),
                    on_retry: move |_| reload.with_mut(|count| *count += 1),
                }
            },
            (StartMode::Target, Loaded::Ready(view)) if !view.targets.is_empty() => {
                let options = view.targets.clone();
                let runtimes = runtimes.clone();

                rsx! {
                    form {
                        id: FORM_ID,
                        class: "flex flex-col gap-(--space-4)",
                        onsubmit: move |event| {
                            event.prevent_default();
                            submit_target(());
                        },
                        SavedTargetForm {
                            targets: options,
                            runtimes,
                            form: target_form,
                            running,
                        }
                    }
                }
            }
            (_, Loaded::Ready(_)) if custom_allowed => rsx! {
                form {
                    id: FORM_ID,
                    class: if flash() { "fill-flash flex flex-col gap-(--space-4)" } else { "flex flex-col gap-(--space-4)" },
                    onsubmit: move |event| {
                        event.prevent_default();
                        submit_custom(());
                    },
                    CustomForm {
                        form,
                        reference,
                        errors: current_errors.clone(),
                        running,
                    }
                }
            },
            (_, Loaded::Ready(_)) => rsx! {
                div {
                    class: "flex flex-col gap-(--space-3) rounded-(--round-lg) border border-(color:--clr-warning)/40 bg-(--clr-warning-soft) p-(--space-4) sm:flex-row sm:items-center sm:justify-between",
                    role: "status",
                    div { class: "flex min-w-0 items-start gap-(--space-3) text-(color:--clr-warning-text)",
                        Icon { kind: IconKind::Alert, class: "mt-px size-(--icon-md) shrink-0" }
                        div { class: "min-w-0",
                            p { class: "text-(length:--fs-base) font-(weight:--fw-semibold)",
                                "No saved targets"
                            }
                            p { class: "text-(length:--fs-sm)",
                                "Custom runs are disabled on this server, and no saved targets are defined."
                            }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "{secondary}",
                        onclick: move |_| reload.with_mut(|count| *count += 1),
                        "Reload"
                    }
                }
            },
        },
    };

    // The panel with the summary, the checks and the Run button
    let side: Element = if matches!(phase(), Phase::Failed(_)) {
        rsx! {}
    } else {
        match (effective_mode, &targets_state) {
            (StartMode::Target, Loaded::Ready(view)) if !view.targets.is_empty() => {
                let selected = current_target
                    .as_ref()
                    .and_then(|filled| target_by_id(view, &filled.target_id));

                let plan = selected
                    .zip(current_target.as_ref())
                    .map(|(target, filled)| target_plan(target, filled))
                    .unwrap_or_default();
                let answer = estimate();
                let (timeline, estimate_label) = panel_parts(&plan, answer.as_ref());

                rsx! {
                    RunPanel {
                        lines: target_lines(selected, current_target.as_ref()),
                        timeline,
                        estimate: estimate_label,
                        checks: target_check_list.clone(),
                        form_id: FORM_ID,
                        ready: target_ready,
                        running,
                        message: target_message.clone(),
                    }
                }
            }
            (_, Loaded::Ready(_)) if custom_allowed => {
                let plan = custom_plan(&state);
                let answer = estimate();
                let (timeline, estimate_label) = panel_parts(&plan, answer.as_ref());

                rsx! {
                    RunPanel {
                        lines: custom_lines(&state),
                        timeline,
                        estimate: estimate_label,
                        checks: custom_check_list.clone(),
                        form_id: FORM_ID,
                        ready: custom_ready,
                        running,
                        copy_json: live
                            .as_ref()
                            .ok()
                            .and_then(|request| serde_json::to_string_pretty(request).ok()),
                    }
                }
            }
            _ => rsx! {},
        }
    };

    // Mini cards of earlier runs of the project being set up
    let previous: Element = match (&history_state, &targets_state) {
        (Loaded::Ready(groups), Loaded::Ready(view)) => {
            let items = match recent() {
                Some(Ok(page)) => page.items,
                _ => Vec::new(),
            };

            rsx! {
                PreviousRuns {
                    cards: cards(groups, &items, view, &wanted_path(), now_epoch_ms()),
                    on_use: move |index: usize| apply_history(index),
                }
            }
        }
        _ => rsx! {},
    };

    rsx! {
        document::Title { "New benchmark · Benchmark" }
        div {
            class: "mx-auto w-full max-w-6xl",
            PageHeader {
                title: "New benchmark",
                description: "Build, start and measure a project. You follow it live while it runs, and the result is stored in the run history.",
                Link { to: Route::runs(), class: "{secondary}", "All runs" }
            }
            if paste_open() {
                PasteModal {
                    form,
                    errors,
                    on_filled: move |_| {
                        flash.set(true);
                        spawn(async move {
                            TimeoutFuture::new(1_300).await;
                            flash.set(false);
                        });
                    },
                    on_close: move |_| paste_open.set(false),
                }
            }
            {banner_view}
            div { class: "grid grid-cols-1 gap-(--space-5) lg:grid-cols-[minmax(0,1fr)_22rem] lg:items-start",
                div { class: "flex min-w-0 flex-col gap-(--space-4)",
                    {switch_bar}
                    {body}
                }
                aside { class: "flex flex-col gap-(--space-4) lg:sticky lg:top-[calc(var(--topbar-height)+1rem)] lg:max-h-[calc(100vh-var(--topbar-height)-2rem)] lg:overflow-y-auto",
                    {side}
                    {previous}
                }
            }
        }
    }
}
