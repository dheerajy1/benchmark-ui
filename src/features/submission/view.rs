use dioxus::prelude::*;

use super::{
    api::{submit_live, submit_target_live},
    form::{error_for, validate, Field, FieldError, FormState, WorkloadMode},
    history_panel::{format_when, now_epoch_ms, row_for, HistoryPanel, HistoryRow},
    prefill::{prefill_for, Prefill},
    target_run::{validate_target, TargetForm},
};
use crate::{
    app::Route,
    components::{
        button_class, ButtonVariant, Card, CardHeader, EmptyState, FormField, Icon, IconKind,
        LoadingPanel, PageHeader, INPUT_CLASS, TEXTAREA_CLASS,
    },
    features::{
        configurations::{fetch_configurations, group, ConfigurationEntry, ConfigurationGroup},
        connection::{ApiError, ApiErrorPanel},
        live::{active_run_id, save_token, SessionTokenStore},
        targets::{fetch_targets, TargetView, TargetsView},
    },
};

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

// Read-only facts about a target, as label and value. Header names only, never values.
fn target_lines(target: &TargetView) -> Vec<(&'static str, String)> {
    let mut lines = vec![("Path", target.path.clone())];

    if let Some(startup) = &target.startup {
        let command = format!(
            "{} {}",
            startup.command.program,
            startup.command.args.join(" ")
        );

        lines.push(("Start command", command.trim().to_string()));
        lines.push((
            "Readiness",
            format!(
                "{}:{} {} ({} → {})",
                startup.readiness.host,
                startup.readiness.port,
                startup.readiness.path,
                startup.readiness.method,
                startup.readiness.expected_status
            ),
        ));

        let readiness_headers = match &startup.readiness.header_names {
            Some(names) => names_text(names),
            None => "Server health credentials".to_string(),
        };

        lines.push(("Readiness headers", readiness_headers));
    }

    if let Some(workload) = &target.workload {
        lines.push(("Workload", format!("{} {}", workload.method, workload.path)));

        let amount = if workload.mode == "duration" {
            format!("{} s", workload.duration_seconds.unwrap_or_default())
        } else {
            format!("{} requests", workload.requests.unwrap_or_default())
        };

        lines.push(("Default amount", amount));
        lines.push(("Concurrency", workload.concurrency.to_string()));
        lines.push(("Workload headers", names_text(&workload.header_names)));
    }

    lines
}

fn names_text(names: &[String]) -> String {
    if names.is_empty() {
        "None".to_string()
    } else {
        names.join(", ")
    }
}

// The New Benchmark page opened from a run's "Use these settings" button
#[component]
pub fn SubmissionFrom(run_id: String) -> Element {
    rsx! {
        Submission { from_run: Some(run_id) }
    }
}

// Form that starts a live run, from a saved target or from a custom form, and then opens its
// live view
#[component]
pub fn Submission(#[props(default)] from_run: Option<String>) -> Element {
    let mut form = use_signal(FormState::default);
    let mut errors = use_signal(Vec::<FieldError>::new);
    let mut phase = use_signal(|| Phase::Editing);
    let mut targets = use_signal(|| Loaded::<TargetsView>::Loading);
    let mut history = use_signal(|| Loaded::<Vec<ConfigurationGroup>>::Loading);
    let mut all_runs = use_signal(Vec::<ConfigurationEntry>::new);
    let mut start_mode = use_signal(|| StartMode::Custom);
    let mut target_form = use_signal(|| None::<TargetForm>);
    let mut target_error = use_signal(|| None::<String>);
    let mut banner = use_signal(|| None::<Banner>);
    let mut history_open = use_signal(|| false);
    let mut reload = use_signal(|| 0u32);
    let nav = use_navigator();

    // Loads the saved targets and the run history when the page opens, and again after a retry
    use_effect(move || {
        let _ = reload();

        spawn(async move {
            match fetch_targets().await {
                Ok(view) => {
                    if view.targets.is_empty() {
                        start_mode.set(StartMode::Custom);
                    } else {
                        start_mode.set(StartMode::Target);

                        if target_form.peek().is_none() {
                            target_form.set(view.targets.first().map(TargetForm::for_target));
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
    // the runs have loaded. It runs only once.
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

        let Some(entry) = all_runs().into_iter().find(|entry| entry.run_id == run_id) else {
            banner.set(Some(Banner {
                text: format!(
                    "Run {} is not among the most recent runs, so its settings cannot be filled in.",
                    crate::format::short_id(&run_id)
                ),
                notice: None,
                header_names: Vec::new(),
            }));
            return;
        };

        let text = format!(
            "Filled from run {} ({}). Review and edit before starting.",
            crate::format::short_id(&run_id),
            format_when(&entry.created_at)
        );

        match prefill_for(&entry, &view) {
            Prefill::Target { form: filled } => {
                target_form.set(Some(filled));
                target_error.set(None);
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
                form.set(filled);
                errors.set(Vec::new());
                start_mode.set(StartMode::Custom);
                banner.set(Some(Banner {
                    text,
                    notice,
                    header_names,
                }));
            }
            Prefill::Unavailable { reason } => {
                banner.set(Some(Banner {
                    text: reason,
                    notice: None,
                    header_names: Vec::new(),
                }));
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

        let nav = nav.clone();

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

        let nav = nav.clone();

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
                form.set(filled);
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

        history_open.set(false);
    };

    // The history panel, its loading state or its error
    let panel: Element = if history_open() {
        match (&history_state, &targets_state) {
            (Loaded::Ready(groups), Loaded::Ready(view)) => {
                let rows: Vec<HistoryRow> = groups
                    .iter()
                    .enumerate()
                    .map(|(index, group)| row_for(index, group, view, now_epoch_ms()))
                    .collect();

                rsx! {
                    HistoryPanel {
                        rows,
                        on_use: move |index: usize| apply_history(index),
                        on_close: move |_| history_open.set(false),
                    }
                }
            }
            (Loaded::Failed(error), _) | (_, Loaded::Failed(error)) => rsx! {
                ApiErrorPanel {
                    error: error.clone(),
                    on_retry: move |_| reload.with_mut(|count| *count += 1),
                }
            },
            _ => rsx! {
                LoadingPanel { label: "Loading previous runs" }
            },
        }
    } else {
        rsx! {}
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
            let target_class = if effective_mode == StartMode::Target {
                primary.clone()
            } else {
                secondary.clone()
            };
            let custom_class = if effective_mode == StartMode::Custom {
                primary.clone()
            } else {
                secondary.clone()
            };
            let no_targets = view.targets.is_empty();

            rsx! {
                div {
                    class: "mb-(--space-4) flex flex-wrap items-center gap-(--space-2)",
                    role: "group",
                    "aria-label": "Start from",
                    span { class: "text-(length:--fs-sm) text-(color:--clr-text-muted)", "Start from" }
                    button {
                        r#type: "button",
                        class: "{target_class}",
                        "aria-pressed": if effective_mode == StartMode::Target { "true" } else { "false" },
                        disabled: running || no_targets,
                        onclick: move |_| start_mode.set(StartMode::Target),
                        "Saved target"
                    }
                    button {
                        r#type: "button",
                        class: "{custom_class}",
                        "aria-pressed": if effective_mode == StartMode::Custom { "true" } else { "false" },
                        disabled: running || !custom_allowed,
                        onclick: move |_| start_mode.set(StartMode::Custom),
                        "Custom"
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

            rsx! {
                ApiErrorPanel { error, on_retry: move |_| phase.set(Phase::Editing) }
                if let Some(run_id) = active {
                    div { class: "mt-(--space-3)",
                        Link {
                            to: Route::LiveRun {
                                run_id,
                            },
                            class: "{primary}",
                            "Watch the active run"
                        }
                    }
                }
            }
        }
        Phase::Editing | Phase::Running => match (effective_mode, &targets_state) {
            (_, Loaded::Loading) => rsx! {
                LoadingPanel { label: "Loading saved targets" }
            },
            (_, Loaded::Failed(error)) => rsx! {
                ApiErrorPanel {
                    error: error.clone(),
                    on_retry: move |_| reload.with_mut(|count| *count += 1),
                }
            },
            (StartMode::Target, Loaded::Ready(view)) if !view.targets.is_empty() => {
                let lookup = view.clone();
                let chosen = current_target.clone();
                let selected_id = chosen
                    .as_ref()
                    .map(|filled| filled.target_id.clone())
                    .unwrap_or_default();
                let selected_target = chosen
                    .as_ref()
                    .and_then(|filled| target_by_id(view, &filled.target_id))
                    .cloned();
                let lines = selected_target
                    .as_ref()
                    .map(target_lines)
                    .unwrap_or_default();
                let has_workload = chosen
                    .as_ref()
                    .map(|filled| filled.has_workload)
                    .unwrap_or(false);
                let workload_mode = chosen
                    .as_ref()
                    .map(|filled| filled.mode)
                    .unwrap_or(WorkloadMode::Requests);
                let amount = chosen
                    .as_ref()
                    .map(|filled| filled.amount.clone())
                    .unwrap_or_default();
                let concurrency = chosen
                    .as_ref()
                    .map(|filled| filled.concurrency.clone())
                    .unwrap_or_default();
                let options = view.targets.clone();

                rsx! {
                    form {
                        class: "flex flex-col gap-(--space-4)",
                        onsubmit: move |event| {
                            event.prevent_default();
                            submit_target(());
                        },
                        Card {
                            CardHeader {
                                title: "Saved target",
                                subtitle: "A project defined on the server. Its path, start command and headers stay on the server.",
                            }
                            div { class: "flex flex-col gap-(--space-4) p-(--space-4)",
                                FormField { label: "Target",
                                    select {
                                        class: "{INPUT_CLASS}",
                                        disabled: running,
                                        onchange: move |event| {
                                            if let Some(target) = target_by_id(&lookup, &event.value()) {
                                                target_form.set(Some(TargetForm::for_target(target)));
                                            }
                                        },
                                        for target in options.iter() {
                                            option {
                                                value: "{target.id}",
                                                selected: target.id == selected_id,
                                                "{target.name}"
                                            }
                                        }
                                    }
                                }
                                if selected_target.is_some() {
                                    div { class: "flex flex-col gap-(--space-2) rounded-(--round-md) border border-(color:--clr-border) bg-(--clr-surface-raised) p-(--space-3)",
                                        for (label, value) in lines.iter() {
                                            p { class: "text-(length:--fs-sm) text-(color:--clr-text)",
                                                span { class: "text-(color:--clr-text-muted)", "{label}: " }
                                                span { class: "break-all", "{value}" }
                                            }
                                        }
                                    }
                                }
                                if has_workload {
                                    div { class: "grid grid-cols-1 gap-(--space-4) sm:grid-cols-3",
                                        FormField { label: "Run until",
                                            select {
                                                class: "{INPUT_CLASS}",
                                                disabled: running,
                                                onchange: move |event| {
                                                    let mode = if event.value() == "duration" {
                                                        WorkloadMode::Duration
                                                    } else {
                                                        WorkloadMode::Requests
                                                    };
                                                    target_form.with_mut(|filled| {
                                                        if let Some(filled) = filled {
                                                            filled.mode = mode;
                                                        }
                                                    });
                                                },
                                                option {
                                                    value: "requests",
                                                    selected: workload_mode == WorkloadMode::Requests,
                                                    "A number of requests"
                                                }
                                                option {
                                                    value: "duration",
                                                    selected: workload_mode == WorkloadMode::Duration,
                                                    "A duration"
                                                }
                                            }
                                        }
                                        FormField {
                                            label: if workload_mode == WorkloadMode::Requests { "Requests" } else { "Duration (seconds)" },
                                            hint: if workload_mode == WorkloadMode::Requests { "" } else { "Up to 300 seconds" },
                                            input {
                                                r#type: "text",
                                                inputmode: "numeric",
                                                class: "{INPUT_CLASS}",
                                                value: "{amount}",
                                                disabled: running,
                                                oninput: move |event| {
                                                    target_form.with_mut(|filled| {
                                                        if let Some(filled) = filled {
                                                            filled.amount = event.value();
                                                        }
                                                    });
                                                },
                                            }
                                        }
                                        FormField { label: "Concurrency", hint: "Simultaneous requests",
                                            input {
                                                r#type: "text",
                                                inputmode: "numeric",
                                                class: "{INPUT_CLASS}",
                                                value: "{concurrency}",
                                                disabled: running,
                                                oninput: move |event| {
                                                    target_form.with_mut(|filled| {
                                                        if let Some(filled) = filled {
                                                            filled.concurrency = event.value();
                                                        }
                                                    });
                                                },
                                            }
                                        }
                                    }
                                } else {
                                    p { class: "text-(length:--fs-sm) text-(color:--clr-text-muted)",
                                        "This target defines no workload, so the run only starts the target and checks its readiness."
                                    }
                                }
                                p { class: "text-(length:--fs-xs) text-(color:--clr-text-faint)",
                                    "Targets are changed by editing the server's targets file, not here."
                                }
                            }
                        }
                        div { class: "flex flex-col items-start gap-(--space-3)",
                            button {
                                r#type: "submit",
                                class: "{primary}",
                                disabled: running,
                                Icon { kind: IconKind::Play, class: "size-(--icon-sm)" }
                                if running {
                                    "Running benchmark…"
                                } else {
                                    "Run benchmark"
                                }
                            }
                            if let Some(message) = &target_message {
                                p {
                                    class: "w-full rounded-(--round-md) border border-(color:--clr-danger) bg-(--clr-danger-soft) px-(--space-3) py-(--space-2) text-(length:--fs-sm) text-(color:--clr-danger)",
                                    role: "alert",
                                    "{message}"
                                }
                            }
                        }
                    }
                }
            }
            (_, Loaded::Ready(_)) if custom_allowed => rsx! {
                form {
                    class: "flex flex-col gap-(--space-4)",
                    onsubmit: move |event| {
                        event.prevent_default();
                        submit_custom(());
                    },
                    Card {
                        CardHeader {
                            title: "Target",
                            subtitle: "The project directory on the machine running benchmark-api.",
                        }
                        div { class: "p-(--space-4)",
                            FormField {
                                label: "Project path",
                                hint: "Absolute path, for example /home/me/projects/my-api",
                                error: error_for(&current_errors, Field::Path),
                                input {
                                    r#type: "text",
                                    class: "{INPUT_CLASS}",
                                    value: "{state.path}",
                                    disabled: running,
                                    oninput: move |event| form.with_mut(|f| f.path = event.value()),
                                }
                            }
                        }
                    }
                    Card {
                        CardHeader {
                            title: "Startup",
                            subtitle: "How to start the target and when it counts as ready. Optional.",
                        }
                        div { class: "flex flex-col gap-(--space-4) p-(--space-4)",
                            label { class: "flex cursor-pointer items-center gap-(--space-3) rounded-(--round-md) border border-(color:--clr-border) bg-(--clr-surface-raised) px-(--space-3) py-(--space-2) text-(length:--fs-sm) text-(color:--clr-text) transition-colors hover:border-(color:--clr-border-strong)",
                                input {
                                    r#type: "checkbox",
                                    class: "size-4 cursor-pointer accent-(--clr-primary)",
                                    checked: state.startup_enabled,
                                    disabled: running,
                                    onchange: move |event| form.with_mut(|f| f.startup_enabled = event.checked()),
                                }
                                "Configure startup explicitly"
                            }
                            if state.startup_enabled {
                                div { class: "grid grid-cols-1 gap-(--space-4) sm:grid-cols-2",
                                    FormField {
                                        label: "Program",
                                        error: error_for(&current_errors, Field::Program),
                                        input {
                                            r#type: "text",
                                            class: "{INPUT_CLASS}",
                                            placeholder: "bun",
                                            value: "{state.program}",
                                            disabled: running,
                                            oninput: move |event| form.with_mut(|f| f.program = event.value()),
                                        }
                                    }
                                    FormField { label: "Arguments", hint: "Separated by spaces",
                                        input {
                                            r#type: "text",
                                            class: "{INPUT_CLASS}",
                                            placeholder: "run start",
                                            value: "{state.args}",
                                            disabled: running,
                                            oninput: move |event| form.with_mut(|f| f.args = event.value()),
                                        }
                                    }
                                    FormField {
                                        label: "Readiness host",
                                        error: error_for(&current_errors, Field::Host),
                                        input {
                                            r#type: "text",
                                            class: "{INPUT_CLASS}",
                                            value: "{state.host}",
                                            disabled: running,
                                            oninput: move |event| form.with_mut(|f| f.host = event.value()),
                                        }
                                    }
                                    FormField {
                                        label: "Readiness port",
                                        error: error_for(&current_errors, Field::Port),
                                        input {
                                            r#type: "text",
                                            inputmode: "numeric",
                                            class: "{INPUT_CLASS}",
                                            placeholder: "5001",
                                            value: "{state.port}",
                                            disabled: running,
                                            oninput: move |event| form.with_mut(|f| f.port = event.value()),
                                        }
                                    }
                                    FormField {
                                        label: "Readiness path",
                                        hint: "Checked with GET",
                                        error: error_for(&current_errors, Field::ReadinessPath),
                                        input {
                                            r#type: "text",
                                            class: "{INPUT_CLASS}",
                                            value: "{state.readiness_path}",
                                            disabled: running,
                                            oninput: move |event| form.with_mut(|f| f.readiness_path = event.value()),
                                        }
                                    }
                                    FormField {
                                        label: "Expected status",
                                        error: error_for(&current_errors, Field::ExpectedStatus),
                                        input {
                                            r#type: "text",
                                            inputmode: "numeric",
                                            class: "{INPUT_CLASS}",
                                            value: "{state.expected_status}",
                                            disabled: running,
                                            oninput: move |event| form.with_mut(|f| f.expected_status = event.value()),
                                        }
                                    }
                                }
                                FormField {
                                    label: "Readiness headers",
                                    hint: "Optional. One per line as Name: value. Leave empty to probe with the API's default health credentials. Never stored.",
                                    error: error_for(&current_errors, Field::ReadinessHeaders),
                                    textarea {
                                        class: "{TEXTAREA_CLASS}",
                                        rows: "3",
                                        spellcheck: "false",
                                        autocomplete: "off",
                                        placeholder: "X-Client-Id: my-client\nX-Client-Secret: my-secret",
                                        value: "{state.readiness_headers}",
                                        disabled: running,
                                        oninput: move |event| form.with_mut(|f| f.readiness_headers = event.value()),
                                    }
                                }
                            }
                        }
                    }
                    Card {
                        CardHeader {
                            title: "Workload",
                            subtitle: "HTTP load sent to the running target. Optional, needed for latency, memory and CPU.",
                        }
                        div { class: "flex flex-col gap-(--space-4) p-(--space-4)",
                            label { class: "flex cursor-pointer items-center gap-(--space-3) rounded-(--round-md) border border-(color:--clr-border) bg-(--clr-surface-raised) px-(--space-3) py-(--space-2) text-(length:--fs-sm) text-(color:--clr-text) transition-colors hover:border-(color:--clr-border-strong)",
                                input {
                                    r#type: "checkbox",
                                    class: "size-4 cursor-pointer accent-(--clr-primary)",
                                    checked: state.workload_enabled,
                                    disabled: running,
                                    onchange: move |event| form.with_mut(|f| f.workload_enabled = event.checked()),
                                }
                                "Send an HTTP workload"
                            }
                            if state.workload_enabled {
                                div { class: "grid grid-cols-1 gap-(--space-4) sm:grid-cols-2",
                                    FormField {
                                        label: "Request path",
                                        hint: "Sent with GET",
                                        error: error_for(&current_errors, Field::WorkloadPath),
                                        input {
                                            r#type: "text",
                                            class: "{INPUT_CLASS}",
                                            placeholder: "/items",
                                            value: "{state.workload_path}",
                                            disabled: running,
                                            oninput: move |event| form.with_mut(|f| f.workload_path = event.value()),
                                        }
                                    }
                                    FormField { label: "Run until",
                                        select {
                                            class: "{INPUT_CLASS}",
                                            disabled: running,
                                            onchange: move |event| {
                                                let mode = if event.value() == "duration" {
                                                    WorkloadMode::Duration
                                                } else {
                                                    WorkloadMode::Requests
                                                };
                                                form.with_mut(|f| f.mode = mode);
                                            },
                                            option {
                                                value: "requests",
                                                selected: state.mode == WorkloadMode::Requests,
                                                "A number of requests"
                                            }
                                            option {
                                                value: "duration",
                                                selected: state.mode == WorkloadMode::Duration,
                                                "A duration"
                                            }
                                        }
                                    }
                                    FormField {
                                        label: if state.mode == WorkloadMode::Requests { "Requests" } else { "Duration (seconds)" },
                                        hint: if state.mode == WorkloadMode::Requests { "" } else { "Up to 300 seconds" },
                                        error: error_for(&current_errors, Field::Amount),
                                        input {
                                            r#type: "text",
                                            inputmode: "numeric",
                                            class: "{INPUT_CLASS}",
                                            value: "{state.amount}",
                                            disabled: running,
                                            oninput: move |event| form.with_mut(|f| f.amount = event.value()),
                                        }
                                    }
                                    FormField {
                                        label: "Concurrency",
                                        hint: "Simultaneous requests",
                                        error: error_for(&current_errors, Field::Concurrency),
                                        input {
                                            r#type: "text",
                                            inputmode: "numeric",
                                            class: "{INPUT_CLASS}",
                                            value: "{state.concurrency}",
                                            disabled: running,
                                            oninput: move |event| form.with_mut(|f| f.concurrency = event.value()),
                                        }
                                    }
                                }
                                FormField {
                                    label: "Request headers",
                                    hint: "Optional. One per line as Name: value, for example X-Client-Id: my-client. Sent with every request of the workload and never stored.",
                                    error: error_for(&current_errors, Field::WorkloadHeaders),
                                    textarea {
                                        class: "{TEXTAREA_CLASS}",
                                        rows: "3",
                                        spellcheck: "false",
                                        autocomplete: "off",
                                        placeholder: "X-Client-Id: my-client\nX-Client-Secret: my-secret",
                                        value: "{state.workload_headers}",
                                        disabled: running,
                                        oninput: move |event| form.with_mut(|f| f.workload_headers = event.value()),
                                    }
                                }
                            }
                        }
                    }
                    div { class: "flex flex-col items-start gap-(--space-3)",
                        button {
                            r#type: "submit",
                            class: "{primary}",
                            disabled: running,
                            Icon { kind: IconKind::Play, class: "size-(--icon-sm)" }
                            if running {
                                "Running benchmark…"
                            } else {
                                "Run benchmark"
                            }
                        }
                        if running {
                            div {
                                class: "flex w-full items-start gap-(--space-3) rounded-(--round-md) border border-(color:--clr-border) bg-(--clr-surface) px-(--space-3) py-(--space-3)",
                                role: "status",
                                span { class: "mt-(--space-1) inline-block size-4 shrink-0 animate-spin rounded-(--round-full) border-2 border-(color:--clr-border-strong) border-t-(color:--clr-accent)" }
                                p { class: "text-(length:--fs-sm) text-(color:--clr-text-muted)",
                                    "Starting the run. You will be taken to its live view in a moment."
                                }
                            }
                        }
                        if !current_errors.is_empty() {
                            p {
                                class: "w-full rounded-(--round-md) border border-(color:--clr-danger) bg-(--clr-danger-soft) px-(--space-3) py-(--space-2) text-(length:--fs-sm) text-(color:--clr-danger)",
                                role: "alert",
                                "Fix the highlighted fields and try again."
                            }
                        }
                    }
                }
            },
            (_, Loaded::Ready(_)) => rsx! {
                EmptyState {
                    title: "No saved targets",
                    description: "Custom runs are disabled on this server, and no saved targets are defined.",
                    p { class: "text-(length:--fs-sm) text-(color:--clr-text-muted)",
                        "Add a project to the targets file on the server, then reload this page."
                    }
                }
            },
        },
    };

    rsx! {
        document::Title { "New benchmark · Benchmark" }
        div { class: "mx-auto w-full max-w-(--form-max)",
            PageHeader {
                title: "New benchmark",
                description: "Build, start and measure a project. You follow it live while it runs, and the result is stored in the run history.",
                button {
                    r#type: "button",
                    class: "{secondary}",
                    "aria-expanded": if history_open() { "true" } else { "false" },
                    onclick: move |_| {
                        let open = *history_open.peek();
                        history_open.set(!open);
                    },
                    Icon { kind: IconKind::History, class: "size-(--icon-sm)" }
                    "Previous runs"
                }
                Link { to: Route::History {}, class: "{secondary}", "All runs" }
            }
            {panel}
            {banner_view}
            {switch_bar}
            {body}
        }
    }
}
