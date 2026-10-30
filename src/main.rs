mod app;
mod components;
mod features;
mod format;
mod shell;

fn main() {
    dioxus::launch(app::App);
}
