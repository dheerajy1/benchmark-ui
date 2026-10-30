use serde::Deserialize;
use std::env;
use std::process::Command;
use validator::Validate;

#[derive(Clone, Debug, Deserialize, Validate)]
pub struct EnvSchema {
    #[validate(range(min = 1, message = "APP_PORT must be a valid port number"))]
    pub app_port: u16,
}

fn main() {
    let task = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: cargo <dev|start>");
        std::process::exit(1);
    });

    match task.as_str() {
        "dev" => run_server("development"),
        "start" => run_server("production"),
        _ => {
            eprintln!("Unknown task: {task}. Use dev | start");
            std::process::exit(1);
        }
    }
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
