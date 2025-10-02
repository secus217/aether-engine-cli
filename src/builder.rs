use crate::{AetherError, Result};
use flate2::write::GzEncoder;
use flate2::Compression;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use tar::Builder as TarBuilder;

#[derive(Debug, Deserialize)]
pub struct PackageJson {
    pub name: String,
    pub version: Option<String>,
    pub scripts: Option<std::collections::HashMap<String, String>>,
    pub dependencies: Option<std::collections::HashMap<String, String>>,
    pub engines: Option<Engines>,
}

#[derive(Debug, Deserialize)]
pub struct Engines {
    pub node: Option<String>,
    pub npm: Option<String>,
}

pub struct ProjectBuilder {
    project_path: PathBuf,
    package_json: PackageJson,
    output_callback: Option<Box<dyn Fn(&str) + Send + Sync>>,
}

impl ProjectBuilder {
    pub fn new<P: AsRef<Path>>(project_path: P) -> Result<Self> {
        let project_path = project_path.as_ref().to_path_buf();
        let package_json_path = project_path.join("package.json");

        if !package_json_path.exists() {
            return Err(AetherError::invalid_project(
                "No package.json found in project directory",
            ));
        }

        let package_json_content = std::fs::read_to_string(&package_json_path)?;
        let package_json: PackageJson = serde_json::from_str(&package_json_content)?;

        Ok(Self {
            project_path,
            package_json,
            output_callback: None,
        })
    }

    pub fn get_app_name(&self) -> &str {
        &self.package_json.name
    }

    pub fn get_version(&self) -> String {
        self.package_json
            .version
            .clone()
            .unwrap_or_else(|| "1.0.0".to_string())
    }

    pub fn with_output_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.output_callback = Some(Box::new(callback));
        self
    }

    fn output(&self, message: &str) {
        if let Some(ref callback) = self.output_callback {
            callback(message);
        } else {
            println!("{}", message);
        }
    }

    pub fn get_node_version(&self) -> String {
        self.package_json
            .engines
            .as_ref()
            .and_then(|e| e.node.as_deref())
            .unwrap_or("20")
            .to_string()
    }

    pub fn detect_runtime(&self) -> String {
        let node_version = self.get_node_version();
        // Extract major version number
        if let Some(major) = node_version
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<u32>()
            .ok()
        {
            format!("node:{}", major)
        } else {
            "node:20".to_string()
        }
    }

    pub async fn build(&self, output_path: Option<PathBuf>) -> Result<PathBuf> {
        self.output("🔧 Building NodeJS application...");

        // Install dependencies
        self.install_dependencies().await?;

        // Run build script if available
        self.run_build_script().await?;

        // Create artifact
        let artifact_path = output_path.unwrap_or_else(|| {
            std::env::temp_dir().join(format!("{}.tar.gz", self.get_app_name()))
        });

        self.create_artifact(&artifact_path).await?;

        self.output(&format!("✅ Build completed: {}", artifact_path.display()));
        Ok(artifact_path)
    }

    async fn install_dependencies(&self) -> Result<()> {
        self.output("📦 Installing dependencies...");

        // Check if node_modules exists and has content
        let node_modules = self.project_path.join("node_modules");
        if node_modules.exists() && std::fs::read_dir(&node_modules)?.count() > 0 {
            self.output("📦 Dependencies already installed, skipping...");
            return Ok(());
        }

        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message("Installing dependencies...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));

        // Determine package manager
        let package_manager = self.detect_package_manager();

        let mut cmd = Command::new(&package_manager);
        cmd.current_dir(&self.project_path);

        match package_manager.as_str() {
            "npm" => {
                cmd.args(&["install", "--production"]);
            }
            "yarn" => {
                cmd.args(&["install", "--production"]);
            }
            "pnpm" => {
                cmd.args(&["install", "--prod"]);
            }
            _ => {
                cmd.args(&["install", "--production"]);
            }
        }

        let output = cmd.output()?;
        pb.finish_and_clear();

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(AetherError::build(format!(
                "Failed to install dependencies: {}",
                error
            )));
        }

        self.output("✅ Dependencies installed successfully");
        Ok(())
    }

    fn detect_package_manager(&self) -> String {
        // Check for lock files to determine package manager
        if self.project_path.join("yarn.lock").exists() {
            "yarn".to_string()
        } else if self.project_path.join("pnpm-lock.yaml").exists() {
            "pnpm".to_string()
        } else {
            "npm".to_string()
        }
    }

    async fn run_build_script(&self) -> Result<()> {
        let scripts = match &self.package_json.scripts {
            Some(scripts) => scripts,
            None => return Ok(()), // No scripts defined
        };

        // Check for common build script names
        let build_script = scripts
            .get("build")
            .or_else(|| scripts.get("compile"))
            .or_else(|| scripts.get("prepare"));

        if let Some(_script) = build_script {
            println!("🏗️  Running build script...");

            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );
            pb.set_message("Building application...");
            pb.enable_steady_tick(std::time::Duration::from_millis(100));

            let package_manager = self.detect_package_manager();
            let mut cmd = Command::new(&package_manager);
            cmd.current_dir(&self.project_path);

            match package_manager.as_str() {
                "npm" => {
                    cmd.args(&["run", "build"]);
                }
                "yarn" => {
                    cmd.args(&["run", "build"]);
                }
                "pnpm" => {
                    cmd.args(&["run", "build"]);
                }
                _ => {
                    cmd.args(&["run", "build"]);
                }
            }

            let output = cmd.output()?;
            pb.finish_and_clear();

            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                return Err(AetherError::build(format!(
                    "Build script failed: {}",
                    error
                )));
            }

            self.output("✅ Build script completed successfully");
        }

        Ok(())
    }

    async fn create_artifact(&self, output_path: &Path) -> Result<()> {
        self.output("📦 Creating deployment artifact...");

        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message("Packaging application...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));

        let tar_gz = File::create(output_path)?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = TarBuilder::new(enc);

        // Add essential files and directories
        self.add_to_archive(&mut tar, "package.json")?;

        // Add package-lock.json or yarn.lock if they exist
        if self.project_path.join("package-lock.json").exists() {
            self.add_to_archive(&mut tar, "package-lock.json")?;
        }
        if self.project_path.join("yarn.lock").exists() {
            self.add_to_archive(&mut tar, "yarn.lock")?;
        }
        if self.project_path.join("pnpm-lock.yaml").exists() {
            self.add_to_archive(&mut tar, "pnpm-lock.yaml")?;
        }

        // Add node_modules
        if self.project_path.join("node_modules").exists() {
            self.add_directory_to_archive(&mut tar, "node_modules")?;
        }

        // Add source files (common patterns)
        for pattern in &["src", "lib", "dist", "build", "public", "views"] {
            let dir_path = self.project_path.join(pattern);
            if dir_path.exists() && dir_path.is_dir() {
                self.add_directory_to_archive(&mut tar, pattern)?;
            }
        }

        // Add common files
        for file in &["index.js", "server.js", "app.js", "main.js", ".env.example"] {
            let file_path = self.project_path.join(file);
            if file_path.exists() && file_path.is_file() {
                self.add_to_archive(&mut tar, file)?;
            }
        }

        tar.finish()?;
        pb.finish_and_clear();

        self.output("✅ Artifact created successfully");
        Ok(())
    }

    fn add_to_archive(
        &self,
        tar: &mut TarBuilder<GzEncoder<File>>,
        relative_path: &str,
    ) -> Result<()> {
        let full_path = self.project_path.join(relative_path);
        if full_path.exists() {
            tar.append_path_with_name(&full_path, relative_path)?;
        }
        Ok(())
    }

    fn add_directory_to_archive(
        &self,
        tar: &mut TarBuilder<GzEncoder<File>>,
        dir_name: &str,
    ) -> Result<()> {
        let dir_path = self.project_path.join(dir_name);
        if dir_path.exists() && dir_path.is_dir() {
            tar.append_dir_all(dir_name, &dir_path)?;
        }
        Ok(())
    }

    // Public getters for private fields
    pub fn get_project_path(&self) -> &PathBuf {
        &self.project_path
    }

    pub fn get_package_json(&self) -> &PackageJson {
        &self.package_json
    }
}
