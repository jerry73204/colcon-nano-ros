//! cargo-nano-ros library
//!
//! This library provides functionality for generating nano-ros message bindings
//! from package.xml dependencies.
//!
//! # Public API
//!
//! This library exposes a high-level API for:
//! - Generating Rust bindings from package.xml dependencies
//! - Generating bindings for individual ROS 2 interface packages
//! - Cleaning generated bindings
//!
//! # Example
//!
//! ```no_run
//! use cargo_nano_ros::{GenerateConfig, generate_from_package_xml};
//! use std::path::PathBuf;
//!
//! let config = GenerateConfig {
//!     manifest_path: PathBuf::from("package.xml"),
//!     output_dir: PathBuf::from("generated"),
//!     generate_config: true,
//!     force: false,
//!     verbose: false,
//! };
//!
//! generate_from_package_xml(config).expect("Failed to generate bindings");
//! ```

pub mod ament_installer;
pub mod cache;
pub mod config_patcher;
pub mod dependency_parser;
pub mod package_discovery;
pub mod package_xml;
pub mod workflow;

use eyre::{eyre, Result, WrapErr};
use rosidl_bindgen::ament::{AmentIndex, Package};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Configuration for generating bindings from package.xml
#[derive(Debug, Clone)]
pub struct GenerateConfig {
    /// Path to package.xml
    pub manifest_path: PathBuf,
    /// Output directory for generated bindings
    pub output_dir: PathBuf,
    /// Generate .cargo/config.toml with [patch.crates-io] entries
    pub generate_config: bool,
    /// Path to nano-ros crates directory (for config patches)
    pub nano_ros_path: Option<PathBuf>,
    /// Overwrite existing bindings
    pub force: bool,
    /// Enable verbose output
    pub verbose: bool,
}

/// Configuration for binding generation (single package)
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

/// Configuration for ament installation (used by colcon integration)
#[derive(Debug, Clone)]
pub struct InstallConfig {
    /// Project root directory (where Cargo.toml is located)
    pub project_root: PathBuf,
    /// Install base directory (install/<package>/)
    pub install_base: PathBuf,
    /// Workspace build directory (where ros2_cargo_config.toml is located)
    pub build_base: PathBuf,
    /// Build profile: "debug" or "release"
    pub profile: String,
    /// Enable verbose output
    pub verbose: bool,
}

/// Configuration for C code generation
#[derive(Debug, Clone)]
pub struct GenerateCConfig {
    /// Path to JSON arguments file
    pub args_file: PathBuf,
    /// Enable verbose output
    pub verbose: bool,
}

/// Generate bindings from package.xml dependencies
///
/// This is the main entry point for standalone usage. It:
/// 1. Parses package.xml to find dependencies
/// 2. Resolves transitive dependencies via ament index
/// 3. Filters to interface packages (those with msg/srv/action)
/// 4. Generates nano-ros bindings for each
/// 5. Optionally generates .cargo/config.toml
pub fn generate_from_package_xml(config: GenerateConfig) -> Result<()> {
    use package_xml::PackageXml;

    // Parse package.xml
    let pkg_xml = PackageXml::parse(&config.manifest_path)?;

    if config.verbose {
        println!("Package: {} v{}", pkg_xml.name, pkg_xml.version);
        println!(
            "Dependencies from package.xml: {:?}",
            pkg_xml.all_dependencies()
        );
    }

    // Load ament index
    let index =
        AmentIndex::from_env().wrap_err("Failed to load ament index (is ROS 2 sourced?)")?;

    // Resolve all dependencies (including transitive)
    let all_deps =
        resolve_transitive_dependencies(&index, pkg_xml.all_dependencies(), config.verbose)?;

    if config.verbose {
        println!("Resolved {} total dependencies", all_deps.len());
    }

    // Filter to interface packages only
    let interface_packages = filter_interface_packages(&index, &all_deps, config.verbose)?;

    if interface_packages.is_empty() {
        println!("No interface packages found in dependencies");
        return Ok(());
    }

    println!(
        "Generating bindings for {} interface packages...",
        interface_packages.len()
    );

    // Create output directory
    std::fs::create_dir_all(&config.output_dir)?;

    // Generate bindings for each interface package
    let mut generated_packages = Vec::new();
    for (pkg_name, package) in &interface_packages {
        let pkg_output = config.output_dir.join(pkg_name);

        // Skip if exists and not forcing
        if pkg_output.exists() && !config.force {
            if config.verbose {
                println!("  Skipping {} (already exists)", pkg_name);
            }
            generated_packages.push(pkg_name.clone());
            continue;
        }

        if config.verbose {
            println!("  Generating {}...", pkg_name);
        }

        let result = rosidl_bindgen::generator::generate_package(package, &config.output_dir)?;

        println!(
            "  ✓ {} ({} messages, {} services, {} actions)",
            pkg_name, result.message_count, result.service_count, result.action_count
        );

        generated_packages.push(pkg_name.clone());
    }

    // Generate .cargo/config.toml if requested
    if config.generate_config {
        generate_cargo_config(
            &config.output_dir,
            &generated_packages,
            config.nano_ros_path.as_deref(),
            config.verbose,
        )?;
    }

    println!(
        "✓ Generated bindings for {} packages in {}",
        generated_packages.len(),
        config.output_dir.display()
    );

    Ok(())
}

/// Resolve transitive dependencies
fn resolve_transitive_dependencies(
    index: &AmentIndex,
    initial: &HashSet<String>,
    verbose: bool,
) -> Result<HashSet<String>> {
    let mut all_deps = HashSet::new();
    let mut queue: Vec<String> = initial.iter().cloned().collect();
    let mut visited = HashSet::new();

    while let Some(pkg_name) = queue.pop() {
        if visited.contains(&pkg_name) {
            continue;
        }
        visited.insert(pkg_name.clone());
        all_deps.insert(pkg_name.clone());

        // Try to find package in ament index
        if let Some(package) = index.find_package(&pkg_name) {
            // Get dependencies from package.xml in share directory
            let pkg_xml_path = package.share_dir.join("package.xml");
            if pkg_xml_path.exists() {
                if let Ok(pkg_xml) = package_xml::PackageXml::parse(&pkg_xml_path) {
                    for dep in pkg_xml.all_dependencies() {
                        if !visited.contains(dep) {
                            queue.push(dep.clone());
                        }
                    }
                }
            }
        } else if verbose {
            eprintln!("  Warning: {} not found in ament index", pkg_name);
        }
    }

    Ok(all_deps)
}

/// Filter to only interface packages (those with msg/srv/action directories)
fn filter_interface_packages(
    index: &AmentIndex,
    packages: &HashSet<String>,
    verbose: bool,
) -> Result<Vec<(String, Package)>> {
    let mut interface_packages = Vec::new();

    for pkg_name in packages {
        if let Some(package) = index.find_package(pkg_name) {
            let has_msg = package.share_dir.join("msg").exists();
            let has_srv = package.share_dir.join("srv").exists();
            let has_action = package.share_dir.join("action").exists();

            if has_msg || has_srv || has_action {
                interface_packages.push((pkg_name.clone(), package.clone()));
                if verbose {
                    println!(
                        "  Found interface package: {} (msg={}, srv={}, action={})",
                        pkg_name, has_msg, has_srv, has_action
                    );
                }
            }
        }
    }

    // Sort for deterministic output
    interface_packages.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(interface_packages)
}

/// Generate .cargo/config.toml with patch entries
fn generate_cargo_config(
    output_dir: &Path,
    packages: &[String],
    nano_ros_path: Option<&Path>,
    verbose: bool,
) -> Result<()> {
    let cargo_dir = Path::new(".cargo");
    std::fs::create_dir_all(cargo_dir)?;

    let config_path = cargo_dir.join("config.toml");

    // Build patch entries
    let mut patches = String::new();
    patches.push_str("[patch.crates-io]\n");

    // Add nano-ros crate patches if path provided
    if let Some(crates_path) = nano_ros_path {
        patches.push_str(&format!(
            "nano-ros-core = {{ path = \"{}\" }}\n",
            crates_path.join("nano-ros-core").display()
        ));
        patches.push_str(&format!(
            "nano-ros-serdes = {{ path = \"{}\" }}\n",
            crates_path.join("nano-ros-serdes").display()
        ));
    }

    // Add message package patches
    for pkg in packages {
        let pkg_path = output_dir.join(pkg);
        patches.push_str(&format!(
            "{} = {{ path = \"{}\" }}\n",
            pkg,
            pkg_path.display()
        ));
    }

    // Write or append to config.toml
    if config_path.exists() {
        // Read existing content
        let existing = std::fs::read_to_string(&config_path)?;

        // Check if [patch.crates-io] section already exists
        if existing.contains("[patch.crates-io]") {
            if verbose {
                println!(
                    "Warning: .cargo/config.toml already has [patch.crates-io] section, not modifying"
                );
            }
            return Ok(());
        }

        // Append to existing
        let new_content = format!("{}\n{}", existing.trim_end(), patches);
        std::fs::write(&config_path, new_content)?;
    } else {
        std::fs::write(&config_path, patches)?;
    }

    if verbose {
        let nano_count = if nano_ros_path.is_some() { 2 } else { 0 };
        println!(
            "Generated .cargo/config.toml with {} patch entries",
            packages.len() + nano_count
        );
    }

    Ok(())
}

/// Generate Rust bindings for a single ROS 2 interface package
pub fn generate_bindings(config: BindgenConfig) -> Result<()> {
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

/// Arguments structure for C code generation (parsed from JSON)
#[derive(Debug, serde::Deserialize)]
struct GenerateCArgs {
    package_name: String,
    output_dir: PathBuf,
    interface_files: Vec<PathBuf>,
    #[serde(default)]
    dependencies: Vec<String>,
}

/// Generate C bindings from an arguments file
///
/// This is called by the CMake `nano_ros_generate_interfaces()` function.
/// It reads a JSON arguments file and generates C code for each interface.
pub fn generate_c_from_args_file(config: GenerateCConfig) -> Result<()> {
    // Read and parse arguments file
    let args_content = std::fs::read_to_string(&config.args_file)
        .wrap_err_with(|| format!("Failed to read args file: {}", config.args_file.display()))?;

    let args: GenerateCArgs = serde_json::from_str(&args_content)
        .wrap_err_with(|| format!("Failed to parse args file: {}", config.args_file.display()))?;

    if config.verbose {
        println!("Generating C bindings for package: {}", args.package_name);
        println!("Output directory: {}", args.output_dir.display());
        println!("Interface files: {:?}", args.interface_files);
        println!("Dependencies: {:?}", args.dependencies);
    }

    // Create output directories
    let msg_dir = args.output_dir.join("msg");
    let srv_dir = args.output_dir.join("srv");
    let action_dir = args.output_dir.join("action");
    std::fs::create_dir_all(&msg_dir)?;
    std::fs::create_dir_all(&srv_dir)?;
    std::fs::create_dir_all(&action_dir)?;

    // Track generated files for umbrella header
    let mut msg_headers = Vec::new();
    let mut srv_headers = Vec::new();
    let mut action_headers = Vec::new();

    // Process each interface file
    for file_path in &args.interface_files {
        let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let file_name = file_path
            .file_stem()
            .and_then(|n| n.to_str())
            .ok_or_else(|| eyre!("Invalid interface file name: {}", file_path.display()))?;

        // Read file content
        let content = std::fs::read_to_string(file_path)
            .wrap_err_with(|| format!("Failed to read interface file: {}", file_path.display()))?;

        // Generate placeholder type hash (in production, compute from IDL)
        let type_hash = "0000000000000000000000000000000000000000000000000000000000000000";

        match extension {
            "msg" => {
                let parsed = rosidl_parser::parse_message(&content)
                    .wrap_err_with(|| format!("Failed to parse message: {}", file_name))?;

                let generated = rosidl_codegen::generate_c_message_package(
                    &args.package_name,
                    file_name,
                    &parsed,
                    type_hash,
                )
                .wrap_err_with(|| {
                    format!("Failed to generate C code for message: {}", file_name)
                })?;

                // Write header and source
                let header_path = msg_dir.join(&generated.header_name);
                let source_path = msg_dir.join(&generated.source_name);
                std::fs::write(&header_path, &generated.header)?;
                std::fs::write(&source_path, &generated.source)?;

                msg_headers.push(generated.header_name);

                if config.verbose {
                    println!("  Generated message: {}", file_name);
                }
            }
            "srv" => {
                let parsed = rosidl_parser::parse_service(&content)
                    .wrap_err_with(|| format!("Failed to parse service: {}", file_name))?;

                let generated = rosidl_codegen::generate_c_service_package(
                    &args.package_name,
                    file_name,
                    &parsed,
                    type_hash,
                )
                .wrap_err_with(|| {
                    format!("Failed to generate C code for service: {}", file_name)
                })?;

                // Write header and source
                let header_path = srv_dir.join(&generated.header_name);
                let source_path = srv_dir.join(&generated.source_name);
                std::fs::write(&header_path, &generated.header)?;
                std::fs::write(&source_path, &generated.source)?;

                srv_headers.push(generated.header_name);

                if config.verbose {
                    println!("  Generated service: {}", file_name);
                }
            }
            "action" => {
                let parsed = rosidl_parser::parse_action(&content)
                    .wrap_err_with(|| format!("Failed to parse action: {}", file_name))?;

                let generated = rosidl_codegen::generate_c_action_package(
                    &args.package_name,
                    file_name,
                    &parsed,
                    type_hash,
                )
                .wrap_err_with(|| format!("Failed to generate C code for action: {}", file_name))?;

                // Write header and source
                let header_path = action_dir.join(&generated.header_name);
                let source_path = action_dir.join(&generated.source_name);
                std::fs::write(&header_path, &generated.header)?;
                std::fs::write(&source_path, &generated.source)?;

                action_headers.push(generated.header_name);

                if config.verbose {
                    println!("  Generated action: {}", file_name);
                }
            }
            _ => {
                return Err(eyre!(
                    "Unknown interface file type: {} (expected .msg, .srv, or .action)",
                    file_path.display()
                ));
            }
        }
    }

    // Generate umbrella header
    let umbrella_header = generate_umbrella_header(
        &args.package_name,
        &msg_headers,
        &srv_headers,
        &action_headers,
        &args.dependencies,
    );
    let umbrella_path = args.output_dir.join(format!("{}.h", args.package_name));
    std::fs::write(&umbrella_path, umbrella_header)?;

    if config.verbose {
        println!("  Generated umbrella header: {}.h", args.package_name);
    }

    println!(
        "✓ Generated {} messages, {} services, {} actions for {}",
        msg_headers.len(),
        srv_headers.len(),
        action_headers.len(),
        args.package_name
    );

    Ok(())
}

/// Generate umbrella header that includes all generated headers
fn generate_umbrella_header(
    package_name: &str,
    msg_headers: &[String],
    srv_headers: &[String],
    action_headers: &[String],
    dependencies: &[String],
) -> String {
    let guard_name = format!("{}_H", package_name.to_uppercase().replace('-', "_"));

    let mut content = String::new();

    // Header guard
    content.push_str(&format!("#ifndef {}\n", guard_name));
    content.push_str(&format!("#define {}\n\n", guard_name));

    // Include nano_ros.h
    content.push_str("#include <nano_ros.h>\n\n");

    // Include dependency headers
    if !dependencies.is_empty() {
        content.push_str("// Dependencies\n");
        for dep in dependencies {
            content.push_str(&format!("#include <{}.h>\n", dep));
        }
        content.push('\n');
    }

    // Include message headers
    if !msg_headers.is_empty() {
        content.push_str("// Messages\n");
        for header in msg_headers {
            content.push_str(&format!("#include \"msg/{}\"\n", header));
        }
        content.push('\n');
    }

    // Include service headers
    if !srv_headers.is_empty() {
        content.push_str("// Services\n");
        for header in srv_headers {
            content.push_str(&format!("#include \"srv/{}\"\n", header));
        }
        content.push('\n');
    }

    // Include action headers
    if !action_headers.is_empty() {
        content.push_str("// Actions\n");
        for header in action_headers {
            content.push_str(&format!("#include \"action/{}\"\n", header));
        }
        content.push('\n');
    }

    // End header guard
    content.push_str(&format!("#endif  // {}\n", guard_name));

    content
}

/// Clean generated bindings
pub fn clean_generated(output_dir: &Path, clean_config: bool, verbose: bool) -> Result<()> {
    // Remove output directory
    if output_dir.exists() {
        std::fs::remove_dir_all(output_dir)
            .wrap_err_with(|| format!("Failed to remove {}", output_dir.display()))?;
        if verbose {
            println!("Removed {}", output_dir.display());
        }
    }

    // Remove .cargo/config.toml if requested
    if clean_config {
        let config_path = Path::new(".cargo").join("config.toml");
        if config_path.exists() {
            std::fs::remove_file(&config_path)
                .wrap_err_with(|| format!("Failed to remove {}", config_path.display()))?;
            if verbose {
                println!("Removed {}", config_path.display());
            }
        }
    }

    Ok(())
}

/// Clean bindings in a project directory (for colcon integration)
///
/// This removes the target/ros2_bindings directory and related cache.
pub fn clean_bindings(project_root: &Path, verbose: bool) -> Result<()> {
    let bindings_dir = project_root.join("target").join("ros2_bindings");
    if bindings_dir.exists() {
        std::fs::remove_dir_all(&bindings_dir)
            .wrap_err_with(|| format!("Failed to remove {}", bindings_dir.display()))?;
        if verbose {
            println!("Removed {}", bindings_dir.display());
        }
    }
    Ok(())
}

/// Install package binaries and libraries to ament layout (for colcon integration)
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
    let mut metadata_cmd = MetadataCommand::new();

    // Use build_base provided by colcon to locate config file
    let config_file = config.build_base.join("ros2_cargo_config.toml");

    if config.verbose {
        eprintln!("Using config file: {}", config_file.display());
    }

    if config_file.exists() {
        metadata_cmd.other_options(vec![
            "--config".to_string(),
            config_file.display().to_string(),
        ]);
    }

    let metadata = metadata_cmd
        .exec()
        .wrap_err("Failed to read Cargo metadata")?;

    let root_package = metadata
        .root_package()
        .ok_or_else(|| eyre!("No root package found in Cargo.toml"))?;

    let package_name = root_package.name.clone();
    let target_dir = metadata.target_directory.clone().into_std_path_buf();
    let is_lib_only = is_library_package(&config.project_root)?;

    let installer = AmentInstaller::new(
        config.install_base.clone(),
        package_name,
        config.project_root.clone(),
        target_dir,
        config.verbose,
        config.profile.clone(),
    );

    let result = installer.install(is_lib_only);

    env::set_current_dir(original_dir)?;

    result
}
