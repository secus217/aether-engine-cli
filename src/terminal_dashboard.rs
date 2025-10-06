use crate::pokemon_theme::{PokemonLoader, PokemonTheme, PokemonType};
use crate::pokemon_widgets::{BattleAnimation, PokemonNotification, PokemonStatus};
use crate::{api::ApiClient, builder::ProjectBuilder, config::Config, Result};

use tar::Builder as TarBuilder;

use chrono;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::Rng;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Widget,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame, Terminal,
};
use std::{
    io,
    process::{Command, Stdio},
    time::Duration,
};

pub struct TerminalApp {
    client: ApiClient,
    should_quit: bool,
    command_input: String,
    command_history: Vec<String>,
    history_index: Option<usize>,
    output_lines: Vec<String>,
    current_dir: std::path::PathBuf,
    cursor_position: usize,
    // Terminal scroll state
    terminal_scroll_offset: usize, // For scrolling through terminal output
    // File explorer state
    #[allow(dead_code)]
    show_file_explorer: bool,
    current_tab: usize, // 0: terminal, 1: file explorer, 2: apps, 3: auth, 4: domains
    file_tree: Vec<FileTreeItem>,
    selected_file_index: usize,
    // Tab completion
    completion_suggestions: Vec<String>,
    show_completions: bool,
    completion_index: usize,
    // Real-time log streaming
    is_streaming_logs: bool,
    streaming_app_id: Option<uuid::Uuid>,
    last_log_content: String,
    last_log_check: std::time::Instant,
    // Authentication state
    is_authenticated: bool,
    #[allow(dead_code)]
    current_user_email: Option<String>,
    // Applications state
    applications: Vec<crate::api::Application>,
    apps_last_fetched: std::time::Instant,
    selected_app_index: usize,
    pending_delete_app: Option<(uuid::Uuid, String)>,
    // Pokemon theme state
    pokemon_theme: PokemonTheme,
    pokemon_loader: PokemonLoader,
    animation_timer: std::time::Instant,
    show_notification: bool,
    current_notification: Option<PokemonNotification>,
    battle_animation: Option<BattleAnimation>,
    sparkle_positions: Vec<(u16, u16)>,
    // Output buffering for better log organization
    output_buffer: Vec<String>,
    is_command_running: bool,
}

#[derive(Clone)]
struct FileTreeItem {
    name: String,
    path: std::path::PathBuf,
    is_dir: bool,
    is_expanded: bool,
    depth: usize,
}

impl TerminalApp {
    pub fn new(client: ApiClient) -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

        // Check authentication status
        let config = crate::config::Config::load().unwrap_or_default();
        let is_authenticated = config.is_authenticated();
        let current_user_email = None; // Will be fetched async if authenticated

        let mut app = Self {
            client,
            should_quit: false,
            command_input: String::new(),
            command_history: Vec::new(),
            history_index: None,
            output_lines: Vec::new(),
            current_dir: current_dir.clone(),
            cursor_position: 0,
            terminal_scroll_offset: 0,
            show_file_explorer: false,
            current_tab: 0,
            file_tree: Vec::new(),
            selected_file_index: 0,
            completion_suggestions: Vec::new(),
            show_completions: false,
            completion_index: 0,
            is_streaming_logs: false,
            streaming_app_id: None,
            last_log_content: String::new(),
            last_log_check: std::time::Instant::now(),
            is_authenticated,
            current_user_email,
            applications: Vec::new(),
            apps_last_fetched: std::time::Instant::now(),
            selected_app_index: 0,
            pending_delete_app: None,
            pokemon_theme: PokemonTheme::new(PokemonType::Electric),
            pokemon_loader: PokemonLoader::new(PokemonType::Electric),
            animation_timer: std::time::Instant::now(),
            show_notification: false,
            current_notification: None,
            battle_animation: None,
            sparkle_positions: Vec::new(),
            output_buffer: Vec::new(),
            is_command_running: false,
        };

        // Build initial file tree
        app.rebuild_file_tree();

        let mut app = app;

        // Welcome message with beautiful Pokemon theme
        app.show_pokemon_welcome();

        // Generate initial sparkles
        app.generate_sparkles();

        app
    }

    fn show_pokemon_welcome(&mut self) {
        self.add_output_line("".to_string());
        self.add_output_line(
            "╔═══════════════════════════════════════════════════════════════════════════╗"
                .to_string(),
        );
        self.add_output_line(
            "║             🚀  WELCOME TO AETHER DASHBOARD  🚀                          ║"
                .to_string(),
        );
        self.add_output_line(
            "║            Platform as a Service - Deploy Anything Anywhere              ║"
                .to_string(),
        );
        self.add_output_line(
            "╚═══════════════════════════════════════════════════════════════════════════╝"
                .to_string(),
        );
        self.add_output_line("".to_string());

        // Status section
        if self.is_authenticated {
            self.add_output_line("✅ Status: AUTHENTICATED - Ready to deploy!".to_string());
        } else {
            self.add_output_line("⚠️  Status: NOT AUTHENTICATED".to_string());
            self.add_output_line("💡 Run 'aether login' to start using Aether".to_string());
        }

        self.add_output_line("".to_string());
        
        // Web Dashboard info
        self.add_output_line(
            "╔═══════════════════════════════════════════════════════════════════════════╗"
                .to_string(),
        );
        self.add_output_line(
            "║                    🌐  WEB DASHBOARD AVAILABLE  🌐                       ║"
                .to_string(),
        );
        self.add_output_line(
            "║                                                                           ║"
                .to_string(),
        );
        self.add_output_line(
            "║  🎯 Manage your deployments with our beautiful web interface:            ║"
                .to_string(),
        );
        self.add_output_line(
            "║                                                                           ║"
                .to_string(),
        );
        self.add_output_line(
            "║                    ➡️  https://aetherngine.com/  ⬅️                        ║"
                .to_string(),
        );
        self.add_output_line(
            "║                                                                           ║"
                .to_string(),
        );
        self.add_output_line(
            "║  ✨ Features: Visual app management, real-time monitoring, logs & more!  ║"
                .to_string(),
        );
        self.add_output_line(
            "╚═══════════════════════════════════════════════════════════════════════════╝"
                .to_string(),
        );
        self.add_output_line("".to_string());
        self.add_output_line(
            "╔═══════════════════════════════════════════════════════════════════════════╗"
                .to_string(),
        );
        self.add_output_line(
            "║                        📖  COMMAND REFERENCE  �                         ║"
                .to_string(),
        );
        self.add_output_line(
            "╚═══════════════════════════════════════════════════════════════════════════╝"
                .to_string(),
        );
        self.add_output_line("".to_string());

        self.add_output_line("🔐 AUTHENTICATION:".to_string());
        self.add_output_line(
            "   aether register <email> <password>  - Create new account".to_string(),
        );
        self.add_output_line(
            "   aether login <email> <password>     - Login to your account".to_string(),
        );
        self.add_output_line(
            "   aether logout                       - Logout from account".to_string(),
        );
        self.add_output_line("".to_string());

        self.add_output_line("🚀 APPLICATION MANAGEMENT:".to_string());
        self.add_output_line(
            "   aether deploy                       - Deploy current project".to_string(),
        );
        self.add_output_line(
            "   aether apps                         - List all applications".to_string(),
        );
        self.add_output_line(
            "   aether delete <app-name>            - Delete an application".to_string(),
        );
        self.add_output_line(
            "   aether logs <app-name>              - View application logs".to_string(),
        );
        self.add_output_line(
            "   aether deploy --name <name>         - Deploy with custom name".to_string(),
        );
        self.add_output_line(
            "   aether deploy --runtime <runtime>   - Specify runtime (nodejs, python)".to_string(),
        );
        self.add_output_line(
            "   aether deploy --env KEY=VALUE       - Set environment variables".to_string(),
        );
        self.add_output_line(
            "   aether deploy --port <port>         - Specify custom port".to_string(),
        );
        self.add_output_line(
            "   aether restart <app-name>           - Restart an application".to_string(),
        );
        self.add_output_line("".to_string());

        self.add_output_line("🌐 CUSTOM DOMAINS:".to_string());
        self.add_output_line(
            "   aether domain list <app-name>             - List domains for app".to_string(),
        );
        self.add_output_line(
            "   aether domain add <app-name> <domain>     - Add custom domain".to_string(),
        );
        self.add_output_line(
            "   aether domain delete <app-name> <domain>  - Remove domain".to_string(),
        );
        self.add_output_line(
            "   aether domain verify <app-name> <domain>  - Verify domain setup".to_string(),
        );
        self.add_output_line("".to_string());

        self.add_output_line("💡 OTHER:".to_string());
        self.add_output_line("   help              - Show this help message".to_string());
        self.add_output_line("   clear             - Clear terminal output".to_string());
        self.add_output_line("   pwd               - Print current directory".to_string());
        self.add_output_line("   ls                - List files in directory".to_string());
        self.add_output_line("   cd <directory>    - Change directory".to_string());
        self.add_output_line("".to_string());

        self.add_output_line("⌨️  KEYBOARD SHORTCUTS:".to_string());
        self.add_output_line("   Tab               - Cycle through tabs".to_string());
        self.add_output_line("   ↑↓                - Navigate history / lists".to_string());
        self.add_output_line("   Ctrl+C            - Stop current operation".to_string());
        self.add_output_line("   Ctrl+D            - Exit dashboard".to_string());
        self.add_output_line("".to_string());

        self.add_output_line("📌 TABS:".to_string());
        self.add_output_line("   Tab 1: 🎮 Terminal    - Execute commands".to_string());
        self.add_output_line("   Tab 2: 📁 Files       - Browse project files".to_string());
        self.add_output_line("   Tab 3: 🚀 Apps        - View & manage deployments".to_string());
        self.add_output_line("   Tab 4: � Auth        - Authentication status".to_string());
        self.add_output_line("".to_string());

        self.add_output_line(
            "╔═══════════════════════════════════════════════════════════════════════════╗"
                .to_string(),
        );
        self.add_output_line(
            "║  💡 TIP: Type 'help' anytime to see this guide again                     ║"
                .to_string(),
        );
        self.add_output_line(
            "╚═══════════════════════════════════════════════════════════════════════════╝"
                .to_string(),
        );
        self.add_output_line("".to_string());

        if self.is_authenticated {
            self.add_output_line(
                "✨ You're all set! Start by typing 'aether deploy' to deploy your app."
                    .to_string(),
            );
        } else {
            self.add_output_line("⚡ Get started: aether register <email> <password>".to_string());
        }
        self.add_output_line("".to_string());
    }

    fn generate_sparkles(&mut self) {
        let mut rng = rand::thread_rng();
        self.sparkle_positions.clear();

        // Generate random sparkle positions
        for _ in 0..10 {
            let x = rng.gen_range(0..100);
            let y = rng.gen_range(0..50);
            self.sparkle_positions.push((x, y));
        }
    }

    fn cycle_pokemon_theme(&mut self) {
        self.pokemon_theme.cycle_type();
        self.pokemon_loader = PokemonLoader::new(self.pokemon_theme.current_type);

        // Show notification about theme change
        let type_name = match self.pokemon_theme.current_type {
            PokemonType::Electric => "Electric ⚡",
            PokemonType::Fire => "Fire 🔥",
            PokemonType::Water => "Water 💧",
            PokemonType::Grass => "Grass 🌿",
            PokemonType::Psychic => "Psychic 🔮",
            PokemonType::Dragon => "Dragon 🐉",
            PokemonType::Ghost => "Ghost 👻",
            PokemonType::Normal => "Normal ⭐",
            PokemonType::Ice => "Ice ❄️",
            PokemonType::Dark => "Dark 🌙",
        };

        self.current_notification = Some(PokemonNotification::info(format!(
            "Theme switched to {} type!",
            type_name
        )));
        self.show_notification = true;
    }

    fn show_battle_animation(&mut self, command: &str) {
        // Create battle animation based on command
        let (move_name, pokemon_type) = match command {
            cmd if cmd.contains("build") => ("Code Compilation", PokemonType::Electric),
            cmd if cmd.contains("deploy") => ("Deploy Attack", PokemonType::Fire),
            cmd if cmd.contains("test") => ("Debug Scan", PokemonType::Psychic),
            cmd if cmd.contains("run") => ("Execute Rush", PokemonType::Normal),
            cmd if cmd.contains("install") => ("Package Summon", PokemonType::Grass),
            _ => ("Terminal Strike", PokemonType::Electric),
        };

        self.battle_animation = Some(BattleAnimation::new(
            "Aether Trainer",
            "Wild Bug",
            move_name,
            pokemon_type,
        ));
    }

    fn add_output_line(&mut self, line: String) {
        if self.is_command_running {
            // Buffer output during command execution
            self.output_buffer.push(line);
        } else {
            // Add directly if no command is running
            self.output_lines.push(line);
            // Reset scroll offset when new output is added (auto-scroll to bottom)
            self.terminal_scroll_offset = 0;
            // Keep only last 1000 lines to avoid memory issues
            if self.output_lines.len() > 1000 {
                self.output_lines.drain(0..100);
            }
        }
    }

    fn flush_output_buffer(&mut self) {
        if !self.output_buffer.is_empty() {
            // Sort buffer by content to maintain logical order
            // Add all buffered lines to output
            for line in self.output_buffer.drain(..) {
                self.output_lines.push(line);
            }
            // Reset scroll offset when flushing (auto-scroll to bottom)
            self.terminal_scroll_offset = 0;
            // Keep only last 1000 lines to avoid memory issues
            if self.output_lines.len() > 1000 {
                self.output_lines.drain(0..100);
            }
        }
    }

    fn start_command(&mut self) {
        self.is_command_running = true;
        self.output_buffer.clear();
    }

    fn end_command(&mut self) {
        self.flush_output_buffer();
        self.is_command_running = false;
    }

    fn get_project_name_from_current_dir(&self) -> Option<String> {
        let package_json_path = self.current_dir.join("package.json");

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

    fn rebuild_file_tree(&mut self) {
        self.file_tree.clear();
        self.build_file_tree(&self.current_dir.clone(), 0);
        self.selected_file_index = 0;
    }

    fn build_file_tree(&mut self, path: &std::path::Path, depth: usize) {
        if let Ok(entries) = std::fs::read_dir(path) {
            let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            entries.sort_by(|a, b| {
                let a_is_dir = a.file_type().map_or(false, |ft| ft.is_dir());
                let b_is_dir = b.file_type().map_or(false, |ft| ft.is_dir());
                match (a_is_dir, b_is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.file_name().cmp(&b.file_name()),
                }
            });

            for entry in entries.into_iter().take(50) {
                // Limit for performance
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.starts_with('.') && depth == 0 {
                    continue; // Skip hidden files at root
                }

                let is_dir = entry.file_type().map_or(false, |ft| ft.is_dir());
                self.file_tree.push(FileTreeItem {
                    name: file_name,
                    path: entry.path(),
                    is_dir,
                    is_expanded: false,
                    depth,
                });
            }
        }
    }

    fn expand_directory(&mut self, index: usize) {
        if index < self.file_tree.len() && self.file_tree[index].is_dir {
            let item = &mut self.file_tree[index];
            if !item.is_expanded {
                item.is_expanded = true;
                let path = item.path.clone();
                let depth = item.depth + 1;

                // Build subdirectory entries
                let mut new_items = Vec::new();
                if let Ok(entries) = std::fs::read_dir(&path) {
                    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
                    entries.sort_by(|a, b| {
                        let a_is_dir = a.file_type().map_or(false, |ft| ft.is_dir());
                        let b_is_dir = b.file_type().map_or(false, |ft| ft.is_dir());
                        match (a_is_dir, b_is_dir) {
                            (true, false) => std::cmp::Ordering::Less,
                            (false, true) => std::cmp::Ordering::Greater,
                            _ => a.file_name().cmp(&b.file_name()),
                        }
                    });

                    for entry in entries.into_iter().take(20) {
                        // Limit subdirs
                        let file_name = entry.file_name().to_string_lossy().to_string();
                        let is_dir = entry.file_type().map_or(false, |ft| ft.is_dir());
                        new_items.push(FileTreeItem {
                            name: file_name,
                            path: entry.path(),
                            is_dir,
                            is_expanded: false,
                            depth,
                        });
                    }
                }

                // Insert new items after current item
                for (i, item) in new_items.into_iter().enumerate() {
                    self.file_tree.insert(index + 1 + i, item);
                }
            } else {
                // Collapse: remove all deeper items
                item.is_expanded = false;
                let i = index + 1;
                while i < self.file_tree.len()
                    && self.file_tree[i].depth > self.file_tree[index].depth
                {
                    self.file_tree.remove(i);
                }
            }
        }
    }

    fn generate_completions(&mut self) {
        self.completion_suggestions.clear();
        self.show_completions = false;

        let words: Vec<&str> = self.command_input.split_whitespace().collect();

        if words.is_empty() {
            return;
        }

        match words[0] {
            "cd" => {
                let partial = if words.len() > 1 {
                    words[1].to_string()
                } else {
                    String::new()
                };

                self.get_directory_completions(&partial);
            }
            "aether" => {
                if words.len() == 2 && words[1] == "logs" {
                    // Get app names for logs completion
                    self.get_app_completions();
                } else if words.len() == 1 || (words.len() == 2 && !words[1].is_empty()) {
                    // Aether subcommands
                    let aether_commands = vec!["deploy", "apps", "logs", "dashboard"];
                    let partial = if words.len() > 1 { words[1] } else { "" };

                    for cmd in aether_commands {
                        if cmd.starts_with(partial) {
                            self.completion_suggestions.push(cmd.to_string());
                        }
                    }
                }
            }
            _ => {
                // Command completions
                let common_commands = vec!["ls", "ll", "pwd", "clear", "help", "cd", "aether"];
                for cmd in common_commands {
                    if cmd.starts_with(words[0]) {
                        self.completion_suggestions.push(cmd.to_string());
                    }
                }
            }
        }

        if !self.completion_suggestions.is_empty() {
            self.show_completions = true;
            self.completion_index = 0;
        }
    }

    fn get_directory_completions(&mut self, partial: &str) {
        let search_dir = if partial.starts_with('/') {
            // Absolute path
            std::path::PathBuf::from(
                partial
                    .rsplit('/')
                    .skip(1)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<String>(),
            )
        } else if partial.contains('/') {
            // Relative path with subdirectory
            let parent = partial
                .rsplit('/')
                .skip(1)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<String>();
            self.current_dir.join(parent)
        } else {
            // Current directory
            self.current_dir.clone()
        };

        let filename_partial = partial.split('/').last().unwrap_or("");

        if let Ok(entries) = std::fs::read_dir(&search_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.starts_with(filename_partial) && !name.starts_with('.') {
                            let full_path = if partial.contains('/') {
                                let prefix = &partial[..partial.rfind('/').unwrap() + 1];
                                format!("{}{}", prefix, name)
                            } else {
                                name
                            };
                            self.completion_suggestions.push(full_path);
                        }
                    }
                }
            }
        }
    }

    fn get_app_completions(&mut self) {
        // This would be async in real implementation, for now just add placeholder
        self.completion_suggestions.push("hello-aether".to_string());
    }

    fn apply_completion(&mut self) {
        if !self.show_completions || self.completion_suggestions.is_empty() {
            return;
        }

        let completion = &self.completion_suggestions[self.completion_index];
        let words: Vec<&str> = self.command_input.split_whitespace().collect();

        if words.is_empty() {
            return;
        }

        match words[0] {
            "cd" => {
                if words.len() == 1 {
                    self.command_input = format!("cd {}", completion);
                } else {
                    self.command_input = format!("cd {}", completion);
                }
            }
            "aether" => {
                if words.len() == 1 {
                    self.command_input = format!("aether {}", completion);
                } else if words.len() == 2 {
                    self.command_input = format!("aether {}", completion);
                } else if words.len() == 3 && words[1] == "logs" {
                    self.command_input = format!("aether logs {}", completion);
                }
            }
            _ => {
                self.command_input = completion.clone();
            }
        }

        self.cursor_position = self.command_input.len();
        self.show_completions = false;
    }

    async fn execute_command(&mut self, command: String) -> Result<()> {
        if command.trim().is_empty() {
            return Ok(());
        }

        // Start command execution - enable buffering
        self.start_command();

        // Refresh authentication status before executing command
        let config = crate::config::Config::load().unwrap_or_default();
        let was_authenticated = self.is_authenticated;
        self.is_authenticated = config.is_authenticated();

        // Update client with fresh token if available
        if let Some(token) = config.auth_token {
            self.client = ApiClient::new(config.api_endpoint, Some(token))?;
        }

        // If authentication status changed to authenticated, refresh applications
        if !was_authenticated && self.is_authenticated {
            self.apps_last_fetched = std::time::Instant::now() - std::time::Duration::from_secs(10);
        }

        // Add to history
        if !self.command_history.contains(&command) {
            self.command_history.push(command.clone());
        }

        // Show battle animation for the command
        self.show_battle_animation(&command);

        // Show the command being executed with Pokemon theme
        let sparkle = PokemonTheme::get_random_sparkle();
        let prompt = format!(
            "{}⚔️ {}$ {} {}",
            sparkle,
            self.current_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("~"),
            command,
            sparkle
        );
        // This goes directly to output (not buffered)
        self.is_command_running = false;
        self.add_output_line(prompt);
        self.is_command_running = true;

        let parts: Vec<&str> = command.trim().split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        match parts[0] {
            "help" => {
                self.show_help();
            }
            "aether" => {
                self.execute_aether_command(&parts[1..]).await?;
            }
            "cd" => {
                self.change_directory(&parts[1..]);
            }
            "ls" | "ll" | "dir" => {
                self.list_directory();
            }
            "pwd" => {
                self.add_output_line(format!("📂 {}", self.current_dir.display()));
            }
            "clear" | "cls" => {
                self.output_lines.clear();
                // Re-add wolf welcome
                self.add_output_line("".to_string());
                self.add_output_line("🐺 Terminal cleared! Wolf is still here! 🐺".to_string());
                self.add_output_line("".to_string());
            }
            "exit" | "quit" => {
                self.should_quit = true;
            }
            _ => {
                // Execute as shell command
                self.execute_shell_command(&command).await;
            }
        }

        // End command execution - flush buffer
        self.end_command();
        Ok(())
    }

    fn show_help(&mut self) {
        self.add_output_line("".to_string());
        self.add_output_line(
            "╔═══════════════════════════════════════════════════════════════════════════╗"
                .to_string(),
        );
        self.add_output_line(
            "║                        📖  COMMAND REFERENCE  📖                         ║"
                .to_string(),
        );
        self.add_output_line(
            "╚═══════════════════════════════════════════════════════════════════════════╝"
                .to_string(),
        );
        self.add_output_line("".to_string());

        self.add_output_line("🔐 AUTHENTICATION:".to_string());
        self.add_output_line(
            "   aether register <email> <password>  - Create new account".to_string(),
        );
        self.add_output_line(
            "   aether login <email> <password>     - Login to your account".to_string(),
        );
        self.add_output_line(
            "   aether logout                       - Logout from account".to_string(),
        );
        self.add_output_line("".to_string());

        self.add_output_line("🚀 DEPLOYMENT & APP MANAGEMENT:".to_string());
        self.add_output_line(
            "   aether deploy --name <name>         - Deploy with custom name".to_string(),
        );
        self.add_output_line(
            "   aether deploy --runtime <runtime>   - Specify runtime (nodejs, python)".to_string(),
        );
        self.add_output_line(
            "   aether deploy --env KEY=VALUE       - Set environment variables".to_string(),
        );
        self.add_output_line(
            "   aether deploy --port <port>         - Specify custom port".to_string(),
        );
        self.add_output_line(
            "   aether deploy                       - Deploy current project".to_string(),
        );
        self.add_output_line(
            "   aether apps                         - List all applications".to_string(),
        );
        self.add_output_line(
            "   aether delete <app-name>            - Delete an application".to_string(),
        );
        self.add_output_line(
            "   aether logs <app-name>              - View application logs".to_string(),
        );
        self.add_output_line(
            "   aether restart <app-name>           - Restart an application".to_string(),
        );
        self.add_output_line("".to_string());

        self.add_output_line("🌐 CUSTOM DOMAINS:".to_string());
        self.add_output_line(
            "   aether domain list <app-name>             - List domains for app".to_string(),
        );
        self.add_output_line(
            "   aether domain add <app-name> <domain>     - Add custom domain".to_string(),
        );
        self.add_output_line(
            "   aether domain delete <app-name> <domain>  - Remove domain".to_string(),
        );
        self.add_output_line(
            "   aether domain verify <app-name> <domain>  - Verify domain setup".to_string(),
        );
        self.add_output_line("".to_string());

        self.add_output_line("💡 OTHER:".to_string());
        self.add_output_line("   help              - Show this help message".to_string());
        self.add_output_line("   clear             - Clear terminal output".to_string());
        self.add_output_line("   pwd               - Print current directory".to_string());
        self.add_output_line("   ls                - List files in directory".to_string());
        self.add_output_line("   cd <directory>    - Change directory".to_string());
        self.add_output_line("".to_string());

        self.add_output_line("⌨️  KEYBOARD SHORTCUTS:".to_string());
        self.add_output_line("   Tab               - Cycle through tabs".to_string());
        self.add_output_line("   ↑↓                - Navigate history / lists".to_string());
        self.add_output_line("   Ctrl+C            - Stop current operation".to_string());
        self.add_output_line("   Ctrl+D            - Exit dashboard".to_string());
        self.add_output_line("".to_string());

        self.add_output_line("📌 TABS:".to_string());
        self.add_output_line("   Tab 1: 🎮 Terminal    - Execute commands".to_string());
        self.add_output_line("   Tab 2: 📁 Files       - Browse project files".to_string());
        self.add_output_line("   Tab 3: 🚀 Apps        - View & manage deployments".to_string());
        self.add_output_line("   Tab 4: � Auth        - Authentication status".to_string());
        self.add_output_line("".to_string());
        
        // Web Dashboard promotion in help
        self.add_output_line(
            "╔═══════════════════════════════════════════════════════════════════════════╗"
                .to_string(),
        );
        self.add_output_line(
            "║                    🌐  WEB DASHBOARD AVAILABLE  🌐                       ║"
                .to_string(),
        );
        self.add_output_line(
            "║                                                                           ║"
                .to_string(),
        );
        self.add_output_line(
            "║  🎯 For visual app management, visit our web dashboard:                  ║"
                .to_string(),
        );
        self.add_output_line(
            "║                                                                           ║"
                .to_string(),
        );
        self.add_output_line(
            "║                    ➡️  https://aetherngine.com/  ⬅️                        ║"
                .to_string(),
        );
        self.add_output_line(
            "║                                                                           ║"
                .to_string(),
        );
        self.add_output_line(
            "║  ✨ Features: GUI management, real-time monitoring & more!               ║"
                .to_string(),
        );
        self.add_output_line(
            "╚═══════════════════════════════════════════════════════════════════════════╝"
                .to_string(),
        );
        self.add_output_line("".to_string());
    }

    async fn execute_aether_command(&mut self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            self.add_output_line("Usage: aether <command>".to_string());
            return Ok(());
        }

        match args[0] {
            "register" | "login" | "logout" => {
                self.add_output_line(
                    "💡 Authentication commands should be run outside the dashboard".to_string(),
                );
                self.add_output_line("   Exit the dashboard (Ctrl+C) and run:".to_string());
                self.add_output_line(format!("   aether {}", args[0]));
            }
            "apps" | "list" => {
                self.add_output_line("📋 Fetching applications...".to_string());
                match self.client.list_applications().await {
                    Ok(apps) => {
                        if apps.is_empty() {
                            self.add_output_line("📭 No applications found".to_string());
                        } else {
                            self.add_output_line("".to_string());
                            self.add_output_line("🚀 Applications:".to_string());
                            for app in apps {
                                // For now, just show running status - we can add status later
                                self.add_output_line(format!("  🚀 {} ({})", app.name, app.id));
                            }
                        }
                    }
                    Err(e) => {
                        self.add_output_line(format!("❌ Error: {}", e));
                    }
                }
            }
            "deploy" => {
                // Check authentication first
                if !self.is_authenticated {
                    self.add_output_line(
                        "❌ Authentication required! Please login first:".to_string(),
                    );
                    self.add_output_line("   Run: aether login".to_string());
                    return Ok(());
                }

                self.add_output_line("🚀 Starting deployment...".to_string());

                // Use built-in deploy functionality instead of external command
                match self.deploy_current_project().await {
                    Ok(_) => {
                        self.add_output_line("✅ Deployment completed successfully!".to_string());
                    }
                    Err(e) => {
                        self.add_output_line(format!("❌ Deployment failed: {}", e));
                    }
                }
            }
            "logs" => {
                let (app_name, follow) = if args.len() == 1
                    || (args.len() == 2 && (args[1] == "--follow" || args[1] == "-f"))
                {
                    // No app name provided, or only --follow flag
                    if let Some(project_name) = self.get_project_name_from_current_dir() {
                        let follow = args.len() == 2 && (args[1] == "--follow" || args[1] == "-f");
                        self.add_output_line(format!("📂 Auto-detected project: {}", project_name));
                        (project_name, follow)
                    } else {
                        self.add_output_line(
                            "Usage: aether logs <app_name> [--follow/-f]".to_string(),
                        );
                        self.add_output_line(
                            "💡 Or run in a project directory with package.json to auto-detect"
                                .to_string(),
                        );
                        return Ok(());
                    }
                } else {
                    // App name provided
                    let app_name = args[1].to_string();
                    let follow = args.len() > 2 && (args[2] == "--follow" || args[2] == "-f");
                    (app_name, follow)
                };

                if follow {
                    self.add_output_line("🚀 Real-time log streaming requested...".to_string());
                    self.add_output_line(
                        "📡 Starting streaming mode... Press 'Esc' to stop".to_string(),
                    );
                    self.add_output_line("".to_string());
                }

                self.add_output_line(format!("📜 Fetching logs for '{}'...", app_name));

                // Find app by name first
                match self.client.list_applications().await {
                    Ok(apps) => {
                        if let Some(app) = apps.iter().find(|a| a.name == app_name) {
                            if follow {
                                // Enable streaming mode
                                self.is_streaming_logs = true;
                                self.streaming_app_id = Some(app.id);
                                self.add_output_line(
                                    "🚀 Starting REAL-TIME log streaming...".to_string(),
                                );
                                self.add_output_line(
                                    "📡 Connected! Press 'Esc' to stop streaming.".to_string(),
                                );
                                self.add_output_line("".to_string());

                                // Show initial logs
                                match self.client.get_logs(app.id, Some(20)).await {
                                    Ok(logs) => {
                                        if !logs.trim().is_empty() {
                                            for line in logs.lines().take(20) {
                                                self.add_output_line(line.to_string());
                                            }
                                        }
                                        self.last_log_content = logs;
                                    }
                                    Err(e) => {
                                        self.add_output_line(format!(
                                            "❌ Error fetching initial logs: {}",
                                            e
                                        ));
                                    }
                                }
                            } else if false {
                                self.add_output_line(
                                    "🚀 Starting REAL-TIME log streaming...".to_string(),
                                );
                                self.add_output_line(
                                    "📡 Connected! Press 'Esc' or 'Ctrl+C' to stop streaming."
                                        .to_string(),
                                );
                                self.add_output_line("".to_string());

                                self.is_streaming_logs = true;
                                self.streaming_app_id = Some(app.id);

                                // Start continuous streaming
                                let app_id = app.id;
                                let mut last_log_content = String::new();

                                // Stream until stopped by user
                                while self.is_streaming_logs {
                                    match self.client.get_logs(app_id, Some(500)).await {
                                        Ok(logs) => {
                                            if !logs.trim().is_empty() && logs != last_log_content {
                                                // Find new log lines
                                                let old_lines: Vec<&str> =
                                                    last_log_content.lines().collect();
                                                let new_lines: Vec<&str> = logs.lines().collect();

                                                // Show only new lines
                                                if new_lines.len() > old_lines.len() {
                                                    for line in
                                                        new_lines.iter().skip(old_lines.len())
                                                    {
                                                        if !line.trim().is_empty() {
                                                            self.add_output_line(format!(
                                                                "📄 {}",
                                                                line
                                                            ));
                                                        }
                                                    }
                                                }
                                                last_log_content = logs;
                                            }
                                        }
                                        Err(e) => {
                                            self.add_output_line(format!(
                                                "❌ Error streaming logs: {}",
                                                e
                                            ));
                                            self.is_streaming_logs = false;
                                            break;
                                        }
                                    }

                                    // Short polling interval for responsiveness
                                    tokio::time::sleep(tokio::time::Duration::from_millis(1000))
                                        .await;
                                }

                                self.streaming_app_id = None;
                                self.add_output_line("".to_string());
                                self.add_output_line("� Log streaming stopped.".to_string());
                            } else {
                                // Regular logs fetch
                                match self.client.get_logs(app.id, Some(50)).await {
                                    Ok(logs) => {
                                        if logs.trim().is_empty() {
                                            self.add_output_line(
                                                "📭 No logs available".to_string(),
                                            );
                                        } else {
                                            self.add_output_line("".to_string());
                                            for line in logs.lines().take(20) {
                                                self.add_output_line(line.to_string());
                                            }
                                            if logs.lines().count() > 20 {
                                                self.add_output_line(
                                                    "... (showing first 20 lines)".to_string(),
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        self.add_output_line(format!(
                                            "❌ Error fetching logs: {}",
                                            e
                                        ));
                                    }
                                }
                            }
                        } else {
                            self.add_output_line(format!(
                                "❌ Application '{}' not found",
                                app_name
                            ));
                        }
                    }
                    Err(e) => {
                        self.add_output_line(format!("❌ Error listing applications: {}", e));
                    }
                }
            }
            "domain" => {
                if args.len() < 2 {
                    self.add_output_line("Usage: aether domain <action> [options]".to_string());
                    self.add_output_line("Actions:".to_string());
                    self.add_output_line("  add <app> <domain>   - Add custom domain".to_string());
                    self.add_output_line("  list <app>           - List domains".to_string());
                    self.add_output_line("  delete <app> <domain> - Delete domain".to_string());
                    self.add_output_line("".to_string());
                    self.add_output_line(
                        "💡 Or use the Domains tab (Tab key to switch)".to_string(),
                    );
                    return Ok(());
                }

                match args[1] {
                    "add" => {
                        if args.len() < 4 {
                            self.add_output_line(
                                "Usage: aether domain add <app> <domain>".to_string(),
                            );
                            return Ok(());
                        }
                        let app_name = args[2];
                        let domain = args[3];

                        self.add_output_line(format!(
                            "🌐 Adding domain '{}' to '{}'...",
                            domain, app_name
                        ));

                        // Find app by name
                        match self.client.list_applications().await {
                            Ok(apps) => {
                                if let Some(app) = apps.iter().find(|a| a.name == app_name) {
                                    match self
                                        .client
                                        .add_custom_domain(app.id, domain.to_string())
                                        .await
                                    {
                                        Ok(_) => {
                                            self.add_output_line(format!(
                                                "✅ Domain '{}' added successfully!",
                                                domain
                                            ));
                                            self.add_output_line(
                                                "💡 Point your DNS A record to the cluster IP"
                                                    .to_string(),
                                            );
                                        }
                                        Err(e) => {
                                            self.add_output_line(format!(
                                                "❌ Failed to add domain: {}",
                                                e
                                            ));
                                        }
                                    }
                                } else {
                                    self.add_output_line(format!(
                                        "❌ Application '{}' not found",
                                        app_name
                                    ));
                                }
                            }
                            Err(e) => {
                                self.add_output_line(format!(
                                    "❌ Error listing applications: {}",
                                    e
                                ));
                            }
                        }
                    }
                    "list" => {
                        if args.len() < 3 {
                            self.add_output_line("Usage: aether domain list <app>".to_string());
                            return Ok(());
                        }
                        let app_name = args[2];

                        self.add_output_line(format!("🌐 Fetching domains for '{}'...", app_name));

                        // Find app by name
                        match self.client.list_applications().await {
                            Ok(apps) => {
                                if let Some(app) = apps.iter().find(|a| a.name == app_name) {
                                    match self.client.list_custom_domains(app.id).await {
                                        Ok(domains) => {
                                            if domains.is_empty() {
                                                self.add_output_line(
                                                    "📭 No custom domains configured".to_string(),
                                                );
                                            } else {
                                                self.add_output_line(format!(
                                                    "Found {} domain(s):",
                                                    domains.len()
                                                ));
                                                for domain in domains {
                                                    let status = if domain.verified {
                                                        "✅ Verified"
                                                    } else {
                                                        "⏳ Pending"
                                                    };
                                                    self.add_output_line(format!(
                                                        "  🌐 {} - {}",
                                                        domain.domain, status
                                                    ));
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            self.add_output_line(format!(
                                                "❌ Failed to list domains: {}",
                                                e
                                            ));
                                        }
                                    }
                                } else {
                                    self.add_output_line(format!(
                                        "❌ Application '{}' not found",
                                        app_name
                                    ));
                                }
                            }
                            Err(e) => {
                                self.add_output_line(format!(
                                    "❌ Error listing applications: {}",
                                    e
                                ));
                            }
                        }
                    }
                    _ => {
                        self.add_output_line(format!("❌ Unknown domain action: {}", args[1]));
                        self.add_output_line("💡 Use: add, list, or delete".to_string());
                    }
                }
            }
            _ => {
                self.add_output_line(format!("❌ Unknown aether command: {}", args[0]));
                self.add_output_line("💡 Type 'help' for available commands".to_string());
            }
        }

        Ok(())
    }

    fn change_directory(&mut self, args: &[&str]) {
        let target = if args.is_empty() {
            dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."))
        } else {
            let path_str = args[0];
            if path_str.starts_with('/') {
                std::path::PathBuf::from(path_str)
            } else {
                self.current_dir.join(path_str)
            }
        };

        match std::fs::canonicalize(&target) {
            Ok(canonical_path) => {
                if canonical_path.is_dir() {
                    self.current_dir = canonical_path;
                    self.add_output_line(format!("📂 Changed to: {}", self.current_dir.display()));
                } else {
                    self.add_output_line(format!("❌ Not a directory: {}", target.display()));
                }
            }
            Err(e) => {
                self.add_output_line(format!("❌ Cannot change directory: {}", e));
            }
        }
    }

    fn list_directory(&mut self) {
        match std::fs::read_dir(&self.current_dir) {
            Ok(entries) => {
                self.add_output_line(format!("📂 Contents of {}:", self.current_dir.display()));
                self.add_output_line("".to_string());

                let mut dirs = Vec::new();
                let mut files = Vec::new();

                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                        dirs.push(format!("📁 {}/", name));
                    } else {
                        let icon = match name.split('.').last().unwrap_or("") {
                            "js" | "ts" => "🟨",
                            "json" => "🟫",
                            "md" => "📝",
                            "txt" => "📄",
                            "rs" => "🦀",
                            "py" => "🐍",
                            _ => "📄",
                        };
                        files.push(format!("{} {}", icon, name));
                    }
                }

                // Show directories first
                for dir in dirs {
                    self.add_output_line(format!("  {}", dir));
                }
                for file in files {
                    self.add_output_line(format!("  {}", file));
                }
            }
            Err(e) => {
                self.add_output_line(format!("❌ Cannot read directory: {}", e));
            }
        }
    }

    async fn execute_shell_command(&mut self, command: &str) {
        self.add_output_line("🔄 Executing shell command...".to_string());

        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(&["/C", command])
                .current_dir(&self.current_dir)
                .output()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(&self.current_dir)
                .output()
        };

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if !stdout.trim().is_empty() {
                    for line in stdout.lines() {
                        self.add_output_line(line.to_string());
                    }
                }

                if !stderr.trim().is_empty() {
                    for line in stderr.lines() {
                        self.add_output_line(format!("❌ {}", line));
                    }
                }

                if output.status.success() {
                    if stdout.trim().is_empty() && stderr.trim().is_empty() {
                        self.add_output_line("✅ Command completed successfully".to_string());
                    }
                } else {
                    self.add_output_line(format!(
                        "❌ Command failed with exit code: {:?}",
                        output.status.code()
                    ));
                }
            }
            Err(e) => {
                self.add_output_line(format!("❌ Failed to execute command: {}", e));
            }
        }
    }

    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                if self.current_tab == 0 && !self.command_input.is_empty() {
                    // Tab completion in terminal
                    if self.show_completions {
                        // Navigate through completions
                        self.completion_index =
                            (self.completion_index + 1) % self.completion_suggestions.len();
                    } else {
                        // Generate completions
                        self.generate_completions();
                    }
                } else {
                    // Switch between tabs: Terminal, File Explorer, Apps, Auth
                    let old_tab = self.current_tab;
                    self.current_tab = (self.current_tab + 1) % 4;

                    // Refresh data when switching to certain tabs
                    if self.current_tab == 2 && old_tab != 2 && self.is_authenticated {
                        // Switched to apps tab, reset fetch timer to refresh immediately
                        self.apps_last_fetched =
                            std::time::Instant::now() - std::time::Duration::from_secs(10);
                    }
                }
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match c {
                        'c' => {
                            if self.is_streaming_logs {
                                // Stop streaming first, don't quit immediately
                                self.is_streaming_logs = false;
                                self.add_output_line("⏹️  Battle log streaming ended!".to_string());
                            } else {
                                let sparkle = PokemonTheme::get_random_sparkle();
                                self.add_output_line(format!("{} Thanks for playing, Trainer! Returning to Pallet Town... {}", sparkle, sparkle));
                                self.should_quit = true;
                            }
                        }
                        'l' => {
                            // Clear screen with Pokemon theme
                            self.output_lines.clear();
                            self.show_pokemon_welcome();
                        }
                        't' => {
                            // Cycle Pokemon theme
                            self.cycle_pokemon_theme();
                        }
                        's' => {
                            // Generate new sparkles
                            self.generate_sparkles();
                            let sparkle = PokemonTheme::get_random_sparkle();
                            self.add_output_line(format!(
                                "{} Sparkles refreshed! Magic is everywhere! {}",
                                sparkle, sparkle
                            ));
                        }
                        _ => {}
                    }
                } else if c == 'd' && self.current_tab == 2 && self.pending_delete_app.is_none() {
                    // Delete app in Apps tab
                    if self.is_authenticated
                        && !self.applications.is_empty()
                        && self.selected_app_index < self.applications.len()
                    {
                        let app_index = self.selected_app_index;
                        let app_name = self.applications[app_index].name.clone();
                        let app_id = self.applications[app_index].id;

                        let sparkle = PokemonTheme::get_random_sparkle();
                        self.add_output_line(format!(
                            "{} Preparing to delete application: {} {}",
                            sparkle, app_name, sparkle
                        ));
                        self.add_output_line(
                            "⚠️  Are you sure? This action cannot be undone!".to_string(),
                        );
                        self.add_output_line(
                            "Press 'y' to confirm, any other key to cancel...".to_string(),
                        );

                        // Set a flag to wait for confirmation
                        self.pending_delete_app = Some((app_id, app_name));
                    }
                } else if c == 'y' && self.pending_delete_app.is_some() && self.current_tab == 2 {
                    // Mark for deletion - will be handled in run_app main loop
                    // Keep the pending_delete_app to signal deletion
                } else if self.pending_delete_app.is_some() {
                    // Cancel delete on any other key
                    if let Some((_, app_name)) = self.pending_delete_app.take() {
                        let sparkle = PokemonTheme::get_random_sparkle();
                        self.add_output_line(format!(
                            "{} Deletion cancelled for '{}' {}",
                            sparkle, app_name, sparkle
                        ));
                    }
                } else if self.current_tab == 0 {
                    // Only accept text input in terminal tab
                    // Hide completions when typing
                    self.show_completions = false;
                    self.command_input.insert(self.cursor_position, c);
                    self.cursor_position += 1;
                }
            }
            KeyCode::Backspace => {
                if self.current_tab == 0 && self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.command_input.remove(self.cursor_position);
                    self.show_completions = false; // Hide completions when editing
                }
            }
            KeyCode::Delete => {
                if self.cursor_position < self.command_input.len() {
                    self.command_input.remove(self.cursor_position);
                }
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_position < self.command_input.len() {
                    self.cursor_position += 1;
                }
            }
            KeyCode::Home => {
                self.cursor_position = 0;
            }
            KeyCode::End => {
                self.cursor_position = self.command_input.len();
            }
            KeyCode::Up => {
                if self.current_tab == 0 {
                    // Terminal tab
                    if self.show_completions && !self.completion_suggestions.is_empty() {
                        if self.completion_index > 0 {
                            self.completion_index -= 1;
                        }
                    } else {
                        self.navigate_history_up();
                    }
                } else if self.current_tab == 1 {
                    // File explorer tab
                    if self.selected_file_index > 0 {
                        self.selected_file_index -= 1;
                    }
                } else if self.current_tab == 2 {
                    // Apps tab
                    if self.selected_app_index > 0 {
                        self.selected_app_index -= 1;
                    }
                }
            }
            KeyCode::Down => {
                if self.current_tab == 0 {
                    // Terminal tab
                    if self.show_completions && !self.completion_suggestions.is_empty() {
                        if self.completion_index
                            < self.completion_suggestions.len().saturating_sub(1)
                        {
                            self.completion_index += 1;
                        }
                    } else {
                        self.navigate_history_down();
                    }
                } else if self.current_tab == 1 {
                    // File explorer tab
                    if self.selected_file_index < self.file_tree.len().saturating_sub(1) {
                        self.selected_file_index += 1;
                    }
                } else if self.current_tab == 2 {
                    // Apps tab
                    if self.selected_app_index < self.applications.len().saturating_sub(1) {
                        self.selected_app_index += 1;
                    }
                }
            }
            KeyCode::Enter => {
                if self.current_tab == 0 && self.show_completions {
                    // Apply selected completion with Pokemon feedback
                    self.apply_completion();
                    let sparkle = PokemonTheme::get_random_sparkle();
                    self.add_output_line(format!("{} Move selected! {}", sparkle, sparkle));
                } else if self.current_tab == 1 && !self.file_tree.is_empty() {
                    // File explorer
                    let selected = self.selected_file_index;
                    if selected < self.file_tree.len() {
                        if self.file_tree[selected].is_dir {
                            // Toggle directory expansion with Pokemon theme
                            self.expand_directory(selected);
                            let sparkle = PokemonTheme::get_random_sparkle();
                            self.add_output_line(format!(
                                "🌳 {} Exploring route... {}",
                                sparkle, sparkle
                            ));
                        } else {
                            // Navigate to directory containing the file
                            let parent = self.file_tree[selected].path.parent();
                            if let Some(parent_path) = parent {
                                self.current_dir = parent_path.to_path_buf();
                                self.rebuild_file_tree();
                                self.add_output_line(format!(
                                    "�️ ✨ Traveled to new area: {} ✨",
                                    self.current_dir.display()
                                ));
                            }
                        }
                    }
                } else if self.current_tab == 2
                    && self.is_authenticated
                    && !self.applications.is_empty()
                {
                    // Apps tab
                    // Open deployment URL if available
                    let app_index = self.selected_app_index;
                    if app_index < self.applications.len() {
                        if let Some(url) = self.applications[app_index].deployment_url.clone() {
                            self.add_output_line(format!("🌐 Opening URL: {}", url));

                            // Try to open URL in browser
                            #[cfg(target_os = "linux")]
                            {
                                let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                            }
                            #[cfg(target_os = "macos")]
                            {
                                let _ = std::process::Command::new("open").arg(&url).spawn();
                            }
                            #[cfg(target_os = "windows")]
                            {
                                let _ = std::process::Command::new("cmd")
                                    .args(&["/C", "start", &url])
                                    .spawn();
                            }

                            let sparkle = PokemonTheme::get_random_sparkle();
                            self.add_output_line(format!(
                                "{} URL opened in browser! {}",
                                sparkle, sparkle
                            ));
                        } else {
                            let app_name = self.applications[app_index].name.clone();
                            self.add_output_line(format!(
                                "❌ No deployment URL available for '{}'",
                                app_name
                            ));
                        }
                    }
                }
                // Handle command execution in main loop for terminal tab
            }
            KeyCode::PageUp => {
                // Scroll up in terminal output
                if self.current_tab == 0 && self.output_lines.len() > 0 {
                    // Scroll up by 10 lines at a time
                    self.terminal_scroll_offset = self.terminal_scroll_offset.saturating_add(10);
                    // Don't scroll past the beginning
                    if self.terminal_scroll_offset > self.output_lines.len() {
                        self.terminal_scroll_offset = self.output_lines.len();
                    }
                }
            }
            KeyCode::PageDown => {
                // Scroll down in terminal output
                if self.current_tab == 0 {
                    // Scroll down by 10 lines at a time
                    self.terminal_scroll_offset = self.terminal_scroll_offset.saturating_sub(10);
                }
            }
            KeyCode::Esc => {
                if self.current_tab == 0 {
                    if self.is_streaming_logs {
                        // Stop log streaming
                        self.is_streaming_logs = false;
                        self.add_output_line("⏹️  Log streaming stopped by user.".to_string());
                    }
                    self.show_completions = false;
                }
            }

            _ => {}
        }
    }

    fn navigate_history_up(&mut self) {
        if self.command_history.is_empty() {
            return;
        }

        let new_index = match self.history_index {
            None => self.command_history.len() - 1,
            Some(i) => {
                if i > 0 {
                    i - 1
                } else {
                    0
                }
            }
        };

        self.history_index = Some(new_index);
        self.command_input = self.command_history[new_index].clone();
        self.cursor_position = self.command_input.len();
    }

    fn navigate_history_down(&mut self) {
        match self.history_index {
            None => return,
            Some(i) => {
                if i < self.command_history.len() - 1 {
                    self.history_index = Some(i + 1);
                    self.command_input = self.command_history[i + 1].clone();
                } else {
                    self.history_index = None;
                    self.command_input.clear();
                }
                self.cursor_position = self.command_input.len();
            }
        }
    }
}

pub async fn run_terminal_dashboard() -> Result<()> {
    let config = Config::load()?;
    let client = ApiClient::new(config.api_endpoint, config.auth_token)?;
    let mut app = TerminalApp::new(client);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut TerminalApp,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        // Update applications list if authenticated and on apps tab
        if app.is_authenticated
            && (app.current_tab == 2 || app.apps_last_fetched.elapsed() > Duration::from_secs(30))
        {
            if app.apps_last_fetched.elapsed() > Duration::from_secs(5) {
                // Fetch every 5 seconds when on apps tab, or 30 seconds otherwise
                match app.client.list_applications().await {
                    Ok(applications) => {
                        app.applications = applications;
                        app.apps_last_fetched = std::time::Instant::now();
                    }
                    Err(_) => {
                        // Silently ignore errors to avoid spam, user can check auth status
                    }
                }
            }
        }

        // Handle pending delete confirmation
        if let Some((app_id, app_name)) = app.pending_delete_app.clone() {
            // Check if user confirmed by pressing 'y'
            // We need to process this here since we're in async context
            app.add_output_line(format!("🗑️  Deleting application '{}'...", app_name));

            match app.client.delete_application(app_id).await {
                Ok(_) => {
                    let sparkle = PokemonTheme::get_random_sparkle();
                    app.add_output_line(format!(
                        "{} Application '{}' deleted successfully! {}",
                        sparkle, app_name, sparkle
                    ));

                    // Refresh apps list
                    match app.client.list_applications().await {
                        Ok(apps) => {
                            app.applications = apps;
                            if app.selected_app_index >= app.applications.len()
                                && app.selected_app_index > 0
                            {
                                app.selected_app_index = app.applications.len() - 1;
                            }
                        }
                        Err(e) => {
                            app.add_output_line(format!("⚠️  Failed to refresh apps list: {}", e));
                        }
                    }
                }
                Err(e) => {
                    app.add_output_line(format!("❌ Failed to delete application: {}", e));
                }
            }

            // Clear pending delete
            app.pending_delete_app = None;
        }

        // Update streaming logs if active
        if app.is_streaming_logs {
            if let Some(app_id) = app.streaming_app_id {
                if app.last_log_check.elapsed() > Duration::from_millis(250) {
                    match app.client.get_logs(app_id, Some(100)).await {
                        Ok(logs) => {
                            if !logs.trim().is_empty() {
                                let current_time = chrono::Local::now().format("%H:%M:%S");

                                if logs != app.last_log_content {
                                    // Find new log lines by comparing line counts and content
                                    let old_lines: Vec<&str> =
                                        app.last_log_content.lines().collect();
                                    let new_lines: Vec<&str> = logs.lines().collect();

                                    if new_lines.len() > old_lines.len() {
                                        // Show only the new lines that were added
                                        for line in new_lines.iter().skip(old_lines.len()) {
                                            if !line.trim().is_empty() {
                                                app.add_output_line(format!(
                                                    "📄 [{}] {}",
                                                    current_time, line
                                                ));
                                            }
                                        }
                                    } else if new_lines != old_lines {
                                        // Content changed, show latest few lines
                                        for line in new_lines.iter().rev().take(3).rev() {
                                            if !line.trim().is_empty() {
                                                app.add_output_line(format!(
                                                    "📄 [{}] {}",
                                                    current_time, line
                                                ));
                                            }
                                        }
                                    }
                                    app.last_log_content = logs;
                                } else {
                                    // Show streaming indicator every few seconds when no new logs
                                    if app.last_log_check.elapsed() > Duration::from_secs(5) {
                                        app.add_output_line(format!(
                                            "⏳ [{}] 🔄 Streaming... (no new logs)",
                                            current_time
                                        ));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            app.add_output_line(format!("❌ Error streaming logs: {}", e));
                            app.is_streaming_logs = false;
                            app.streaming_app_id = None;
                        }
                    }
                    app.last_log_check = std::time::Instant::now();
                }
            }
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Enter => {
                            if app.current_tab == 0 {
                                // Terminal tab
                                if app.show_completions {
                                    // Apply completion instead of executing
                                    app.apply_completion();
                                } else {
                                    // Execute command
                                    let command = app.command_input.clone();
                                    app.command_input.clear();
                                    app.cursor_position = 0;
                                    app.history_index = None;
                                    app.show_completions = false;

                                    if let Err(e) = app.execute_command(command).await {
                                        app.add_output_line(format!("❌ Error: {}", e));
                                    }
                                }
                            } else {
                                // Handle enter for other tabs in handle_key_event
                                app.handle_key_event(key);
                            }
                        }
                        _ => {
                            app.handle_key_event(key);
                        }
                    }
                }
            }
        }

        // Update animations and timers
        if app.animation_timer.elapsed() >= Duration::from_millis(200) {
            app.pokemon_loader.next_frame();
            app.animation_timer = std::time::Instant::now();

            // Auto-dismiss notifications after 3 seconds
            if app.show_notification && app.animation_timer.elapsed() >= Duration::from_secs(3) {
                app.show_notification = false;
                app.current_notification = None;
            }

            // Clear battle animation after 2 seconds
            if let Some(ref mut battle) = app.battle_animation {
                battle.next_frame();
                if app.animation_timer.elapsed() >= Duration::from_secs(2) {
                    app.battle_animation = None;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, app: &mut TerminalApp) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Length(3), // Tabs area
            Constraint::Min(5),    // Content area
            Constraint::Length(3), // Input area (only visible in terminal tab)
        ])
        .split(f.area());

    // Pokemon-themed tabs with dynamic styling
    let tab_titles = match app.pokemon_theme.current_type {
        PokemonType::Electric => vec![
            "⚡ Battle Terminal",
            "🌳 Route Files",
            "🏥 Pokemon Center",
            "👤 Trainer Card",
        ],
        PokemonType::Fire => vec![
            "🔥 Volcano Terminal",
            "🌳 Route Files",
            "🏥 Pokemon Center",
            "👤 Trainer Card",
        ],
        PokemonType::Water => vec![
            "💧 Ocean Terminal",
            "🌳 Route Files",
            "🏥 Pokemon Center",
            "👤 Trainer Card",
        ],
        PokemonType::Grass => vec![
            "🌿 Forest Terminal",
            "🌳 Route Files",
            "🏥 Pokemon Center",
            "👤 Trainer Card",
        ],
        _ => vec!["✨ Terminal", "📂 Files", "🚀 Apps", "🔐 Auth"],
    };

    let sparkle1 = app.pokemon_theme.get_sparkle();
    let sparkle2 = PokemonTheme::get_random_sparkle();

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " {} AETHER POKEMON TERMINAL {} ",
                    sparkle1, sparkle2
                ))
                .title_style(app.pokemon_theme.title_style())
                .border_style(app.pokemon_theme.border_style()),
        )
        .select(app.current_tab)
        .style(app.pokemon_theme.info_style())
        .highlight_style(
            app.pokemon_theme
                .accent_style()
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        );

    f.render_widget(tabs, main_chunks[0]);

    match app.current_tab {
        0 => render_terminal_tab(f, app, main_chunks[1], main_chunks[2]),
        1 => render_file_explorer_tab(f, app, main_chunks[1]),
        2 => render_apps_tab(f, app, main_chunks[1]),
        3 => render_auth_tab(f, app, main_chunks[1]),
        _ => {}
    }

    // Render notification popup if active
    if app.show_notification {
        if let Some(ref notification) = app.current_notification {
            let popup_area = centered_rect(50, 25, f.area());
            notification.clone().render(popup_area, f.buffer_mut());
        }
    }

    // Render Pokemon status widget in corner only if there's enough space
    if f.area().width > 30 && f.area().height > 15 {
        let status = PokemonStatus::new("Aether", app.pokemon_theme.current_type)
            .hp(85.0)
            .mp(70.0)
            .level(42)
            .add_status("Coding Boost")
            .add_status("Debug Vision");

        let status_area = Rect {
            x: f.area().width.saturating_sub(25),
            y: f.area().height.saturating_sub(12),
            width: 24.min(f.area().width),
            height: 11.min(f.area().height),
        };

        status.render(status_area, f.buffer_mut());
    }
}

// Helper function for centered popup rectangles
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_terminal_tab(
    f: &mut Frame,
    app: &mut TerminalApp,
    content_area: Rect,
    _input_area: Rect,
) {
    // Split content area to show Pokemon ASCII art on the side if there's enough space
    let (main_chunks, pokemon_area) = if content_area.width > 120 {
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Min(80),    // Main terminal area
                Constraint::Length(40), // Pokemon ASCII area
            ])
            .split(content_area);

        let chunks = if app.show_completions && !app.completion_suggestions.is_empty() {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Min(5), // Output area
                    Constraint::Length(std::cmp::min(
                        app.completion_suggestions.len() as u16 + 2,
                        8,
                    )), // Completions
                    Constraint::Length(3), // Input area
                ])
                .split(horizontal_chunks[0])
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Min(3),    // Output area
                    Constraint::Length(3), // Input area
                ])
                .split(horizontal_chunks[0])
        };
        (chunks, Some(horizontal_chunks[1]))
    } else {
        let chunks = if app.show_completions && !app.completion_suggestions.is_empty() {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Min(5), // Output area
                    Constraint::Length(std::cmp::min(
                        app.completion_suggestions.len() as u16 + 2,
                        8,
                    )), // Completions
                    Constraint::Length(3), // Input area
                ])
                .split(content_area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Min(3),    // Output area
                    Constraint::Length(3), // Input area
                ])
                .split(content_area)
        };
        (chunks, None)
    };

    let chunks = main_chunks;

    // Show battle animation if active
    if let Some(ref battle_anim) = app.battle_animation {
        let battle_area = Rect {
            x: content_area.x + content_area.width / 4,
            y: content_area.y + 2,
            width: content_area.width / 2,
            height: 8,
        };
        Clear.render(battle_area, f.buffer_mut());
        battle_anim.clone().render(battle_area, f.buffer_mut());
    }

    // Pokemon-themed output area with dynamic styling
    let output_text = app.output_lines.join("\n");
    let loader_frame = app.pokemon_loader.frames[app.pokemon_loader.current_frame].clone();

    // Calculate scroll position
    let total_lines = app.output_lines.len();
    let visible_lines = chunks[0].height.saturating_sub(2) as usize; // Minus borders

    // Calculate scroll: if scroll_offset is 0, show bottom; otherwise scroll up by offset
    let scroll_position = if app.terminal_scroll_offset == 0 {
        // Auto-scroll to bottom (default behavior)
        total_lines.saturating_sub(visible_lines) as u16
    } else {
        // Manual scroll: scroll_offset is how many lines from bottom we are
        let lines_from_bottom = total_lines.saturating_sub(app.terminal_scroll_offset);
        lines_from_bottom.saturating_sub(visible_lines) as u16
    };

    let title = if app.is_streaming_logs {
        format!(
            " {} BATTLE LOG STREAMING {} ",
            PokemonTheme::get_random_sparkle(),
            PokemonTheme::get_random_sparkle()
        )
    } else {
        let scroll_indicator = if app.terminal_scroll_offset > 0 {
            format!(
                " [↑ Scrolled: {}/{} lines] ",
                app.terminal_scroll_offset, total_lines
            )
        } else {
            String::new()
        };
        format!(
            " {} POKEMON BATTLE TERMINAL {}{} ",
            app.pokemon_theme.get_sparkle(),
            loader_frame.trim(),
            scroll_indicator
        )
    };

    let output = Paragraph::new(output_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(if app.is_streaming_logs {
                    app.pokemon_theme
                        .error_style()
                        .add_modifier(Modifier::RAPID_BLINK)
                } else {
                    app.pokemon_theme.header_style()
                })
                .border_style(if app.is_streaming_logs {
                    app.pokemon_theme.error_style()
                } else {
                    app.pokemon_theme.border_style()
                }),
        )
        .style(app.pokemon_theme.info_style())
        .wrap(Wrap { trim: false })
        .scroll((scroll_position, 0));

    f.render_widget(output, chunks[0]);

    // Completions area (if visible)
    let input_chunk_index = if app.show_completions && !app.completion_suggestions.is_empty() {
        let completion_height = chunks[1].height.saturating_sub(2) as usize; // Available height minus borders
        let total_items = app.completion_suggestions.len();

        // Calculate scroll offset to keep selected item visible
        let scroll_offset = if app.completion_index >= completion_height {
            app.completion_index
                .saturating_sub(completion_height.saturating_sub(1))
        } else {
            0
        };

        let mut completion_items = Vec::new();
        let visible_start = scroll_offset;
        let visible_end = (scroll_offset + completion_height).min(total_items);

        for i in visible_start..visible_end {
            let suggestion = &app.completion_suggestions[i];
            let (prefix, style) = if i == app.completion_index {
                (
                    "🔥 📁",
                    Style::default()
                        .fg(Color::Rgb(255, 255, 255)) // White
                        .bg(Color::Rgb(255, 69, 0)) // Red orange
                        .add_modifier(Modifier::BOLD | Modifier::ITALIC),
                )
            } else {
                (
                    "💎 📁",
                    Style::default()
                        .fg(Color::Rgb(135, 206, 250)) // Light sky blue
                        .add_modifier(Modifier::BOLD),
                )
            };
            completion_items.push(ListItem::new(format!("{} {}", prefix, suggestion)).style(style));
        }

        let scroll_indicator = if total_items > completion_height {
            if scroll_offset > 0 && visible_end < total_items {
                " ⬆️⬇️ "
            } else if scroll_offset > 0 {
                " ⬆️ "
            } else if visible_end < total_items {
                " ⬇️ "
            } else {
                ""
            }
        } else {
            ""
        };

        let help_text = format!(
            " ↑↓ Navigate | Enter Select | Esc Cancel | {}/{}{} ",
            app.completion_index + 1,
            app.completion_suggestions.len(),
            scroll_indicator
        );

        let completions_list = List::new(completion_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(help_text)
                .title_style(
                    Style::default()
                        .fg(Color::Rgb(255, 140, 0)) // Dark orange
                        .bg(Color::Rgb(72, 61, 139)) // Dark slate blue
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                )
                .border_style(Style::default().fg(Color::Rgb(255, 215, 0))),
        );

        f.render_widget(completions_list, chunks[1]);
        2
    } else {
        1
    };

    // Input area with full path
    let full_path = app.current_dir.display().to_string();
    let prompt = if full_path.len() > 60 {
        // Truncate long paths
        format!("...{}$ ", &full_path[full_path.len() - 57..])
    } else {
        format!("{}$ ", full_path)
    };

    let input_text = format!("{}{}", prompt, app.command_input);
    let input = Paragraph::new(input_text.clone())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ⚡ ═══ COMMAND NEXUS ═══ ⚡ ")
                .title_style(
                    Style::default()
                        .fg(Color::Rgb(50, 205, 50)) // Lime green
                        .bg(Color::Rgb(25, 25, 25)) // Very dark gray
                        .add_modifier(Modifier::BOLD | Modifier::ITALIC),
                )
                .border_style(Style::default().fg(Color::Rgb(0, 250, 154))),
        ) // Medium spring green
        .style(
            Style::default()
                .fg(Color::Rgb(255, 255, 255)) // White text
                .bg(Color::Rgb(0, 0, 0)),
        );

    f.render_widget(input, chunks[input_chunk_index]);

    // Set cursor position
    let cursor_x = chunks[input_chunk_index].x + (prompt.len() + app.cursor_position) as u16;
    let cursor_y = chunks[input_chunk_index].y + 1;
    f.set_cursor_position((
        cursor_x.min(chunks[input_chunk_index].right() - 1),
        cursor_y,
    ));

    // Render Pokemon ASCII art if there's space
    if let Some(pokemon_area) = pokemon_area {
        render_pokemon_ascii(f, app, pokemon_area);
    }
}

fn render_file_explorer_tab(f: &mut Frame, app: &TerminalApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints::<Vec<Constraint>>(
            [
                Constraint::Percentage(50), // File tree
                Constraint::Percentage(50), // File info
            ]
            .into(),
        )
        .split(area);

    // File tree
    let mut items = Vec::new();
    for (i, item) in app.file_tree.iter().enumerate() {
        let indent = "  ".repeat(item.depth);
        let icon = if item.is_dir {
            if item.is_expanded {
                "📂"
            } else {
                "📁"
            }
        } else {
            match item.path.extension().and_then(|s| s.to_str()).unwrap_or("") {
                "rs" => "🦀",
                "js" | "ts" => "🟨",
                "json" => "🟫",
                "md" => "📝",
                "txt" => "📄",
                _ => "📄",
            }
        };

        let style = if i == app.selected_file_index {
            Style::default()
                .fg(Color::Rgb(255, 255, 255)) // White
                .bg(Color::Rgb(255, 20, 147)) // Deep pink
                .add_modifier(Modifier::BOLD | Modifier::ITALIC)
        } else if item.is_dir {
            Style::default()
                .fg(Color::Rgb(135, 206, 250)) // Light sky blue
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(152, 251, 152)) // Pale green
        };

        items.push(ListItem::new(format!("{}{} {}", indent, icon, item.name)).style(style));
    }

    let file_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" 🌲 ═══ {} ═══ 🌲 ", app.current_dir.display()))
            .title_style(
                Style::default()
                    .fg(Color::Rgb(255, 140, 0)) // Dark orange
                    .bg(Color::Rgb(47, 79, 79)) // Dark slate gray
                    .add_modifier(Modifier::BOLD | Modifier::ITALIC),
            )
            .border_style(Style::default().fg(Color::Rgb(0, 206, 209))),
    );

    f.render_widget(file_list, chunks[0]);

    // File info panel
    let info_text = if app.selected_file_index < app.file_tree.len() {
        let selected = &app.file_tree[app.selected_file_index];
        format!(
            "📁 Selected: {}\n\n📍 Path: {}\n\n📊 Type: {}\n\n💡 Press Enter to {}",
            selected.name,
            selected.path.display(),
            if selected.is_dir { "Directory" } else { "File" },
            if selected.is_dir {
                if selected.is_expanded {
                    "collapse"
                } else {
                    "expand"
                }
            } else {
                "navigate to parent"
            }
        )
    } else {
        "Use ↑↓ arrows to navigate\nPress Enter to expand/collapse directories\nPress Tab to switch tabs".to_string()
    };

    let info_panel = Paragraph::new(info_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" � ═══ FILE NEXUS ═══ 💠 ")
                .title_style(
                    Style::default()
                        .fg(Color::Rgb(50, 205, 50)) // Lime green
                        .bg(Color::Rgb(105, 105, 105)) // Dim gray
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                )
                .border_style(Style::default().fg(Color::Rgb(127, 255, 212))),
        ) // Aquamarine
        .style(
            Style::default()
                .fg(Color::Rgb(240, 248, 255)) // Alice blue
                .add_modifier(Modifier::ITALIC),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(info_panel, chunks[1]);
}

fn render_apps_tab(f: &mut Frame, app: &TerminalApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60), // Apps list
            Constraint::Percentage(40), // App details
        ])
        .split(area);

    // Applications list
    let mut app_items = Vec::new();

    if !app.is_authenticated {
        app_items.push(
            ListItem::new("� Please authenticate first").style(
                Style::default()
                    .fg(Color::Rgb(255, 165, 0))
                    .add_modifier(Modifier::BOLD),
            ),
        );
        app_items.push(
            ListItem::new("   Run: aether login")
                .style(Style::default().fg(Color::Rgb(169, 169, 169))),
        );
    } else if app.applications.is_empty() {
        app_items.push(
            ListItem::new("📭 No applications found").style(
                Style::default()
                    .fg(Color::Rgb(169, 169, 169))
                    .add_modifier(Modifier::ITALIC),
            ),
        );
        app_items.push(
            ListItem::new("💡 Deploy an app first:")
                .style(Style::default().fg(Color::Rgb(135, 206, 250))),
        );
        app_items.push(
            ListItem::new("   aether deploy").style(Style::default().fg(Color::Rgb(144, 238, 144))),
        );
    } else {
        for (i, application) in app.applications.iter().enumerate() {
            let icon = "🚀";
            let status_icon = "✅"; // For now, assume all are running

            let style = if i == app.selected_app_index {
                // Highlight selected app
                Style::default()
                    .fg(Color::Rgb(255, 255, 255)) // White text
                    .bg(Color::Rgb(255, 20, 147)) // Deep pink background
                    .add_modifier(Modifier::BOLD | Modifier::ITALIC)
            } else if i % 2 == 0 {
                Style::default()
                    .fg(Color::Rgb(144, 238, 144))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::Rgb(135, 206, 250))
                    .add_modifier(Modifier::BOLD)
            };

            let url_display = if let Some(url) = &application.deployment_url {
                // Truncate long URLs to prevent buffer overflow
                let truncated_url = if url.len() > 50 {
                    format!("{}...", &url[..47])
                } else {
                    url.clone()
                };
                format!(" 🌐 {}", truncated_url)
            } else {
                " ❌ No URL".to_string()
            };

            app_items.push(
                ListItem::new(format!(
                    "{} {} {} {}{}",
                    icon, application.name, status_icon, "Running", url_display
                ))
                .style(style),
            );
        }
    }

    let apps_list = List::new(app_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " 🚀 ═══ APPLICATIONS ({}) ═══ 🚀 ",
                    app.applications.len()
                ))
                .title_style(
                    Style::default()
                        .fg(Color::Rgb(255, 20, 147)) // Deep pink
                        .bg(Color::Rgb(25, 25, 112)) // Midnight blue
                        .add_modifier(Modifier::BOLD | Modifier::ITALIC),
                )
                .border_style(Style::default().fg(Color::Rgb(138, 43, 226))),
        ) // Purple
        .style(Style::default().fg(Color::Rgb(173, 216, 230))); // Light blue

    f.render_widget(apps_list, chunks[0]);

    // Application details panel
    let details_text = if !app.is_authenticated {
        "🔐 Authentication Required\n\n❌ Status: NOT AUTHENTICATED\n\n🔧 Actions needed:\n• Run 'aether login' to authenticate\n• Then return to view your applications\n\n💡 Commands:\n  aether register  - Create account\n  aether login     - Login to account".to_string()
    } else if app.applications.is_empty() {
        "📦 No Applications Yet\n\n✨ Ready to deploy your first app!\n\n🚀 Quick Start:\n1. Navigate to your project folder\n2. Run 'aether deploy'\n3. Watch your app come to life!\n\n💡 Supported runtimes:\n• Node.js (package.json)\n• More coming soon...".to_string()
    } else {
        let selected_app = if app.selected_app_index < app.applications.len() {
            Some(&app.applications[app.selected_app_index])
        } else {
            None
        };

        if let Some(selected) = selected_app {
            let url_info = if let Some(url) = &selected.deployment_url {
                // Break long URLs into multiple lines to prevent buffer issues
                let formatted_url = if url.len() > 40 {
                    format!(
                        "🌐 Deployment URL:\n   {}\n   {}\n\n💡 Press ENTER to open in browser!",
                        &url[..40],
                        &url[40..]
                    )
                } else {
                    format!(
                        "🌐 Deployment URL:\n   {}\n\n💡 Press ENTER to open in browser!",
                        url
                    )
                };
                formatted_url
            } else {
                "❌ No deployment URL available\n   Deploy this app to get a URL".to_string()
            };

            format!(
                "📱 Selected App Details\n\n📦 Name: {}\n🔧 Runtime: {}\n📅 Created: {}\n\n{}\n\n🎯 Quick Actions:\n• ENTER  → Open URL in browser\n• 'd'    → Delete app\n• ↑↓     → Select app\n• Tab    → Switch tabs",
                selected.name,
                selected.runtime,
                selected.created_at.format("%Y-%m-%d %H:%M"),
                url_info
            )
        } else {
            let app_count = app.applications.len();
            let updated = chrono::Local::now().format("%H:%M:%S");

            format!(
                "📊 Applications Overview\n\n� Statistics:\n• Total Applications: {}\n• Last Updated: {}\n\n🎯 Controls:\n• ↑↓ arrows to select app\n• Enter to open deployment URL\n• Tab to switch between areas\n\n💡 Tips:\n• Apps auto-refresh every 30s\n• Use terminal tab for commands",
                app_count, updated
            )
        }
    };

    let details_panel = Paragraph::new(details_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" � ═══ APP DETAILS ═══ 💠 ")
                .title_style(
                    Style::default()
                        .fg(Color::Rgb(50, 205, 50)) // Lime green
                        .bg(Color::Rgb(105, 105, 105)) // Dim gray
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                )
                .border_style(Style::default().fg(Color::Rgb(127, 255, 212))),
        ) // Aquamarine
        .style(
            Style::default()
                .fg(Color::Rgb(240, 248, 255)) // Alice blue
                .add_modifier(Modifier::ITALIC),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(details_panel, chunks[1]);
}

fn render_auth_tab(f: &mut Frame, app: &TerminalApp, area: Rect) {
    let auth_text = if app.is_authenticated {
        format!(
            "🔐 Authentication Status\n\n✅ Status: AUTHENTICATED\n\n🔧 Available Actions:\n• View user info\n• Logout from account\n• Deploy applications\n• Manage apps\n\n💡 Commands:\n  aether logout    - Logout and clear token\n  aether deploy    - Deploy your applications\n  aether apps      - List your applications\n\n🌟 You are ready to deploy!"
        )
    } else {
        "🔓 Authentication Status\n\n❌ Status: NOT AUTHENTICATED\n\n🔐 Required Actions:\n• Register new account OR Login to existing account\n\n💡 Commands:\n  aether register  - Create new account\n  aether login     - Login to existing account\n\n⚠️  You must authenticate before deploying applications!\n\n🎯 Quick Start:\n1. Run 'aether register' to create account\n2. Or 'aether login' if you have account\n3. Then use 'aether deploy' to deploy apps".to_string()
    };

    let title = if app.is_authenticated {
        " 🔐 ═══ AUTHENTICATED USER ═══ 🔐 "
    } else {
        " 🔓 ═══ PLEASE AUTHENTICATE ═══ 🔓 "
    };

    let title_color = if app.is_authenticated {
        Color::Rgb(0, 255, 0) // Green for authenticated
    } else {
        Color::Rgb(255, 165, 0) // Orange for not authenticated
    };

    let auth_panel = Paragraph::new(auth_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(
                    Style::default()
                        .fg(title_color)
                        .bg(Color::Rgb(25, 25, 25))
                        .add_modifier(Modifier::BOLD | Modifier::ITALIC),
                )
                .border_style(Style::default().fg(title_color)),
        )
        .style(
            Style::default()
                .fg(Color::Rgb(220, 220, 220))
                .add_modifier(Modifier::ITALIC),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(auth_panel, area);
}

impl TerminalApp {
    /// Deploy the current project using API directly instead of external command
    async fn deploy_current_project(&mut self) -> Result<()> {
        use crate::{
            api::CreateAppRequest, builder::ProjectBuilder, commands::find_app_by_name,
            presigned_uploader::PresignedUploader,
        };

        let project_path = self.current_dir.clone();

        // Step 1: Project Analysis
        self.add_output_line("".to_string());
        self.add_output_line("🔍 Analyzing project...".to_string());
        self.add_output_line(format!("📁 Project path: {}", project_path.display()));

        // Initialize project builder
        let builder = ProjectBuilder::new(&project_path)?;
        let app_name = builder.get_app_name();
        let app_runtime = builder.detect_runtime();

        self.add_output_line(format!("📦 App name: {}", app_name));
        self.add_output_line(format!("🏷️ Version: {}", builder.get_version()));
        self.add_output_line(format!("🔧 Runtime: {}", app_runtime));
        self.add_output_line("".to_string());

        // Check if app already exists
        let existing_app = find_app_by_name(&self.client, &app_name).await?;

        let app = if let Some(existing_app) = existing_app {
            self.add_output_line(format!("� Using existing application: {}", app_name));
            existing_app
        } else {
            // Create new application
            self.add_output_line("� Creating new application...".to_string());
            let create_request = CreateAppRequest {
                name: app_name.to_string(),
                description: Some(
                    "NodeJS application deployed via AetherEngine CLI Dashboard".to_string(),
                ),
                runtime: app_runtime.clone(),
            };

            self.client.create_application(create_request).await?
        };

        // Build the project
        self.add_output_line("� Building project...".to_string());
        let artifact_path = self.build_project_silent(&builder).await?;
        self.add_output_line("🗜️ Creating deployment artifact...".to_string());
        self.add_output_line(format!("📦 Artifact: {}", artifact_path.display()));
        self.add_output_line("".to_string());

        // Step 4: Upload to S3
        self.add_output_line("☁️ Preparing S3 upload...".to_string());
        self.add_output_line("📤 Uploading artifact to S3...".to_string());
        let (artifact_url, _presigned_url) = self
            .upload_to_s3_silent(&artifact_path, app.id, &builder.get_version())
            .await?;

        self.add_output_line("✅ Upload successful!".to_string());
        self.add_output_line("".to_string());

        // Step 5: Create Deployment
        self.add_output_line("🚀 Initiating deployment...".to_string());
        let deployment = self
            .client
            .deploy_application(app.id, builder.get_version(), artifact_url.clone())
            .await?;

        self.add_output_line("🎉 Deployment completed successfully!".to_string());
        self.add_output_line(format!("📱 App ID: {}", app.id));
        self.add_output_line(format!("� Deployment ID: {}", deployment.id));
        self.add_output_line("".to_string());
        
        // Web Dashboard promotion
        self.add_output_line("╔═══════════════════════════════════════════════════════════════════════════╗".to_string());
        self.add_output_line("║                    🌐  MANAGE YOUR APP ONLINE  🌐                        ║".to_string());
        self.add_output_line("║                                                                           ║".to_string());
        self.add_output_line("║  🎯 View, monitor and manage your deployed app at:                       ║".to_string());
        self.add_output_line("║                                                                           ║".to_string());
        self.add_output_line("║                    ➡️  https://aetherngine.com/  ⬅️                        ║".to_string());
        self.add_output_line("║                                                                           ║".to_string());
        self.add_output_line("║  ✨ Real-time monitoring, logs, metrics & deployment management!         ║".to_string());
        self.add_output_line("╚═══════════════════════════════════════════════════════════════════════════╝".to_string());

        Ok(())
    }

    // Silent build method that doesn't interfere with dashboard output
    async fn build_project_silent(&self, builder: &ProjectBuilder) -> Result<std::path::PathBuf> {
        use std::process::Stdio;

        // Check if dependencies need to be installed
        let node_modules = builder.get_project_path().join("node_modules");
        if !node_modules.exists() {
            // Run npm install with suppressed output
            let output = Command::new("npm")
                .args(&["install"])
                .current_dir(builder.get_project_path())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .output()
                .map_err(|e| anyhow::anyhow!("Failed to run npm install: {}", e))?;

            if !output.status.success() {
                return Err(anyhow::anyhow!("npm install failed").into());
            }
        }

        // Run build script if it exists
        if let Some(ref scripts) = builder.get_package_json().scripts {
            if scripts.contains_key("build") {
                let output = Command::new("npm")
                    .args(&["run", "build"])
                    .current_dir(builder.get_project_path())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .output()
                    .map_err(|e| anyhow::anyhow!("Failed to run build script: {}", e))?;

                if !output.status.success() {
                    return Err(anyhow::anyhow!("Build script failed").into());
                }
            }
        }

        // Create artifact
        self.create_artifact_silent(builder).await
    }

    async fn create_artifact_silent(&self, builder: &ProjectBuilder) -> Result<std::path::PathBuf> {
        use flate2::{write::GzEncoder, Compression};
        use std::fs::File;
        use tar::Builder as TarBuilder;

        let temp_dir = std::env::temp_dir();
        let artifact_path = temp_dir.join(format!("{}.tar.gz", builder.get_app_name()));

        let tar_gz = File::create(&artifact_path)?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = TarBuilder::new(enc);

        // Add files to tar
        self.add_directory_to_tar(&mut tar, builder.get_project_path(), "")?;

        tar.finish()?;
        Ok(artifact_path)
    }

    fn add_directory_to_tar<W: std::io::Write>(
        &self,
        tar: &mut TarBuilder<W>,
        dir: &std::path::Path,
        prefix: &str,
    ) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Skip certain directories and files
            if name_str.starts_with('.')
                || name_str == "node_modules"
                || name_str == "target"
                || name_str == "dist"
                || name_str.ends_with(".log")
            {
                continue;
            }

            let archive_path = if prefix.is_empty() {
                name_str.to_string()
            } else {
                format!("{}/{}", prefix, name_str)
            };

            if path.is_dir() {
                self.add_directory_to_tar(tar, &path, &archive_path)?;
            } else {
                tar.append_path_with_name(&path, &archive_path)?;
            }
        }
        Ok(())
    }

    // Silent S3 upload that doesn't interfere with dashboard output
    async fn upload_to_s3_silent(
        &self,
        artifact_path: &std::path::Path,
        app_id: uuid::Uuid,
        version: &str,
    ) -> Result<(String, String)> {
        use crate::presigned_uploader::PresignedUploader;

        let presigned_uploader = PresignedUploader::new(self.client.clone());
        let (artifact_url, download_url) = presigned_uploader
            .upload_artifact(artifact_path, app_id, version)
            .await?;

        Ok((artifact_url, download_url))
    }
}

fn render_pokemon_ascii(f: &mut Frame, app: &TerminalApp, area: Rect) {
    // Pokemon ASCII art mới - nhỏ gọn hơn
    let pokemon_art = vec![
        "⠀⠀⠀⠀⠀⠀⣀⣠⣤⡔⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⡴⣧",
        "⠀⠀⣀⣤⣶⣿⣿⣿⣿⣏⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⡾⠁⣼",
        "⢠⣾⣿⣿⣿⣿⣿⣿⣿⣿⢷⣆⣤⣀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣰⡟⠀⠀⣿",
        "⣾⣿⣿⣟⢛⣛⣛⣛⣋⠭⠥⠿⣿⣿⣷⣤⠀⠀⠀⢀⣀⣀⣠⣀⡀⢿⡇⠀⣸⡇",
        "⣿⣿⣿⠻⢧⠙⢯⡀⠈⠉⠙⠛⠳⢦⣝⢿⣷⢠⣾⣿⣿⣿⣿⣿⣯⣬⡥⢰⡟⠀",
        "⢹⣯⣛⠯⢿⣾⣷⣍⡳⣤⣀⠀⠀⠀⠉⠳⣍⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡃⠀⠀",
        "⠈⠹⣿⣿⣾⣿⣿⣿⣿⣷⣭⣛⠷⢦⣤⣤⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⠀⠀",
        "⠀⠀⠈⠛⠿⣿⣿⣿⣿⣿⢋⣵⡾⣣⣤⣦⢻⣿⣿⣿⣻⠻⣿⣿⣿⡏⠖⢻⠀⠀",
        "⠀⠀⠀⠀⠀⠀⠈⠉⠉⣱⣿⡟⠼⣻⣿⣿⢸⣿⣿⡇⡛⠀⣿⣿⣿⣧⡠⣸⡇⠀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⢠⣿⣿⣷⢹⣿⣿⣿⣏⢿⣿⣷⣕⣧⣿⣿⣿⢿⣿⡿⠀⠀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠸⣿⣿⣿⣠⢿⣿⡟⣿⣷⣭⣻⠿⢿⠿⠷⢞⣫⣵⠿⠀⠀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⢀⣿⣿⣿⡟⣎⢿⣧⢿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡟⠀⠀⠀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠸⣿⣿⣿⠁⠻⣷⡝⣩⣿⣿⣿⣿⣿⣿⣿⠿⠁⠀⠀⠀⠀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⢻⣿⣿⠀⠀⠀⠀⣿⣿⣿⠉⠙⢻⢟⣿⡇⠀⠀⠀⠀⠀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢹⣿⣿⡀⠀⠸⣿⣿⡇⠀⠀⠀⠀⠀",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⢿⢿⡧⠀⠀⠈⠉⠀⠀⠀⠀⠀⠀",
    ];

    // Animated sparkles và effects
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let sparkle = if current_time % 1000 < 500 {
        "✨"
    } else {
        "⭐"
    };
    let lightning = if current_time % 800 < 400 {
        "⚡"
    } else {
        "💫"
    };

    // Create animated title
    let animated_title = format!(" {} POKEMON COMPANION {} ", sparkle, lightning);

    // Pokemon status based on theme
    let pokemon_status = match app.pokemon_theme.current_type {
        PokemonType::Electric => vec![
            format!("{} ⚡ EEVEE ⚡ {}", sparkle, sparkle),
            "Level: 42 🏆".to_string(),
            format!("HP: ████████░░ 85% {}", lightning),
            "MP: ██████████ 100% 💫".to_string(),
            "".to_string(),
            "Status Effects: 🔥".to_string(),
            "• Coding Boost ⚡".to_string(),
            "• Debug Vision 👁️".to_string(),
            "• Terminal Mastery 💻".to_string(),
            "".to_string(),
            "Moves Available:".to_string(),
            "• Thunder Deploy 🌩️".to_string(),
            "• Quick Build ⚡".to_string(),
            "• Log Stream 📡".to_string(),
            "• Ctrl+C Escape 🏃".to_string(),
        ],
        PokemonType::Fire => vec![
            format!("{} 🔥 CHARIZARD 🔥 {}", sparkle, sparkle),
            "Level: 45 🏆".to_string(),
            "HP: ██████████ 100% 🔥".to_string(),
            "MP: ████████░░ 90% 🌟".to_string(),
            "".to_string(),
            "Status Effects: 🔥".to_string(),
            "• Flame Compiler 🔥".to_string(),
            "• Hot Deploy 🚀".to_string(),
            "• Burn Bugs 🐛💥".to_string(),
        ],
        _ => vec![
            format!("{} ✨ MYSTICAL POKEMON ✨ {}", sparkle, sparkle),
            "Level: ?? 🎭".to_string(),
            "HP: ??????????".to_string(),
            "Status: Mysterious ❓".to_string(),
        ],
    };

    // Combine pokemon art with status
    let mut combined_content = Vec::new();

    // Add Pokemon ASCII art
    for (i, line) in pokemon_art.iter().enumerate() {
        if i < area.height.saturating_sub(8) as usize {
            combined_content.push(line.to_string());
        }
    }

    // Add separator
    combined_content.push("".to_string());
    combined_content.push("═══════════════════════════".to_string());
    combined_content.push("".to_string());

    // Add Pokemon status
    for line in pokemon_status {
        combined_content.push(line);
    }

    let pokemon_text = combined_content.join("\n");

    let pokemon_widget = Paragraph::new(pokemon_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(animated_title)
                .title_style(
                    app.pokemon_theme
                        .accent_style()
                        .add_modifier(Modifier::BOLD | Modifier::ITALIC),
                )
                .border_style(app.pokemon_theme.border_style()),
        )
        .style(app.pokemon_theme.info_style())
        .wrap(Wrap { trim: false })
        .scroll((0, 0)); // Keep at top

    f.render_widget(pokemon_widget, area);

    // Add floating sparkles if there's space
    if area.width > 10 && area.height > 10 {
        for (i, &(x, y)) in app.sparkle_positions.iter().enumerate().take(3) {
            let sparkle_x = area.x + (x % (area.width.saturating_sub(2)));
            let sparkle_y = area.y + (y % (area.height.saturating_sub(2)));

            if sparkle_x < area.right() && sparkle_y < area.bottom() {
                let sparkle_char = match i % 4 {
                    0 => "✨",
                    1 => "⭐",
                    2 => "💫",
                    _ => "🌟",
                };

                let sparkle_rect = Rect {
                    x: sparkle_x,
                    y: sparkle_y,
                    width: 1,
                    height: 1,
                };

                let sparkle_widget =
                    Paragraph::new(sparkle_char).style(app.pokemon_theme.accent_style());

                f.render_widget(Clear, sparkle_rect);
                f.render_widget(sparkle_widget, sparkle_rect);
            }
        }
    }
}
