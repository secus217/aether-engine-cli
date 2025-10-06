use crate::{
    api::{ApiClient, Application, CreateAppRequest},
    builder::ProjectBuilder,
    config::Config,
    presigned_uploader::PresignedUploader,
    terminal_dashboard, utils, Result,
};
use chrono;
use clap::{Parser, Subcommand};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::{io::Write, path::PathBuf};
use uuid::Uuid;

/// Safe password input with fallback for non-TTY environments
fn read_password_safe(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    std::io::stdout().flush().unwrap();

    // Try to read password securely first
    match rpassword::read_password() {
        Ok(password) => Ok(password),
        Err(_) => {
            // Fallback: read from stdin (not secure but works in non-TTY)
            utils::print_warning("Warning: Password input is not hidden in this environment");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            Ok(input.trim().to_string())
        }
    }
}

fn get_project_name_from_current_dir() -> Option<String> {
    let current_dir = std::env::current_dir().ok()?;
    let package_json_path = current_dir.join("package.json");

    if !package_json_path.exists() {
        return None;
    }

    match std::fs::read_to_string(&package_json_path) {
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(json) => json
                .get("name")
                .and_then(|name| name.as_str())
                .map(|s| s.to_string()),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

#[derive(Parser)]
#[command(name = "aether")]
#[command(about = "AetherEngine CLI - Fast NodeJS deployment platform")]
#[command(version = "1.2.1")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Register a new account
    Register {
        /// Email address
        #[arg(short, long)]
        email: Option<String>,
        /// Password (will be prompted if not provided)
        #[arg(short, long)]
        password: Option<String>,
        /// API endpoint URL
        #[arg(long)]
        endpoint: Option<String>,
    },
    /// Login to existing account
    Login {
        /// Email address
        #[arg(short, long)]
        email: Option<String>,
        /// Password (will be prompted if not provided)
        #[arg(short, long)]
        password: Option<String>,
        /// API endpoint URL
        #[arg(long)]
        endpoint: Option<String>,
    },
    /// Logout and clear authentication token
    Logout,
    /// Deploy application
    Deploy {
        /// Application name (auto-detected from package.json if not provided)
        #[arg(short, long)]
        name: Option<String>,
        /// Runtime version (auto-detected from package.json if not provided)
        #[arg(short, long)]
        runtime: Option<String>,
        /// Project path (current directory if not provided)
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// Force redeploy even if app exists
        #[arg(short, long)]
        force: bool,
    },
    /// List deployed applications
    List,
    /// View application logs
    Logs {
        /// Application name or UUID (auto-detected from package.json if not provided)
        app: Option<String>,
        /// Number of lines to show
        #[arg(short, long, default_value = "100")]
        lines: u32,
        /// Follow logs (not implemented yet)
        #[arg(short, long)]
        follow: bool,
    },
    /// Delete application
    Delete {
        /// Application name or UUID
        app: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
    /// Show application status
    Status {
        /// Application name or UUID
        app: String,
    },
    /// Interactive dashboard mode
    Dashboard,
    /// S3 operations
    S3 {
        #[command(subcommand)]
        action: S3Commands,
    },
    /// Custom domain management
    Domain {
        #[command(subcommand)]
        action: DomainCommands,
    },
}

#[derive(Subcommand)]
pub enum S3Commands {
    /// Upload a file to S3
    Upload {
        /// File path to upload
        file: PathBuf,
        /// App name for the upload
        app_name: String,
        /// Version for the upload
        version: String,
    },
}

#[derive(Subcommand)]
pub enum DomainCommands {
    /// Add a custom domain to an application
    Add {
        /// Application name or UUID
        app: String,
        /// Custom domain (e.g., myapp.example.com)
        domain: String,
    },
    /// List all custom domains for an application
    List {
        /// Application name or UUID
        app: String,
    },
    /// Delete a custom domain
    Delete {
        /// Application name or UUID
        app: String,
        /// Domain name to delete
        domain: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
}

pub async fn execute_command(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Register {
            email,
            password,
            endpoint,
        } => register_command(email, password, endpoint).await,
        Commands::Login {
            email,
            password,
            endpoint,
        } => login_command(email, password, endpoint).await,
        Commands::Logout => logout_command().await,
        Commands::Deploy {
            name,
            runtime,
            path,
            force,
        } => deploy_command(name, runtime, path, force).await,
        Commands::List => list_command().await,
        Commands::Logs { app, lines, follow } => logs_command(app, lines, follow).await,
        Commands::Delete { app, yes } => delete_command(app, yes).await,
        Commands::Status { app } => status_command(app).await,
        Commands::Dashboard => dashboard_command().await,
        Commands::S3 { action } => s3_command(action).await,
        Commands::Domain { action } => domain_command(action).await,
    }
}

async fn register_command(
    email: Option<String>,
    password: Option<String>,
    endpoint: Option<String>,
) -> Result<()> {
    let mut config = Config::load()?;

    // Update endpoint if provided
    if let Some(endpoint) = endpoint {
        config.api_endpoint = endpoint;
        config.save()?;
        utils::print_info(&format!("Updated API endpoint to: {}", config.api_endpoint));
    }

    // Get email from user if not provided
    let email = match email {
        Some(email) => email,
        None => {
            print!("Email: ");
            std::io::stdout().flush().unwrap();
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            input.trim().to_string()
        }
    };

    // Get password from user if not provided
    let password = match password {
        Some(password) => password,
        None => read_password_safe("Password (minimum 6 characters): ")?,
    };

    println!("🔐 {}", style("Registering new account...").bold());

    // Create API client and register
    let client = ApiClient::new(config.api_endpoint.clone(), None)?;

    match client.register(email.clone(), password).await {
        Ok(auth_response) => {
            // Save token to config
            config.set_auth_token(auth_response.token)?;

            utils::print_success("Account registered successfully!");
            println!("👤 User ID: {}", style(auth_response.user.id).cyan());
            println!("📧 Email: {}", style(&auth_response.user.email).cyan());
            utils::print_info("You are now logged in and ready to deploy!");
        }
        Err(e) => {
            utils::print_error(&format!("Registration failed: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

async fn login_command(
    email: Option<String>,
    password: Option<String>,
    endpoint: Option<String>,
) -> Result<()> {
    let mut config = Config::load()?;

    // Update endpoint if provided
    if let Some(endpoint) = endpoint {
        config.api_endpoint = endpoint;
        config.save()?;
        utils::print_info(&format!("Updated API endpoint to: {}", config.api_endpoint));
    }

    // Get email from user if not provided
    let email = match email {
        Some(email) => email,
        None => {
            print!("Email: ");
            std::io::stdout().flush().unwrap();
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            input.trim().to_string()
        }
    };

    // Get password from user if not provided
    let password = match password {
        Some(password) => password,
        None => read_password_safe("Password: ")?,
    };

    println!("🔐 {}", style("Logging in...").bold());

    // Create API client and login
    let client = ApiClient::new(config.api_endpoint.clone(), None)?;

    match client.login(email.clone(), password).await {
        Ok(auth_response) => {
            // Save token to config
            config.set_auth_token(auth_response.token)?;

            utils::print_success("Logged in successfully!");
            println!(
                "👤 Welcome back, {}",
                style(&auth_response.user.email).cyan()
            );
            utils::print_info("You are now authenticated and ready to deploy!");
        }
        Err(e) => {
            utils::print_error(&format!("Login failed: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

async fn logout_command() -> Result<()> {
    let mut config = Config::load()?;

    if !config.is_authenticated() {
        utils::print_info("You are not currently logged in");
        return Ok(());
    }

    // Clear the auth token
    config.clear_auth_token()?;

    utils::print_success("Successfully logged out!");
    utils::print_info("Use 'aether login' to authenticate again");

    Ok(())
}

async fn deploy_command(
    name: Option<String>,
    runtime: Option<String>,
    path: Option<PathBuf>,
    force: bool,
) -> Result<()> {
    let config = Config::load()?;

    // Check authentication first
    if !config.is_authenticated() {
        utils::print_error("❌ Authentication required to deploy applications");
        utils::print_info("Please login first:");
        utils::print_info("  aether login --email your@email.com");
        utils::print_info("Or register a new account:");
        utils::print_info("  aether register --email your@email.com");
        return Ok(());
    }

    let project_path = path.unwrap_or_else(|| std::env::current_dir().unwrap());

    // Find project root if we're in a subdirectory
    let project_root = utils::find_project_root(&project_path).unwrap_or(project_path);

    println!("🚀 {}", style("Starting deployment...").bold());
    println!("📁 Project path: {}", project_root.display());

    // Initialize project builder
    let builder = ProjectBuilder::new(&project_root)?;

    // Determine app name
    let app_name = if let Some(name) = name {
        utils::validate_app_name(&name)?;
        name
    } else {
        let detected_name = builder.get_app_name();
        utils::validate_app_name(detected_name)?;
        detected_name.to_string()
    };

    // Determine runtime
    let app_runtime = runtime.unwrap_or_else(|| builder.detect_runtime());

    println!("📦 App name: {}", style(&app_name).cyan());
    println!("🏷️  Version: {}", style(builder.get_version()).cyan());
    println!("🔧 Runtime: {}", style(&app_runtime).cyan());

    // Create API client
    let client = ApiClient::new(config.api_endpoint, config.auth_token)?;

    // Check if app already exists
    let existing_app = find_app_by_name(&client, &app_name).await?;

    let app = if let Some(existing_app) = existing_app {
        if !force {
            let should_continue = utils::confirm(&format!(
                "Application '{}' already exists. Continue with deployment?",
                app_name
            ))?;

            if !should_continue {
                utils::print_info("Deployment cancelled");
                return Ok(());
            }
        }
        existing_app
    } else {
        // Create new application
        println!("📝 Creating new application...");
        let create_request = CreateAppRequest {
            name: app_name.clone(),
            description: Some(format!("NodeJS application deployed via AetherEngine CLI")),
            runtime: app_runtime.clone(),
        };

        client.create_application(create_request).await?
    };

    // Build the application
    let artifact_path = builder.build(None).await?;

    // Get artifact size for display
    let artifact_size = {
        let metadata = std::fs::metadata(&artifact_path)?;
        utils::format_size(metadata.len())
    };

    println!("📤 Uploading artifact to S3 ({})...", artifact_size);

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Uploading to S3...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    // Upload to S3 using presigned URL
    let presigned_uploader = PresignedUploader::new(client.clone());
    let (artifact_url, presigned_url) = presigned_uploader
        .upload_artifact(&artifact_path, app.id, &builder.get_version())
        .await?;

    pb.set_message("Deploying application...");

    // Deploy the application with S3 URL (backend will generate presigned URL)
    let deployment = client
        .deploy_application(app.id, builder.get_version(), artifact_url.clone())
        .await?;

    pb.finish_and_clear();

    // Clean up temporary artifact
    std::fs::remove_file(&artifact_path)?;

    utils::print_success(&format!("Deployment completed successfully!"));
    println!("🆔 App ID: {}", style(app.id).dim());
    println!("🚀 Deployment ID: {}", style(deployment.id).dim());
    println!("📊 Status: {}", style(&deployment.status).green());
    println!("📦 Artifact: {}", style(&artifact_url).dim());
    println!("🔗 Download URL: {}", style(&presigned_url).blue());

    // Web Dashboard promotion
    println!();
    println!("╔═══════════════════════════════════════════════════════════════════════════╗");
    println!("║                    🌐  MANAGE YOUR APP ONLINE  🌐                        ║");
    println!("║                                                                           ║");
    println!("║  🎯 View, monitor and manage your deployed app at:                       ║");
    println!("║                                                                           ║");
    println!("║                    ➡️  https://aetherngine.com/  ⬅️                        ║");
    println!("║                                                                           ║");
    println!("║  ✨ Real-time monitoring, logs, metrics & deployment management!         ║");
    println!("╚═══════════════════════════════════════════════════════════════════════════╝");
    println!();

    // Show logs command hint
    utils::print_info(&format!("View logs with: aether logs {}", app_name));
    utils::print_info("Presigned URL valid for 24 hours");

    Ok(())
}

async fn list_command() -> Result<()> {
    let config = Config::load()?;

    // Check authentication first
    if !config.is_authenticated() {
        utils::print_error("❌ Authentication required to list applications");
        utils::print_info("Please login first: aether login --email your@email.com");
        return Ok(());
    }

    let client = ApiClient::new(config.api_endpoint, config.auth_token)?;

    println!("📋 {}", style("Fetching applications...").bold());

    let apps = client.list_applications().await?;

    if apps.is_empty() {
        utils::print_info("No applications found");
        utils::print_info("Deploy your first app with: aether deploy");
        return Ok(());
    }

    println!(
        "\n{:<20} {:<15} {:<20} {:<40}",
        "NAME", "RUNTIME", "CREATED", "DEPLOYMENT URL"
    );
    println!("{}", "─".repeat(100));

    for app in apps {
        let created = app.created_at.format("%Y-%m-%d %H:%M").to_string();
        let url_display = app
            .deployment_url
            .as_ref()
            .map(|url| style(url).green().to_string())
            .unwrap_or_else(|| style("Not deployed").dim().to_string());

        println!(
            "{:<20} {:<15} {:<20} {:<40}",
            style(&app.name).cyan(),
            app.runtime,
            created,
            url_display
        );
    }

    Ok(())
}

async fn logs_command(app: Option<String>, lines: u32, follow: bool) -> Result<()> {
    let config = Config::load()?;

    // Check authentication first
    if !config.is_authenticated() {
        utils::print_error("❌ Authentication required to view logs");
        utils::print_info("Please login first: aether login --email your@email.com");
        return Ok(());
    }

    let client = ApiClient::new(config.api_endpoint, config.auth_token)?;

    // Determine app name - either provided or auto-detected
    let app_name = if let Some(name) = app {
        name
    } else {
        // Try to auto-detect from package.json in current directory
        match get_project_name_from_current_dir() {
            Some(name) => {
                println!(
                    "📂 {}",
                    style(format!("Auto-detected project: {}", name)).dim()
                );
                name
            }
            None => {
                utils::print_error(
                    "No app name provided and no package.json found in current directory",
                );
                utils::print_info("Usage: aether logs <app_name> [options]");
                utils::print_info("Or run in a project directory with package.json");
                return Ok(());
            }
        }
    };

    println!(
        "📜 {}",
        style(format!("Fetching logs for '{}'...", app_name)).bold()
    );

    // Find application by name or UUID
    let app_id = resolve_app_identifier(&client, &app_name).await?;

    if follow {
        println!("🚀 {}", style("Starting real-time log streaming...").bold());
        println!("📡 {}", style("Press Ctrl+C to stop streaming").dim());
        println!();

        // Get initial logs
        let mut last_logs = client.get_logs(app_id, Some(lines)).await?;
        if !last_logs.trim().is_empty() {
            println!("{}", last_logs);
        }

        // Start streaming loop
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
        loop {
            interval.tick().await;

            match client.get_logs(app_id, Some(200)).await {
                Ok(current_logs) => {
                    if !current_logs.trim().is_empty() {
                        // Find new log lines by comparing last line numbers
                        let current_lines: Vec<&str> = current_logs.lines().collect();
                        let last_lines: Vec<&str> = last_logs.lines().collect();

                        // Get the last line from previous fetch to find new content
                        if !current_lines.is_empty() {
                            let mut new_content = false;

                            // If we have more lines now, or if the content is different
                            if current_lines.len() > last_lines.len() {
                                // Show new lines
                                for line in current_lines.iter().skip(last_lines.len()) {
                                    if !line.trim().is_empty() {
                                        let timestamp = chrono::Local::now().format("%H:%M:%S");
                                        println!(
                                            "🔴 [{}] {}",
                                            style(timestamp).green().bold(),
                                            line
                                        );
                                        new_content = true;
                                    }
                                }
                            } else if !last_lines.is_empty() && !current_lines.is_empty() {
                                // Check if the latest lines are different (content might have changed)
                                let last_line = last_lines.last().unwrap_or(&"");
                                let current_last_line = current_lines.last().unwrap_or(&"");

                                if last_line != current_last_line {
                                    // Show the latest few lines with timestamp
                                    let timestamp = chrono::Local::now().format("%H:%M:%S");
                                    println!(
                                        "🔄 [{}] Latest: {}",
                                        style(timestamp).yellow().bold(),
                                        current_last_line
                                    );
                                    new_content = true;
                                }
                            }

                            if new_content {
                                last_logs = current_logs;
                            }
                        }
                    }
                }
                Err(e) => {
                    utils::print_error(&format!("Error fetching logs: {}", e));
                    break;
                }
            }
        }
    } else {
        let logs = client.get_logs(app_id, Some(lines)).await?;

        if logs.trim().is_empty() {
            utils::print_info("No logs available");
            return Ok(());
        }

        println!("\n{}", logs);
    }

    Ok(())
}

async fn delete_command(app: String, yes: bool) -> Result<()> {
    let config = Config::load()?;

    // Check authentication first
    if !config.is_authenticated() {
        utils::print_error("❌ Authentication required to delete applications");
        utils::print_info("Please login first: aether login --email your@email.com");
        return Ok(());
    }

    let client = ApiClient::new(config.api_endpoint, config.auth_token)?;

    // Find application by name or UUID
    let app_id = resolve_app_identifier(&client, &app).await?;
    let app_details = client.get_application(app_id).await?;

    if !yes {
        let confirmed = utils::confirm(&format!(
            "Are you sure you want to delete application '{}'? This action cannot be undone.",
            app_details.name
        ))?;

        if !confirmed {
            utils::print_info("Deletion cancelled");
            return Ok(());
        }
    }

    println!("🗑️  Deleting application '{}'...", app_details.name);

    client.delete_application(app_id).await?;

    utils::print_success(&format!(
        "Application '{}' deleted successfully",
        app_details.name
    ));

    Ok(())
}

async fn status_command(app: String) -> Result<()> {
    let config = Config::load()?;

    // Check authentication first
    if !config.is_authenticated() {
        utils::print_error("❌ Authentication required to view application status");
        utils::print_info("Please login first: aether login --email your@email.com");
        return Ok(());
    }

    let client = ApiClient::new(config.api_endpoint, config.auth_token)?;

    // Find application by name or UUID
    let app_id = resolve_app_identifier(&client, &app).await?;
    let app_details = client.get_application(app_id).await?;

    println!(
        "📊 {}",
        style(format!("Status for '{}'", app_details.name)).bold()
    );
    println!();

    println!("🆔 ID: {}", app_details.id);
    println!("📦 Name: {}", style(&app_details.name).cyan());
    println!(
        "� Description: {}",
        app_details.description.as_deref().unwrap_or("N/A")
    );
    println!("� Runtime: {}", app_details.runtime);
    println!(
        "📅 Created: {}",
        app_details.created_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!(
        "🔄 Updated: {}",
        app_details.updated_at.format("%Y-%m-%d %H:%M:%S UTC")
    );

    // Get deployments
    println!("\n📚 Recent deployments:");
    let deployments = client.list_deployments(app_id).await?;

    if deployments.is_empty() {
        utils::print_info("No deployments found");
    } else {
        println!(
            "{:<8} {:<15} {:<20} {:<20}",
            "VERSION", "STATUS", "CREATED", "ARTIFACT"
        );
        println!("{}", "─".repeat(70));

        for deployment in deployments.iter().take(5) {
            let created = deployment.created_at.format("%Y-%m-%d %H:%M").to_string();
            let artifact = deployment.artifact_url.as_deref().unwrap_or("N/A");

            println!(
                "{:<8} {:<15} {:<20} {:<20}",
                deployment.version,
                style(&deployment.status).green(),
                created,
                artifact
            );
        }
    }

    Ok(())
}

// Helper function to find app by name
pub async fn find_app_by_name(client: &ApiClient, name: &str) -> Result<Option<Application>> {
    let apps = client.list_applications().await?;
    Ok(apps.into_iter().find(|app| app.name == name))
}

// Helper function to resolve app identifier (name or UUID)
async fn resolve_app_identifier(client: &ApiClient, identifier: &str) -> Result<Uuid> {
    // Try to parse as UUID first
    if let Ok(uuid) = Uuid::parse_str(identifier) {
        // Verify that the UUID exists
        match client.get_application(uuid).await {
            Ok(_) => return Ok(uuid),
            Err(_) => {
                // Fall through to name lookup
            }
        }
    }

    // Look up by name
    if let Some(app) = find_app_by_name(client, identifier).await? {
        Ok(app.id)
    } else {
        Err(crate::AetherError::invalid_project(format!(
            "Application '{}' not found",
            identifier
        )))
    }
}

async fn dashboard_command() -> Result<()> {
    let config = Config::load()?;

    // Check if user is authenticated
    if !config.is_authenticated() {
        utils::print_error("❌ Authentication required to access dashboard");
        utils::print_info("Please login first:");
        utils::print_info("  aether login --email your@email.com");
        utils::print_info("Or register a new account:");
        utils::print_info("  aether register --email your@email.com");
        return Ok(());
    }

    // Verify token is still valid by testing API connection
    let client = ApiClient::new(config.api_endpoint.clone(), config.auth_token.clone())?;
    match client.get_me().await {
        Ok(user) => {
            utils::print_success(&format!("✅ Authenticated as: {}", user.email));
            utils::print_info("Starting AetherEngine Dashboard...");
            utils::print_info("Use Tab/Shift+Tab to switch tabs, ↑↓ or j/k to navigate, 'r' to refresh, 'q' to quit");

            std::thread::sleep(std::time::Duration::from_secs(1)); // Give user time to read

            terminal_dashboard::run_terminal_dashboard().await?;

            utils::print_success("Dashboard closed");
        }
        Err(_) => {
            utils::print_error("❌ Authentication token expired or invalid");
            utils::print_info("Please login again:");
            utils::print_info("  aether login --email your@email.com");
        }
    }

    Ok(())
}

async fn s3_command(action: S3Commands) -> Result<()> {
    match action {
        S3Commands::Upload {
            file,
            app_name,
            version,
        } => s3_upload_command(file, app_name, version).await,
    }
}

async fn s3_upload_command(file: PathBuf, _app_name: String, version: String) -> Result<()> {
    if !file.exists() {
        return Err(crate::AetherError::invalid_project(format!(
            "File not found: {:?}",
            file
        )));
    }

    utils::print_info(&format!("Uploading {} to S3...", file.display()));

    // Generate a UUID for the app (since this is a standalone upload)
    let app_id = Uuid::new_v4();
    utils::print_info(&format!("Generated app ID: {}", app_id));

    // For standalone upload, we need to create API client
    let config = Config::load()?;
    let client = ApiClient::new(config.api_endpoint, config.auth_token)?;

    let presigned_uploader = PresignedUploader::new(client);
    let (artifact_url, presigned_url) = presigned_uploader
        .upload_artifact(&file, app_id, &version)
        .await?;

    utils::print_success(&format!("✅ Upload successful!"));
    utils::print_info(&format!("Artifact URL: {}", artifact_url));
    utils::print_info(&format!("Presigned URL: {}", presigned_url));

    Ok(())
}

async fn domain_command(action: DomainCommands) -> Result<()> {
    match action {
        DomainCommands::Add { app, domain } => domain_add_command(app, domain).await,
        DomainCommands::List { app } => domain_list_command(app).await,
        DomainCommands::Delete { app, domain, yes } => {
            domain_delete_command(app, domain, yes).await
        }
    }
}

async fn domain_add_command(app: String, domain: String) -> Result<()> {
    let config = Config::load()?;

    // Check authentication first
    if !config.is_authenticated() {
        utils::print_error("❌ Authentication required to add custom domains");
        utils::print_info("Please login first: aether login --email your@email.com");
        return Ok(());
    }

    let client = ApiClient::new(config.api_endpoint, config.auth_token)?;

    println!(
        "🌐 {}",
        style(format!("Adding custom domain '{}'...", domain)).bold()
    );

    // Find application by name or UUID
    let app_id = resolve_app_identifier(&client, &app).await?;
    let app_details = client.get_application(app_id).await?;

    // Add the custom domain
    match client.add_custom_domain(app_id, domain.clone()).await {
        Ok(domain_response) => {
            utils::print_success(&format!(
                "✅ Custom domain '{}' added successfully!",
                domain
            ));
            println!("🆔 Domain ID: {}", style(domain_response.id).dim());
            println!("📋 App: {}", style(&app_details.name).cyan());
            println!("🌐 Domain: {}", style(&domain_response.domain).green());
            println!(
                "✓ Verified: {}",
                if domain_response.verified {
                    style("Yes").green()
                } else {
                    style("No (pending)").yellow()
                }
            );
            println!();
            utils::print_info("📝 Next steps:");
            utils::print_info(&format!(
                "1. Point your DNS A record for {} to your cluster's IP",
                domain
            ));
            utils::print_info("2. Wait for DNS propagation (usually 5-60 minutes)");
            utils::print_info("3. Your app will be accessible at the custom domain");
        }
        Err(e) => {
            utils::print_error(&format!("Failed to add custom domain: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

async fn domain_list_command(app: String) -> Result<()> {
    let config = Config::load()?;

    // Check authentication first
    if !config.is_authenticated() {
        utils::print_error("❌ Authentication required to list custom domains");
        utils::print_info("Please login first: aether login --email your@email.com");
        return Ok(());
    }

    let client = ApiClient::new(config.api_endpoint, config.auth_token)?;

    // Find application by name or UUID
    let app_id = resolve_app_identifier(&client, &app).await?;
    let app_details = client.get_application(app_id).await?;

    println!(
        "🌐 {}",
        style(format!("Custom domains for '{}'", app_details.name)).bold()
    );

    let domains = client.list_custom_domains(app_id).await?;

    if domains.is_empty() {
        utils::print_info("No custom domains configured");
        utils::print_info(&format!(
            "Add one with: aether domain add {} <your-domain.com>",
            app
        ));
        return Ok(());
    }

    println!("\n{:<40} {:<15} {:<20}", "DOMAIN", "VERIFIED", "ADDED");
    println!("{}", "─".repeat(80));

    for domain in domains {
        let verified_status = if domain.verified {
            style("✓ Verified").green().to_string()
        } else {
            style("⏳ Pending").yellow().to_string()
        };

        let created = domain.created_at.format("%Y-%m-%d %H:%M").to_string();

        println!(
            "{:<40} {:<15} {:<20}",
            style(&domain.domain).cyan(),
            verified_status,
            created
        );
    }

    Ok(())
}

async fn domain_delete_command(app: String, domain: String, yes: bool) -> Result<()> {
    let config = Config::load()?;

    // Check authentication first
    if !config.is_authenticated() {
        utils::print_error("❌ Authentication required to delete custom domains");
        utils::print_info("Please login first: aether login --email your@email.com");
        return Ok(());
    }

    let client = ApiClient::new(config.api_endpoint, config.auth_token)?;

    // Find application by name or UUID
    let app_id = resolve_app_identifier(&client, &app).await?;
    let app_details = client.get_application(app_id).await?;

    // Find the domain by name
    let domains = client.list_custom_domains(app_id).await?;
    let domain_to_delete = domains.iter().find(|d| d.domain == domain);

    if domain_to_delete.is_none() {
        utils::print_error(&format!(
            "Domain '{}' not found for app '{}'",
            domain, app_details.name
        ));
        return Ok(());
    }

    let domain_id = domain_to_delete.unwrap().id;

    if !yes {
        let confirmed = utils::confirm(&format!(
            "Are you sure you want to delete domain '{}' from '{}'?",
            domain, app_details.name
        ))?;

        if !confirmed {
            utils::print_info("Deletion cancelled");
            return Ok(());
        }
    }

    println!("🗑️  Deleting custom domain '{}'...", domain);

    client.delete_custom_domain(app_id, domain_id).await?;

    utils::print_success(&format!("Custom domain '{}' deleted successfully", domain));

    Ok(())
}
