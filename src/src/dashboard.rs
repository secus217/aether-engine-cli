                    use crate::{api::ApiClient, config::Config, utils, Result};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Paragraph, Wrap, Clear,
    },
    Frame, Terminal,
};
use std::{
    io::{self, Write},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub struct App {
    client: ApiClient,
    should_quit: bool,
    // Terminal state
    command_input: String,
    command_history: Vec<String>,
    history_index: Option<usize>,
    output_lines: Vec<String>,
    current_dir: std::path::PathBuf,
    cursor_position: usize,
}

impl App {
    pub fn new(client: ApiClient) -> Self {
        let mut app_list_state = ListState::default();
        app_list_state.select(Some(0));
        
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let dir_contents = Self::read_directory(&current_dir).unwrap_or_default();
        
        Self {
            client,
            tab_index: 0,
            app_list_state,
            selected_app_id: None,
            apps: Vec::new(),
            logs: String::new(),
            should_quit: false,
            last_refresh: Instant::now(),
            command_mode: false,
            command_input: String::new(),
            command_output: String::new(),
            current_dir,
            dir_contents,
        }
    }

    fn read_directory(path: &std::path::Path) -> io::Result<Vec<std::fs::DirEntry>> {
        let mut entries: Vec<_> = std::fs::read_dir(path)?
            .filter_map(|entry| entry.ok())
            .collect();
        
        entries.sort_by(|a, b| {
            let a_is_dir = a.file_type().map_or(false, |ft| ft.is_dir());
            let b_is_dir = b.file_type().map_or(false, |ft| ft.is_dir());
            
            // Directories first, then files
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.file_name().cmp(&b.file_name()),
            }
        });
        
        Ok(entries)
    }

    pub async fn refresh_data(&mut self) -> Result<()> {
        self.apps = self.client.list_applications().await?;
        
        if let Some(selected) = self.app_list_state.selected() {
            if selected < self.apps.len() {
                self.selected_app_id = Some(self.apps[selected].id);
                
                // Refresh logs for selected app
                if let Ok(logs) = self.client.get_logs(self.apps[selected].id, Some(50)).await {
                    self.logs = logs;
                }
            }
        }
        
        self.last_refresh = Instant::now();
        Ok(())
    }

    fn next_tab(&mut self) {
        self.tab_index = (self.tab_index + 1) % 3;
    }

    fn previous_tab(&mut self) {
        self.tab_index = if self.tab_index > 0 { self.tab_index - 1 } else { 2 };
    }

    fn next_app(&mut self) {
        if self.apps.is_empty() {
            return;
        }
        
        let i = match self.app_list_state.selected() {
            Some(i) => {
                if i >= self.apps.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.app_list_state.select(Some(i));
        
        if i < self.apps.len() {
            self.selected_app_id = Some(self.apps[i].id);
        }
    }

    fn previous_app(&mut self) {
        if self.apps.is_empty() {
            return;
        }
        
        let i = match self.app_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.apps.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.app_list_state.select(Some(i));
        
        if i < self.apps.len() {
            self.selected_app_id = Some(self.apps[i].id);
        }
    }
    
    fn toggle_command_mode(&mut self) {
        self.command_mode = !self.command_mode;
        if !self.command_mode {
            self.command_input.clear();
        }
    }
    
    fn add_char_to_command(&mut self, c: char) {
        self.command_input.push(c);
    }
    
    fn remove_char_from_command(&mut self) {
        self.command_input.pop();
    }
    
    async fn execute_command(&mut self) -> Result<()> {
        let command = self.command_input.trim().to_string();
        if command.is_empty() {
            return Ok(());
        }
        
        // Parse command and execute
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }
        
        match parts[0] {
            "pwd" => {
                self.command_output = format!("📂 Current directory: {}", self.current_dir.display());
            },
            "ls" => {
                let mut output = String::new();
                output.push_str("📁 Directory contents:\n");
                
                if self.dir_contents.is_empty() {
                    output.push_str("   (empty directory)");
                } else {
                    for (i, entry) in self.dir_contents.iter().enumerate() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let is_dir = entry.file_type().map_or(false, |ft| ft.is_dir());
                        
                        if is_dir {
                            output.push_str(&format!("   📁 {}/\n", name));
                        } else {
                            let emoji = if name.ends_with(".js") || name.ends_with(".ts") { "📜" }
                                       else if name.ends_with(".json") { "📋" }
                                       else if name.ends_with(".md") { "📖" }
                                       else { "📄" };
                            output.push_str(&format!("   {} {}\n", emoji, name));
                        }
                        
                        if i >= 20 {  // Limit display
                            output.push_str(&format!("   ... and {} more items", self.dir_contents.len() - i - 1));
                            break;
                        }
                    }
                }
                self.command_output = output.trim_end().to_string();
            },
            "cd" => {
                if parts.len() < 2 {
                    self.command_output = "💖 Usage: cd <directory>\n   💡 Tip: Use 'cd ..' to go up, 'cd ~' for home".to_string();
                    return Ok(());
                }
                
                let target = parts[1];
                let new_path = if target == "~" {
                    dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"))
                } else if target == ".." {
                    self.current_dir.parent().unwrap_or(&self.current_dir).to_path_buf()
                } else if target.starts_with('/') {
                    std::path::PathBuf::from(target)
                } else {
                    self.current_dir.join(target)
                };
                
                if new_path.exists() && new_path.is_dir() {
                    self.current_dir = new_path;
                    self.dir_contents = Self::read_directory(&self.current_dir).unwrap_or_default();
                    self.command_output = format!("✨ Changed to: {}", self.current_dir.display());
                } else {
                    self.command_output = format!("❌ Directory not found: {}", target);
                }
            },
            "deploy" => {
                // Check if we're in a valid project directory
                let package_json = self.current_dir.join("package.json");
                if !package_json.exists() {
                    self.command_output = "❌ No package.json found in current directory!\n💡 Use 'cd' to navigate to a Node.js project first.".to_string();
                    return Ok(());
                }
                
                let app_name = if parts.len() >= 2 {
                    parts[1].to_string()
                } else {
                    // Try to get name from package.json
                    match self.get_app_name_from_package_json() {
                        Some(name) => name,
                        None => {
                            self.command_output = "❌ Could not determine app name. Usage: deploy <app-name>".to_string();
                            return Ok(());
                        }
                    }
                };
                
                // Actually perform the deployment
                match self.perform_deploy(&app_name).await {
                    Ok(message) => {
                        self.command_output = format!("✅ {}", message);
                    }
                    Err(e) => {
                        self.command_output = format!("❌ Deploy failed: {}", e);
                    }
                }
            },
            "delete" => {
                if parts.len() < 2 {
                    self.command_output = "Usage: delete <app-name>".to_string();
                    return Ok(());
                }
                
                // Find app by name
                if let Some(app) = self.apps.iter().find(|a| a.name == parts[1]) {
                    match self.client.delete_application(app.id).await {
                        Ok(_) => {
                            self.command_output = format!("✅ Application '{}' deleted successfully", parts[1]);
                            // Refresh apps list
                            self.refresh_data().await?;
                        },
                        Err(e) => {
                            self.command_output = format!("❌ Failed to delete '{}': {}", parts[1], e);
                        }
                    }
                } else {
                    self.command_output = format!("❌ Application '{}' not found", parts[1]);
                }
            },
            "list" => {
                if self.apps.is_empty() {
                    self.command_output = "No applications found".to_string();
                } else {
                    let mut output = String::new();
                    output.push_str("📋 Applications:\n");
                    for (i, app) in self.apps.iter().enumerate() {
                        output.push_str(&format!(
                            "{}. {} ({}) - {}\n",
                            i + 1,
                            app.name,
                            app.runtime,
                            app.created_at.format("%Y-%m-%d %H:%M")
                        ));
                    }
                    self.command_output = output;
                }
            },
            "refresh" | "r" => {
                match self.refresh_data().await {
                    Ok(_) => {
                        self.command_output = "✅ Data refreshed successfully".to_string();
                    },
                    Err(e) => {
                        self.command_output = format!("❌ Refresh failed: {}", e);
                    }
                }
            },
            "help" | "h" => {
                self.command_output = r#"Available commands:
  list, ls             - List all applications
  deploy <app-name>    - Deploy application
  delete <app-name>    - Delete application
  refresh, r           - Refresh data
  help, h              - Show this help
  clear                - Clear output
  quit, q              - Exit dashboard"#.to_string();
            },
            "clear" => {
                self.command_output.clear();
            },
            "quit" | "q" => {
                self.should_quit = true;
            },
            _ => {
                self.command_output = format!("Unknown command: {}. Type 'help' for available commands.", parts[0]);
            }
        }
        
        self.command_input.clear();
        self.command_mode = false;
        Ok(())
    }

    fn get_app_name_from_package_json(&self) -> Option<String> {
        let package_json_path = self.current_dir.join("package.json");
        if !package_json_path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(package_json_path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;
        
        json.get("name")?.as_str().map(|s| s.to_string())
    }

    fn open_selected_app_url(&mut self) {
        if let Some(selected) = self.app_list_state.selected() {
            if selected < self.apps.len() {
                let app = &self.apps[selected];
                if let Some(url) = &app.deployment_url {
                    self.command_output = format!("🌐 Opening URL: {}", url);
                    
                    // Try to open URL in browser
                    #[cfg(target_os = "linux")]
                    {
                        let _ = std::process::Command::new("xdg-open")
                            .arg(url)
                            .spawn();
                    }
                    #[cfg(target_os = "macos")]
                    {
                        let _ = std::process::Command::new("open")
                            .arg(url)
                            .spawn();
                    }
                    #[cfg(target_os = "windows")]
                    {
                        let _ = std::process::Command::new("cmd")
                            .args(&["/C", "start", url])
                            .spawn();
                    }
                } else {
                    self.command_output = format!("❌ No deployment URL available for '{}'", app.name);
                }
            }
        }
    }
    
    async fn perform_deploy(&mut self, app_name: &str) -> Result<String> {
        // Create builder for the current directory
        let builder = ProjectBuilder::new(self.current_dir.clone())?;
        let app_runtime = builder.detect_runtime();
        let version = builder.get_version();
        
        // Check if app exists, if not create it
        let existing_app = self.client.list_applications().await?
            .into_iter()
            .find(|app| app.name == app_name);
            
        let app = if let Some(existing_app) = existing_app {
            existing_app
        } else {
            // Create new application
            let create_request = crate::api::CreateAppRequest {
                name: app_name.to_string(),
                description: Some("NodeJS application deployed via AetherEngine CLI Dashboard 💖".to_string()),
                runtime: app_runtime.clone(),
            };
            
            self.client.create_application(create_request).await?
        };
        
        // Build the application
        let artifact_path = builder.build(None).await?;
        
        // Upload to S3
        let s3_uploader = S3Uploader::new().await?;
        let (artifact_url, presigned_url) = s3_uploader
            .upload_artifact(&artifact_path, app.id, &version)
            .await?;
            
        // Deploy the application with presigned URL
        let deployment = self.client
            .deploy_application(app.id, version.clone(), artifact_url.clone(), Some(presigned_url.clone()))
            .await?;
            
        // Clean up temporary artifact
        let _ = std::fs::remove_file(&artifact_path);
        
        // Refresh apps list
        self.refresh_data().await?;
        
        Ok(format!(
            "Successfully deployed '{}' v{} 🎉\n🆔 App ID: {}\n🚀 Deployment ID: {}\n📦 Artifact: {}\n🔗 Download: {}",
            app_name, version, app.id, deployment.id, artifact_url, presigned_url
        ))
    }
}

pub async fn run_dashboard() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let config = Config::load()?;
    let client = ApiClient::new(config.api_endpoint, config.auth_token)?;
    let mut app = App::new(client);

    // Initial data load
    if let Err(e) = app.refresh_data().await {
        utils::print_error(&format!("Failed to load data: {}", e));
    }

    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        // Handle events with timeout for auto-refresh
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if app.command_mode {
                        // Command input mode
                        match key.code {
                            KeyCode::Enter => {
                                if let Err(e) = app.execute_command().await {
                                    app.command_output = format!("❌ Command error: {}", e);
                                }
                            }
                            KeyCode::Esc => {
                                app.toggle_command_mode();
                            }
                            KeyCode::Backspace => {
                                app.remove_char_from_command();
                            }
                            KeyCode::Char(c) => {
                                app.add_char_to_command(c);
                            }
                            _ => {}
                        }
                    } else {
                        // Normal navigation mode
                        match key.code {
                            KeyCode::Char('q') => {
                                app.should_quit = true;
                            }
                            KeyCode::Char(':') => {
                                app.toggle_command_mode();
                            }
                            KeyCode::Tab => {
                                app.next_tab();
                            }
                            KeyCode::BackTab => {
                                app.previous_tab();
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.next_app();
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.previous_app();
                            }
                            KeyCode::Char('r') => {
                                if let Err(e) = app.refresh_data().await {
                                    // Handle error silently for now
                                    eprintln!("Refresh error: {}", e);
                                }
                            }
                            KeyCode::Enter => {
                                // Open deployment URL if available
                                if app.tab_index == 0 { // Applications tab
                                    app.open_selected_app_url();
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Auto-refresh every 10 seconds
        if app.last_refresh.elapsed() > Duration::from_secs(10) {
            if let Err(_e) = app.refresh_data().await {
                // Handle error silently
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Create layout
    let main_layout = if app.command_mode || !app.command_output.is_empty() {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(0),     // Main content
                Constraint::Length(5),  // Command area
                Constraint::Length(3),  // Footer
            ])
            .split(size)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Footer
            ])
            .split(size)
    };
    
    let chunks = if app.command_mode || !app.command_output.is_empty() {
        main_layout
    } else {
        main_layout
    };

    // Kawaii Header with title and current directory
    let current_dir_display = app.current_dir.to_string_lossy();
    let header_title = format!("💖 AetherEngine Super Kawaii Dashboard ✨ 📁 {}", current_dir_display);
    let header = Block::default()
        .borders(Borders::ALL)
        .title(header_title)
        .title_style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
        .title_alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);

    let tab_titles = vec!["� Apps", "� Deploy", "� Files"];
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title("🌸 Navigation 🌸"))
        .select(app.tab_index)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .bg(Color::Magenta)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    // Main content area
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(chunks[1]);

    f.render_widget(tabs, main_chunks[0]);

        // Content based on selected tab
    match app.tab_index {
        0 => render_applications(f, main_chunks[1], app),
        1 => render_deployments(f, main_chunks[1], app),
        2 => render_files(f, main_chunks[1], app),
        _ => {}
    }

    // Command area (if in command mode or has output)
    if app.command_mode || !app.command_output.is_empty() {
        render_command_area(f, chunks[2], app);
    }

    // Footer with help
    if app.command_mode {
        let command_input = Paragraph::new(format!(": {}", app.command_input))
            .block(Block::default().borders(Borders::ALL).title("Command Mode (ESC to cancel)"))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(command_input, chunks[2]);
    } else {
    let help_text = if app.command_mode {
        vec![
            Line::from(vec![
                Span::styled("Command Mode", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(" | "),
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::raw(": Execute | "),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::raw(": Cancel | Type 'help' for commands"),
            ]),
        ]
    } else {
        vec![
            Line::from(vec![
                Span::styled("💕 ", Style::default().fg(Color::Magenta)),
                Span::styled("Tab", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(": Switch tabs ✨ "),
                Span::styled("↑↓", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(": Navigate 🎯 "),
                Span::styled("Enter", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(": Open URL 🌐 "),
                Span::styled(":", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(": Command 📝 "),
                Span::styled("r", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
                Span::raw(": Refresh 🔄 "),
                Span::styled("q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::raw(": Quit 👋"),
            ]),
        ]
    };        let footer = Paragraph::new(help_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("🌸 Controls 🌸")
                .title_style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)))
            .alignment(Alignment::Center);
        f.render_widget(footer, chunks[2]);
    }
}

fn render_applications(f: &mut Frame, area: Rect, app: &mut App) {
    if app.apps.is_empty() {
        let empty = Paragraph::new("No applications found\n\nPress 'r' to refresh or deploy an app with 'aether deploy'")
            .block(Block::default().borders(Borders::ALL).title("Applications"))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .apps
        .iter()
        .enumerate()
        .map(|(i, app_item)| {
            let style = if Some(i) == app.app_list_state.selected() {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let url_display = if let Some(url) = &app_item.deployment_url {
                format!(" 🌐 {}", url)
            } else {
                " ❌ No URL".to_string()
            };

            let content = format!(
                "📦 {} | {} | Created: {}{}",
                app_item.name,
                app_item.runtime,
                app_item.created_at.format("%Y-%m-%d %H:%M"),
                url_display
            );
            
            ListItem::new(content).style(style)
        })
        .collect();

    let apps_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Applications"))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("→ ");

    f.render_stateful_widget(apps_list, area, &mut app.app_list_state);
}

fn render_deployments(f: &mut Frame, area: Rect, app: &App) {
    let deployment_info = if let Some(selected) = app.app_list_state.selected() {
        if selected < app.apps.len() {
            format!("Deployments for: {}", app.apps[selected].name)
        } else {
            "No app selected".to_string()
        }
    } else {
        "No app selected".to_string()
    };

    let deployments = Paragraph::new(format!("{}\n\n(Deployment details coming soon...)", deployment_info))
        .block(Block::default().borders(Borders::ALL).title("Deployments"))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    
    f.render_widget(deployments, area);
}

fn render_files(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(70), // File browser
            Constraint::Percentage(30), // Command output
        ])
        .split(area);

    // File browser
    let mut files: Vec<ListItem> = Vec::new();
    
    // Add parent directory option if not at root
    if app.current_dir.parent().is_some() {
        files.push(ListItem::new(Line::from(vec![
            Span::styled("📁 ..", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        ])));
    }
    
    // Add directory contents
    for entry in &app.dir_contents {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().map_or(false, |ft| ft.is_dir());
        
        let (emoji, style) = if is_dir {
            ("📁", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        } else if name.ends_with(".js") || name.ends_with(".ts") {
            ("⚡", Style::default().fg(Color::Yellow))
        } else if name.ends_with(".json") {
            ("📋", Style::default().fg(Color::Green))
        } else if name.ends_with(".md") {
            ("📖", Style::default().fg(Color::Blue))
        } else if name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".gif") {
            ("🖼️", Style::default().fg(Color::Magenta))
        } else {
            ("📄", Style::default().fg(Color::White))
        };
        
        let display_name = if is_dir { format!("{}/", name) } else { name };
        files.push(ListItem::new(Line::from(vec![
            Span::styled(format!("{} {}", emoji, display_name), style)
        ])));
    }
    
    if files.is_empty() {
        files.push(ListItem::new(Line::from(vec![
            Span::styled("💔 Empty directory", Style::default().fg(Color::Red))
        ])));
    }
    
    let files_list = List::new(files)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!("📁 File Explorer - {} 💖", app.current_dir.display()))
            .title_style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)))
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        );
    
    f.render_widget(files_list, chunks[0]);

    // Command output area
    if !app.command_output.is_empty() {
        let command_output = Paragraph::new(app.command_output.clone())
            .block(Block::default()
                .borders(Borders::ALL)
                .title("💬 Command Output")
                .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(command_output, chunks[1]);
    } else {
        let help_text = "💡 Available commands:\n\n\
            📂 cd <dir>     - Change directory\n\
            📋 ls          - List contents\n\
            📍 pwd         - Show current path\n\
            🚀 deploy      - Deploy current project\n\
            💖 Type ':' to enter command mode";
        
        let help = Paragraph::new(help_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("✨ Kawaii Help ✨")
                .title_style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(help, chunks[1]);
    }
}

async fn execute_command_in_dashboard(app: &mut App) -> crate::Result<String> {
    let command = app.command_input.trim();
    
    match command {
        "refresh" | "r" => {
            app.refresh_data().await?;
            Ok("Data refreshed successfully".to_string())
        }
        "list" | "ls" => {
            let count = app.apps.len();
            Ok(format!("Found {} applications", count))
        }
        cmd if cmd.starts_with("logs ") => {
            let app_name = cmd.strip_prefix("logs ").unwrap().trim();
            if let Some(app_item) = app.apps.iter().find(|a| a.name == app_name) {
                match app.client.get_logs(app_item.id, Some(20)).await {
                    Ok(logs) => {
                        app.logs = logs.clone();
                        Ok(format!("Loaded logs for '{}'", app_name))
                    }
                    Err(e) => Err(e),
                }
            } else {
                Ok(format!("Application '{}' not found", app_name))
            }
        }
        cmd if cmd.starts_with("status ") => {
            let app_name = cmd.strip_prefix("status ").unwrap().trim();
            if let Some(app_item) = app.apps.iter().find(|a| a.name == app_name) {
                Ok(format!("App: {} | Runtime: {} | Created: {}", 
                    app_item.name, 
                    app_item.runtime,
                    app_item.created_at.format("%Y-%m-%d %H:%M")
                ))
            } else {
                Ok(format!("Application '{}' not found", app_name))
            }
        }
        "help" | "h" => {
            Ok("Available commands:\n  refresh/r - Refresh data\n  list/ls - List applications\n  logs <app> - Show logs\n  status <app> - Show app status\n  help/h - Show this help\n  quit/q - Quit dashboard".to_string())
        }
        "quit" | "q" => {
            app.should_quit = true;
            Ok("Quitting dashboard...".to_string())
        }
        "" => Ok("Enter a command. Type 'help' for available commands.".to_string()),
        _ => Ok(format!("Unknown command: '{}'. Type 'help' for available commands.", command))
    }
}

fn render_command_area(f: &mut Frame, area: Rect, app: &App) {
    if app.command_mode {
        // Show command input
        let command_input = Paragraph::new(format!(": {}", app.command_input))
            .block(Block::default().borders(Borders::ALL).title("Command Mode (ESC to cancel)"))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(command_input, area);
    } else if !app.command_output.is_empty() {
        // Show command output
        let command_output = Paragraph::new(app.command_output.as_str())
            .block(Block::default().borders(Borders::ALL).title("Command Output"))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::White));
        f.render_widget(command_output, area);
    }
}