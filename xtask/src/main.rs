use serde::Deserialize;
use std::env;
use std::path::Path;
use std::process::Command;
use validator::Validate;

#[derive(Clone, Debug, Deserialize, Validate)]
pub struct EnvSchema {
    #[validate(range(min = 1, message = "APP_PORT must be a valid port number"))]
    pub app_port: u16,
}

// Where `cargo xtask build` writes the production bundle
const BUNDLE_DIR: &str = "target/dx/benchmark-frontend/release/web/public";

// Variables the production build needs, API_BASE_URL is compiled into the WASM bundle
#[derive(Clone, Debug, Deserialize, Validate)]
pub struct BuildEnvSchema {
    #[validate(range(min = 1, message = "APP_PORT must be a valid port number"))]
    pub app_port: u16,
    #[validate(length(min = 1, message = "API_BASE_URL must not be empty"))]
    pub api_base_url: String,
}

fn main() {
    let task = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: cargo <dev|start|build>");
        std::process::exit(1);
    });

    match task.as_str() {
        "dev" => run_server("development"),
        "start" => run_static_server(),
        "build" => run_build(),
        _ => {
            eprintln!("Unknown task: {task}. Use dev | start | build");
            std::process::exit(1);
        }
    }
}

// Serves the production bundle with miniserve on APP_PORT from .env.production
fn run_static_server() {
    let env_file = ".env.production";

    dotenvy::from_filename(env_file)
        .unwrap_or_else(|_| panic!("Failed to load environment file: {}", env_file));

    let config: EnvSchema = envy::from_env().unwrap_or_else(|err| {
        panic!(
            "Failed to parse environment variables from {}: {}",
            env_file, err
        );
    });

    config.validate().unwrap_or_else(|err| {
        panic!("Environment validation failed for {}: {}", env_file, err);
    });

    if !Path::new(BUNDLE_DIR).join("index.html").exists() {
        eprintln!("No bundle found in {BUNDLE_DIR}. Run `cargo xtask build` first.");
        std::process::exit(1);
    }

    let port = config.app_port.to_string();
    println!("-> Loaded {} successfully", env_file);
    println!("-> Serving {} on 127.0.0.1:{}", BUNDLE_DIR, port);

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
        .unwrap_or_else(|err| {
            eprintln!("Could not run miniserve ({err}). Install it with: cargo install miniserve");
            std::process::exit(1);
        });

    std::process::exit(status.code().unwrap_or(1));
}

// Creates the optimized web bundle with the API address from .env.production
fn run_build() {
    let env_file = ".env.production";

    dotenvy::from_filename(env_file)
        .unwrap_or_else(|_| panic!("Failed to load environment file: {}", env_file));

    let config: BuildEnvSchema = envy::from_env().unwrap_or_else(|err| {
        panic!(
            "Failed to parse environment variables from {}: {}",
            env_file, err
        );
    });

    config.validate().unwrap_or_else(|err| {
        panic!("Environment validation failed for {}: {}", env_file, err);
    });

    let api_base_url = config.api_base_url.trim().trim_end_matches('/').to_string();

    if !(api_base_url.starts_with("http://") || api_base_url.starts_with("https://")) {
        eprintln!("API_BASE_URL in {env_file} must start with http:// or https://, got \"{api_base_url}\"");
        std::process::exit(1);
    }

    println!("-> Loaded {} successfully", env_file);
    println!("-> API_BASE_URL compiled into the bundle: {}", api_base_url);

    let status = Command::new("dx")
        .arg("bundle")
        .arg("--platform")
        .arg("web")
        .arg("--release")
        .env("API_BASE_URL", &api_base_url)
        .status()
        .expect("Failed to execute dx bundle");

    if status.success() {
        println!("-> Bundle finished, the output path is printed by dx above");
    }

    std::process::exit(status.code().unwrap_or(1));
}

fn run_server(profile: &str) {
    let env_file = format!(".env.{}", profile);

    dotenvy::from_filename(&env_file)
        .unwrap_or_else(|_| panic!("Failed to load environment file: {}", env_file));

    let config: EnvSchema = envy::from_env().unwrap_or_else(|err| {
        panic!(
            "Failed to parse environment variables from {}: {}",
            env_file, err
        );
    });

    config.validate().unwrap_or_else(|err| {
        panic!("Environment validation failed for {}: {}", env_file, err);
    });

    let port = config.app_port.to_string();
    println!("-> Loaded {} successfully (Port: {})", env_file, port);

    let status = Command::new("dx")
        .arg("serve")
        .arg("--port")
        .arg(port)
        .status()
        .expect("Failed to execute dx serve");

    std::process::exit(status.code().unwrap_or(1));
}
