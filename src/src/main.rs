use aether_cli::commands::{execute_command, Cli};
use clap::Parser;
use tracing_subscriber;

#[tokio::main]
async fn main() -> aether_cli::Result<()> {
    // Load environment variables from .env file in the project root
    // Try multiple possible locations for .env file
    let possible_env_paths = [
        ".env",                                      // Current directory
        "../.env",                                   // Parent directory
        "../../.env",                                // Grandparent directory
        "/home/secus/WorkSpace/aether-engine/.env",  // Project root path
        "/home/secus/Work-Space/Aether-Engine/.env", // Old absolute path
    ];

    for env_path in &possible_env_paths {
        if std::path::Path::new(env_path).exists() {
            if let Ok(_) = dotenvy::from_path(env_path) {
                eprintln!("🔧 Loaded environment from: {}", env_path);
                break;
            }
        }
    }

    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    if let Err(e) = execute_command(cli).await {
        aether_cli::utils::print_error(&format!("Error: {}", e));
        std::process::exit(1);
    }

    Ok(())
}
