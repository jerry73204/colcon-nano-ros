//! cargo-ros2 library
//!
//! This library provides functionality for building ROS 2 Rust projects,
//! including binding generation, build orchestration, and ament installation.
//!
//! # Public API
//!
//! This library exposes a high-level API for:
//! - Generating Rust bindings for ROS 2 interface packages
//! - Installing packages to ament layout
//! - Cleaning generated bindings and cache
//!
//! # Example
//!
//! ```no_run
//! use cargo_ros2::{BindgenConfig, generate_bindings};
//! use std::path::PathBuf;
//!
//! let config = BindgenConfig {
//!     package_name: "std_msgs".to_string(),
//!     package_path: None,
//!     output_dir: PathBuf::from("target/ros2_bindings"),
//!     verbose: false,
//! };
//!
//! generate_bindings(config).expect("Failed to generate bindings");
//! ```

pub mod ament_installer;
pub mod cache;
pub mod config_patcher;
pub mod dependency_parser;
pub mod package_discovery;
pub mod workflow;

use eyre::{eyre, Result, WrapErr};
use std::path::{Path, PathBuf};

/// Configuration for binding generation
#[derive(Debug, Clone)]
pub struct BindgenConfig {
    /// ROS package name (e.g., "std_msgs")
    pub package_name: String,
    /// Optional direct path to package share directory (bypasses ament index)
    pub package_path: Option<PathBuf>,
    /// Output directory for generated bindings
    pub output_dir: PathBuf,
    /// Enable verbose output
    pub verbose: bool,
}

/// Configuration for ament installation
#[derive(Debug, Clone)]
pub struct InstallConfig {
    /// Project root directory (where Cargo.toml is located)
    pub project_root: PathBuf,
    /// Install base directory (install/<package>/)
    pub install_base: PathBuf,
    /// Build profile: "debug" or "release"
    pub profile: String,
    /// Enable verbose output
    pub verbose: bool,
}

/// Generate Rust bindings for a ROS 2 interface package
///
/// This function generates Rust bindings for messages, services, and actions
/// defined in a ROS 2 interface package.
///
/// # Arguments
///
/// * `config` - Configuration for binding generation
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err` if binding generation fails
///
/// # Example
///
/// ```no_run
/// use cargo_ros2::{BindgenConfig, generate_bindings};
/// use std::path::PathBuf;
///
/// let config = BindgenConfig {
///     package_name: "geometry_msgs".to_string(),
///     package_path: None,
///     output_dir: PathBuf::from("target/ros2_bindings"),
///     verbose: true,
/// };
///
/// generate_bindings(config)?;
/// # Ok::<(), eyre::Report>(())
/// ```
pub fn generate_bindings(config: BindgenConfig) -> Result<()> {
    use rosidl_bindgen::ament::{AmentIndex, Package};
    use rosidl_bindgen::generator;

    if config.verbose {
        eprintln!("Generating bindings for {}...", config.package_name);
    }

    // Get package either from path or ament index
    let package = if let Some(share_path) = config.package_path {
        Package::from_share_dir(share_path)?
    } else {
        let index =
            AmentIndex::from_env().wrap_err("Failed to load ament index (is ROS 2 sourced?)")?;
        index
            .find_package(&config.package_name)
            .ok_or_else(|| eyre!("Package '{}' not found in ament index", config.package_name))?
            .clone()
    };

    // Generate bindings using rosidl-bindgen library
    let result = generator::generate_package(&package, &config.output_dir)?;

    if config.verbose {
        eprintln!(
            "✓ Generated {} messages, {} services, {} actions for {}",
            result.message_count, result.service_count, result.action_count, config.package_name
        );
    }

    Ok(())
}

/// Install package binaries and libraries to ament layout
///
/// This function installs compiled binaries, libraries, and creates necessary
/// ament package markers for ROS 2 integration.
///
/// # Arguments
///
/// * `config` - Configuration for installation
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err` if installation fails
///
/// # Example
///
/// ```no_run
/// use cargo_ros2::{InstallConfig, install_to_ament};
/// use std::path::PathBuf;
///
/// let config = InstallConfig {
///     project_root: PathBuf::from("/path/to/project"),
///     install_base: PathBuf::from("install/my_package"),
///     profile: "release".to_string(),
///     verbose: true,
/// };
///
/// install_to_ament(config)?;
/// # Ok::<(), eyre::Report>(())
/// ```
pub fn install_to_ament(config: InstallConfig) -> Result<()> {
    use crate::ament_installer::{is_library_package, AmentInstaller};
    use cargo_metadata::MetadataCommand;
    use std::env;

    if config.verbose {
        eprintln!("Installing package to ament layout...");
    }

    // Save current directory and change to project root
    let original_dir = env::current_dir()?;
    env::set_current_dir(&config.project_root)?;

    // Read package metadata
    // Pass --config flag to use workspace-level config
    let mut metadata_cmd = MetadataCommand::new();

    // Look for ros2_cargo_config.toml in workspace build directory
    // Walk up from project_root to find build/ directory
    let mut search_dir = Some(config.project_root.parent());
    let mut config_file_path = None;
    while let Some(Some(dir)) = search_dir {
        // Check for build/ros2_cargo_config.toml
        let potential_build = dir.join("build");
        if potential_build.exists() {
            let config_file = potential_build.join("ros2_cargo_config.toml");
            if config_file.exists() {
                config_file_path = Some(config_file);
                break;
            }
        }
        search_dir = Some(dir.parent());
    }

    // Add --config flag if found
    if let Some(config_path) = config_file_path {
        metadata_cmd.other_options(vec![
            "--config".to_string(),
            config_path.display().to_string(),
        ]);
    }

    let metadata = metadata_cmd
        .exec()
        .wrap_err("Failed to read Cargo metadata")?;

    let root_package = metadata
        .root_package()
        .ok_or_else(|| eyre!("No root package found in Cargo.toml"))?;

    let package_name = root_package.name.clone();

    // Check if this is a library-only package
    let is_lib_only = is_library_package(&config.project_root)?;

    // Create installer
    let installer = AmentInstaller::new(
        config.install_base.clone(),
        package_name,
        config.project_root.clone(),
        config.verbose,
        config.profile.clone(),
    );

    // Run installation
    let result = installer.install(is_lib_only);

    // Restore original directory
    env::set_current_dir(original_dir)?;

    result
}

/// Clean generated bindings and cache
///
/// This function removes:
/// - Generated bindings directory
/// - Cache file
/// - (Note: .cargo/config.toml patches are NOT removed to avoid breaking other tools)
///
/// # Arguments
///
/// * `project_root` - Project root directory
/// * `verbose` - Enable verbose output
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err` if cleanup fails
///
/// # Example
///
/// ```no_run
/// use cargo_ros2::clean_bindings;
/// use std::path::PathBuf;
///
/// clean_bindings(&PathBuf::from("/path/to/project"), true)?;
/// # Ok::<(), eyre::Report>(())
/// ```
pub fn clean_bindings(project_root: &Path, verbose: bool) -> Result<()> {
    let ctx = workflow::WorkflowContext::new(project_root.to_path_buf(), verbose);

    // Remove bindings directory
    if ctx.output_dir.exists() {
        std::fs::remove_dir_all(&ctx.output_dir)
            .wrap_err_with(|| format!("Failed to remove {}", ctx.output_dir.display()))?;
        if verbose {
            eprintln!("Removed {}", ctx.output_dir.display());
        }
    }

    // Remove cache file
    if ctx.cache_file.exists() {
        std::fs::remove_file(&ctx.cache_file)
            .wrap_err_with(|| format!("Failed to remove {}", ctx.cache_file.display()))?;
        if verbose {
            eprintln!("Removed {}", ctx.cache_file.display());
        }
    }

    // Note: .cargo/config.toml patches not removed (would need selective removal)
    let cargo_config = project_root.join(".cargo").join("config.toml");
    if cargo_config.exists() && verbose {
        eprintln!("Note: .cargo/config.toml patches not removed (would need selective removal)");
    }

    Ok(())
}
