mod quiet;

use std::env;
use std::path::Path;
use std::process::{exit, Command};

use benchmark_ui::config::env::Env;
use quiet::Show;

// Where `cargo build-prod` writes the production bundle
const BUNDLE_DIR: &str = "target/dx/benchmark-ui/release/web/public";

fn main() {
    let task = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: cargo <dev|start|build|verify>");
        exit(1);
    });

    match task.as_str() {
        "dev" => run_dev(),
        "start" => run_static_server(),
        "build" => run_build(),
        "verify" => run_verify(),
        _ => {
            eprintln!("Unknown task: {task}. Use dev | start | build | verify");
            exit(1);
        }
    }
}

// Loads the env file of a profile and validates it with the schema from the main src
fn load_env(profile: &str) -> Env {
    let env_file = format!(".env.{profile}");

    dotenvy::from_filename(&env_file)
        .unwrap_or_else(|_| panic!("Failed to load environment file: {env_file}"));

    Env::load().unwrap_or_else(|error| {
        eprintln!("[env] Validation failed for {env_file}: {error}");
        exit(1);
    })
}

fn run_dev() {
    let env = load_env("development");

    println!("-> Loaded .env.development (Port: {})", env.app_port);

    let status = Command::new("dx")
        .arg("serve")
        .arg("--port")
        .arg(env.app_port.to_string())
        .status()
        .expect("Failed to execute dx serve");

    exit(status.code().unwrap_or(1));
}

// Serves the production bundle with miniserve on APP_PORT from .env.production
fn run_static_server() {
    let env = load_env("production");

    if !Path::new(BUNDLE_DIR).join("index.html").exists() {
        eprintln!("No bundle found in {BUNDLE_DIR}. Run `cargo build-prod` first.");
        exit(1);
    }

    let port = env.app_port.to_string();

    println!("-> Loaded .env.production");
    println!("-> Serving {BUNDLE_DIR} on 127.0.0.1:{port}");

    let status = Command::new("miniserve")
        .arg(BUNDLE_DIR)
        .arg("--spa")
        .arg("--index")
        .arg("index.html")
        .arg("--interfaces")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(&port)
        .status()
        .unwrap_or_else(|error| {
            eprintln!(
                "Could not run miniserve ({error}). Install it with: cargo install miniserve"
            );
            exit(1);
        });

    exit(status.code().unwrap_or(1));
}

// Creates the optimized web bundle with the API address from .env.production
fn run_build() {
    let env = load_env("production");

    let api_base_url = env
        .api_base_url
        .as_deref()
        .map(|url| url.trim().trim_end_matches('/'))
        .filter(|url| !url.is_empty())
        .unwrap_or_else(|| {
            eprintln!("API_BASE_URL in .env.production is required for the build");
            exit(1);
        });

    if !(api_base_url.starts_with("http://") || api_base_url.starts_with("https://")) {
        eprintln!(
            "API_BASE_URL in .env.production must start with http:// or https://, got \"{api_base_url}\""
        );
        exit(1);
    }

    println!("-> Loaded .env.production");
    println!("-> API_BASE_URL compiled into the bundle: {api_base_url}");

    let status = Command::new("dx")
        .arg("bundle")
        .arg("--platform")
        .arg("web")
        .arg("--release")
        .env("API_BASE_URL", api_base_url)
        .status()
        .expect("Failed to execute dx bundle");

    if status.success() {
        println!("-> Bundle finished, the output path is printed by dx above");
    }

    exit(status.code().unwrap_or(1));
}

fn run_step(args: &[String], show: Show) {
    let mut cmd = Command::new("cargo");
    cmd.args(args);

    match quiet::run(&cmd, show) {
        Ok(true) => {}
        Ok(false) => exit(1),
        Err(error) => {
            eprintln!("Failed to run cargo: {error}");
            exit(1);
        }
    }
}

fn run_verify() {
    let steps: [(&str, Vec<&str>, Show); 4] = [
        ("cargo fmt", vec!["fmt"], Show::All),
        (
            "cargo check",
            vec!["check", "--workspace", "--all-targets"],
            Show::All,
        ),
        (
            "cargo clippy",
            vec![
                "clippy",
                "--workspace",
                "--all-targets",
                "--locked",
                "--",
                "-D",
                "warnings",
            ],
            Show::All,
        ),
        ("cargo test", vec!["test", "--workspace"], Show::TestSummary),
    ];

    for (label, args, show) in steps {
        println!("-> {label}");

        let args: Vec<String> = args.into_iter().map(str::to_string).collect();

        run_step(&args, show);
    }

    println!("-> verify passed");
}
