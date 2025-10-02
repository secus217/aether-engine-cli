use crate::Result;
use console::{style, Style};
use std::path::Path;

pub fn success_style() -> Style {
    Style::new().green().bold()
}

pub fn error_style() -> Style {
    Style::new().red().bold()
}

pub fn info_style() -> Style {
    Style::new().blue().bold()
}

pub fn warning_style() -> Style {
    Style::new().yellow().bold()
}

pub fn print_success(message: &str) {
    println!(
        "{} {}",
        style("✅").green(),
        success_style().apply_to(message)
    );
}

pub fn print_error(message: &str) {
    println!("{} {}", style("❌").red(), error_style().apply_to(message));
}

pub fn print_info(message: &str) {
    println!("{} {}", style("ℹ️").blue(), info_style().apply_to(message));
}

pub fn print_warning(message: &str) {
    println!(
        "{} {}",
        style("⚠️").yellow(),
        warning_style().apply_to(message)
    );
}

pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

pub fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m {}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
    }
}

pub fn confirm(message: &str) -> Result<bool> {
    use dialoguer::Confirm;

    Ok(Confirm::new()
        .with_prompt(message)
        .default(false)
        .interact()?)
}

pub fn select_from_list<T: std::fmt::Display>(prompt: &str, items: &[T]) -> Result<usize> {
    use dialoguer::Select;

    Ok(Select::new()
        .with_prompt(prompt)
        .items(items)
        .default(0)
        .interact()?)
}

pub fn find_project_root(start_dir: &Path) -> Option<std::path::PathBuf> {
    let mut current = start_dir;

    loop {
        // Check for package.json
        if current.join("package.json").exists() {
            return Some(current.to_path_buf());
        }

        // Move to parent directory
        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            break;
        }
    }

    None
}

pub fn validate_app_name(name: &str) -> Result<()> {
    // Check if name is valid (lowercase, alphanumeric, hyphens)
    if name.is_empty() {
        return Err(crate::AetherError::invalid_project(
            "App name cannot be empty",
        ));
    }

    if name.len() > 63 {
        return Err(crate::AetherError::invalid_project(
            "App name too long (max 63 characters)",
        ));
    }

    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(crate::AetherError::invalid_project(
            "App name must contain only lowercase letters, numbers, and hyphens",
        ));
    }

    if name.starts_with('-') || name.ends_with('-') {
        return Err(crate::AetherError::invalid_project(
            "App name cannot start or end with a hyphen",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3661), "1h 1m");
    }

    #[test]
    fn test_validate_app_name() {
        assert!(validate_app_name("my-app").is_ok());
        assert!(validate_app_name("myapp123").is_ok());
        assert!(validate_app_name("").is_err());
        assert!(validate_app_name("My-App").is_err());
        assert!(validate_app_name("-invalid").is_err());
        assert!(validate_app_name("invalid-").is_err());
    }
}
