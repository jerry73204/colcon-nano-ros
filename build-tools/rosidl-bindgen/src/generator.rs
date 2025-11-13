//! Generator integration for generating Rust bindings from ROS 2 interface packages.
//!
//! This module integrates with rosidl-codegen to:
//! - Parse interface files (.msg, .srv, .action)
//! - Generate Rust code for messages, services, and actions
//! - Write generated code to output directory with proper structure

use crate::ament::Package;
use askama::Template;
use eyre::{Result, WrapErr};
use rosidl_codegen::{
    generate_action_package, generate_message_package, generate_service_package,
    utils::{extract_dependencies, needs_big_array, to_snake_case},
    GeneratedPackage,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Interface info for template rendering
#[derive(Debug, Clone)]
struct InterfaceInfo {
    type_name: String,
    module_name: String,
}

/// Template for generating lib.rs
#[derive(Template)]
#[template(path = "lib.rs.jinja")]
struct LibRsTemplate {
    package_name: String,
    ffi_module: String,
    has_messages: bool,
    has_services: bool,
    has_actions: bool,
    messages: Vec<InterfaceInfo>,
    services: Vec<InterfaceInfo>,
    actions: Vec<InterfaceInfo>,
}

/// Supported rosidl_runtime_rs version on crates.io
///
/// Generated bindings depend on this version from crates.io.
/// Users must ensure this version is available in their Cargo dependencies.
pub const ROSIDL_RUNTIME_RS_VERSION: &str = "0.5";

/// Supported rclrs version on crates.io
///
/// For building ROS 2 nodes, users should depend on this version from crates.io.
pub const RCLRS_VERSION: &str = "0.6";

/// Top-level module name for C-compatible FFI layer (Foreign Function Interface).
/// This is placed at the crate root level to avoid conflicts with any message/service/action names.
///
/// The dual-layer architecture is:
/// - `pkg::ffi::msg::Type` - C-compatible FFI structs for interop with ROS C libraries
/// - `pkg::msg::Type` - Idiomatic Rust wrappers with safe types (String, Vec, etc.)
///
/// By placing `ffi` at the package root (not nested in msg/srv/action), it cannot conflict
/// with any message names (e.g., ffi.msg, rmw.msg, etc.)
const FFI_MODULE: &str = "ffi";

/// Generated Rust package structure
#[derive(Debug)]
pub struct GeneratedRustPackage {
    /// Package name
    pub name: String,
    /// Output directory where code was written
    pub output_dir: PathBuf,
    /// Number of messages generated
    pub message_count: usize,
    /// Number of services generated
    pub service_count: usize,
    /// Number of actions generated
    pub action_count: usize,
}

/// Generate Rust bindings for a ROS 2 package
pub fn generate_package(package: &Package, output_dir: &Path) -> Result<GeneratedRustPackage> {
    let package_output = output_dir.join(&package.name);
    std::fs::create_dir_all(&package_output).wrap_err_with(|| {
        format!(
            "Failed to create output directory: {}",
            package_output.display()
        )
    })?;

    let mut message_count = 0;
    let mut service_count = 0;
    let mut action_count = 0;
    let mut all_dependencies = HashSet::new();
    let mut package_needs_big_array = false;

    // For dependency tracking (cross-package references)
    let known_packages = HashSet::new(); // TODO: populate from ament index

    // Generate messages
    for msg_name in &package.interfaces.messages {
        let msg_path = package.get_message_path(msg_name);
        let content = std::fs::read_to_string(&msg_path)
            .wrap_err_with(|| format!("Failed to read message file: {}", msg_path.display()))?;

        let parsed_msg = rosidl_parser::parse_message(&content)
            .wrap_err_with(|| format!("Failed to parse message: {}", msg_name))?;

        // Extract dependencies from this message
        let msg_deps = extract_dependencies(&parsed_msg);
        all_dependencies.extend(msg_deps);

        // Check if this message needs big_array support
        if needs_big_array(&parsed_msg) {
            package_needs_big_array = true;
        }

        let generated =
            generate_message_package(&package.name, msg_name, &parsed_msg, &known_packages)
                .wrap_err_with(|| format!("Failed to generate message: {}", msg_name))?;

        write_generated_package(&generated, &package_output, msg_name)?;
        message_count += 1;
    }

    // Generate services
    for srv_name in &package.interfaces.services {
        let srv_path = package.get_service_path(srv_name);
        let content = std::fs::read_to_string(&srv_path)
            .wrap_err_with(|| format!("Failed to read service file: {}", srv_path.display()))?;

        let parsed_srv = rosidl_parser::parse_service(&content)
            .wrap_err_with(|| format!("Failed to parse service: {}", srv_name))?;

        // Extract dependencies from request and response messages
        let req_deps = extract_dependencies(&parsed_srv.request);
        let resp_deps = extract_dependencies(&parsed_srv.response);
        all_dependencies.extend(req_deps);
        all_dependencies.extend(resp_deps);

        // Check if request or response needs big_array support
        if needs_big_array(&parsed_srv.request) || needs_big_array(&parsed_srv.response) {
            package_needs_big_array = true;
        }

        let generated =
            generate_service_package(&package.name, srv_name, &parsed_srv, &known_packages)
                .wrap_err_with(|| format!("Failed to generate service: {}", srv_name))?;

        write_generated_service(&generated, &package_output, srv_name)?;
        service_count += 1;
    }

    // Generate actions
    for action_name in &package.interfaces.actions {
        let action_path = package.get_action_path(action_name);
        let content = std::fs::read_to_string(&action_path)
            .wrap_err_with(|| format!("Failed to read action file: {}", action_path.display()))?;

        let parsed_action = rosidl_parser::parse_action(&content)
            .wrap_err_with(|| format!("Failed to parse action: {}", action_name))?;

        // Extract dependencies from goal, result, and feedback messages
        let goal_deps = extract_dependencies(&parsed_action.spec.goal);
        let result_deps = extract_dependencies(&parsed_action.spec.result);
        let feedback_deps = extract_dependencies(&parsed_action.spec.feedback);
        all_dependencies.extend(goal_deps);
        all_dependencies.extend(result_deps);
        all_dependencies.extend(feedback_deps);

        // Check if goal, result, or feedback needs big_array support
        if needs_big_array(&parsed_action.spec.goal)
            || needs_big_array(&parsed_action.spec.result)
            || needs_big_array(&parsed_action.spec.feedback)
        {
            package_needs_big_array = true;
        }

        let generated =
            generate_action_package(&package.name, action_name, &parsed_action, &known_packages)
                .wrap_err_with(|| format!("Failed to generate action: {}", action_name))?;

        write_generated_action(&generated, &package_output, action_name)?;
        action_count += 1;
    }

    // Remove self-dependency (package shouldn't depend on itself)
    all_dependencies.remove(&package.name);

    // Generate lib.rs that re-exports all generated code
    generate_lib_rs(&package_output, package, &all_dependencies)?;

    // Generate Cargo.toml for the package
    generate_cargo_toml(
        &package_output,
        &package.name,
        &all_dependencies,
        package_needs_big_array,
    )?;

    // Generate build.rs for FFI linking
    generate_build_rs(&package_output, &package.name)?;

    Ok(GeneratedRustPackage {
        name: package.name.clone(),
        output_dir: package_output,
        message_count,
        service_count,
        action_count,
    })
}

/// Write generated message package to files
fn write_generated_package(
    generated: &GeneratedPackage,
    output_dir: &Path,
    name: &str,
) -> Result<()> {
    // Create idiomatic message directory: src/msg/
    let msg_dir = output_dir.join("src").join("msg");
    std::fs::create_dir_all(&msg_dir)?;

    // Create FFI message directory: src/ffi/msg/
    let ffi_msg_dir = output_dir.join("src").join(FFI_MODULE).join("msg");
    std::fs::create_dir_all(&ffi_msg_dir)?;

    // Write FFI message to src/ffi/msg/
    let rmw_file = ffi_msg_dir.join(format!("{}_rmw.rs", to_snake_case(name)));
    std::fs::write(&rmw_file, &generated.message_rmw)?;

    // Write idiomatic message to src/msg/
    let idiomatic_file = msg_dir.join(format!("{}_idiomatic.rs", to_snake_case(name)));
    std::fs::write(&idiomatic_file, &generated.message_idiomatic)?;

    Ok(())
}

/// Write generated service package to files
fn write_generated_service(
    generated: &rosidl_codegen::GeneratedServicePackage,
    output_dir: &Path,
    name: &str,
) -> Result<()> {
    // Create idiomatic service directory: src/srv/
    let srv_dir = output_dir.join("src").join("srv");
    std::fs::create_dir_all(&srv_dir)?;

    // Create FFI service directory: src/ffi/srv/
    let ffi_srv_dir = output_dir.join("src").join(FFI_MODULE).join("srv");
    std::fs::create_dir_all(&ffi_srv_dir)?;

    // Write FFI service to src/ffi/srv/
    let rmw_file = ffi_srv_dir.join(format!("{}_rmw.rs", to_snake_case(name)));
    std::fs::write(&rmw_file, &generated.service_rmw)?;

    // Write idiomatic service to src/srv/
    let idiomatic_file = srv_dir.join(format!("{}_idiomatic.rs", to_snake_case(name)));
    std::fs::write(&idiomatic_file, &generated.service_idiomatic)?;

    Ok(())
}

/// Write generated action package to files
fn write_generated_action(
    generated: &rosidl_codegen::GeneratedActionPackage,
    output_dir: &Path,
    name: &str,
) -> Result<()> {
    // Create idiomatic action directory: src/action/
    let action_dir = output_dir.join("src").join("action");
    std::fs::create_dir_all(&action_dir)?;

    // Create FFI action directory: src/ffi/action/
    let ffi_action_dir = output_dir.join("src").join(FFI_MODULE).join("action");
    std::fs::create_dir_all(&ffi_action_dir)?;

    // Write FFI action to src/ffi/action/
    let rmw_file = ffi_action_dir.join(format!("{}_rmw.rs", to_snake_case(name)));
    std::fs::write(&rmw_file, &generated.action_rmw)?;

    // Write idiomatic action to src/action/
    let idiomatic_file = action_dir.join(format!("{}_idiomatic.rs", to_snake_case(name)));
    std::fs::write(&idiomatic_file, &generated.action_idiomatic)?;

    Ok(())
}

/// Generate lib.rs that re-exports all generated modules
fn generate_lib_rs(
    output_dir: &Path,
    package: &Package,
    _dependencies: &HashSet<String>,
) -> Result<()> {
    let src_dir = output_dir.join("src");
    std::fs::create_dir_all(&src_dir)?;

    // Collect message info
    let messages: Vec<InterfaceInfo> = package
        .interfaces
        .messages
        .iter()
        .map(|name| InterfaceInfo {
            type_name: name.clone(),
            module_name: to_snake_case(name),
        })
        .collect();

    // Collect service info
    let services: Vec<InterfaceInfo> = package
        .interfaces
        .services
        .iter()
        .map(|name| InterfaceInfo {
            type_name: name.clone(),
            module_name: to_snake_case(name),
        })
        .collect();

    // Collect action info
    let actions: Vec<InterfaceInfo> = package
        .interfaces
        .actions
        .iter()
        .map(|name| InterfaceInfo {
            type_name: name.clone(),
            module_name: to_snake_case(name),
        })
        .collect();

    // Render template
    let template = LibRsTemplate {
        package_name: package.name.clone(),
        ffi_module: FFI_MODULE.to_string(),
        has_messages: !messages.is_empty(),
        has_services: !services.is_empty(),
        has_actions: !actions.is_empty(),
        messages,
        services,
        actions,
    };

    let lib_rs = template.render()?;
    std::fs::write(src_dir.join("lib.rs"), lib_rs)?;
    Ok(())
}

/// Generate Cargo.toml for the generated package
fn generate_cargo_toml(
    output_dir: &Path,
    package_name: &str,
    dependencies: &HashSet<String>,
    needs_big_array: bool,
) -> Result<()> {
    let mut cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

# Standalone package (not part of parent workspace)
[workspace]

[dependencies]
# Shared runtime library for ROS 2 types and traits (from crates.io)
rosidl_runtime_rs = "{}"
serde = {{ version = "1.0", features = ["derive"], optional = true }}
"#,
        package_name, ROSIDL_RUNTIME_RS_VERSION
    );

    // Add serde-big-array if needed for arrays > 32 elements
    if needs_big_array {
        cargo_toml.push_str("serde-big-array = { version = \"0.5\", optional = true }\n");
    }

    // Add cross-package dependencies
    for dep in dependencies {
        // Convert package name to valid crate name (replace - with _)
        let crate_name = dep.replace('-', "_");
        cargo_toml.push_str(&format!("{} = {{ path = \"../{}\" }}\n", crate_name, dep));
    }

    // Add features section
    cargo_toml.push_str("\n[features]\ndefault = []\n");
    if needs_big_array {
        cargo_toml.push_str("serde = [\"dep:serde\", \"dep:serde-big-array\"]\n");
    } else {
        cargo_toml.push_str("serde = [\"dep:serde\"]\n");
    }

    cargo_toml.push_str(
        r#"
[build-dependencies]
# For linking against ROS 2 C libraries
"#,
    );

    std::fs::write(output_dir.join("Cargo.toml"), cargo_toml)?;
    Ok(())
}

/// Generate build.rs for linking against ROS 2 C libraries
fn generate_build_rs(output_dir: &Path, package_name: &str) -> Result<()> {
    let build_rs = format!(
        r#"fn main() {{
    // Add ROS library search paths from AMENT_PREFIX_PATH (for system packages)
    if let Ok(ament_prefix_path) = std::env::var("AMENT_PREFIX_PATH") {{
        for prefix in ament_prefix_path.split(':') {{
            let lib_path = std::path::Path::new(prefix).join("lib");
            if lib_path.exists() {{
                println!("cargo:rustc-link-search=native={{}}", lib_path.display());
            }}
        }}
    }}

    // Also search for workspace-local install directory (for custom packages)
    // This is critical for colcon workspaces where packages are built incrementally
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {{
        let mut search_dir = std::path::Path::new(&manifest_dir);

        // Walk up the directory tree to find workspace root
        for _ in 0..10 {{
            // Check if this looks like a colcon workspace root
            let install_dir = search_dir.join("install");
            if install_dir.exists() && install_dir.is_dir() {{
                // Add all package lib directories from install/
                if let Ok(entries) = std::fs::read_dir(&install_dir) {{
                    for entry in entries.flatten() {{
                        let lib_path = entry.path().join("lib");
                        if lib_path.exists() {{
                            println!("cargo:rustc-link-search=native={{}}", lib_path.display());
                        }}
                    }}
                }}
                break;
            }}

            // Move up one directory
            if let Some(parent) = search_dir.parent() {{
                search_dir = parent;
            }} else {{
                break;
            }}
        }}
    }}

    // Link against ROS 2 C libraries
    println!("cargo:rustc-link-lib={package}__rosidl_typesupport_c");
    println!("cargo:rustc-link-lib={package}__rosidl_generator_c");
}}
"#,
        package = package_name
    );

    std::fs::write(output_dir.join("build.rs"), build_rs)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ament::Package;
    use std::fs;

    /// Helper to create a test package with interface files
    fn create_test_package(temp_dir: &Path) -> Package {
        let share_dir = temp_dir.join("test_pkg");

        // Create msg files
        let msg_dir = share_dir.join("msg");
        fs::create_dir_all(&msg_dir).unwrap();
        fs::write(msg_dir.join("Point.msg"), "float64 x\nfloat64 y\n").unwrap();

        // Create srv files
        let srv_dir = share_dir.join("srv");
        fs::create_dir_all(&srv_dir).unwrap();
        fs::write(
            srv_dir.join("AddTwoInts.srv"),
            "int64 a\nint64 b\n---\nint64 sum\n",
        )
        .unwrap();

        // Create action files
        let action_dir = share_dir.join("action");
        fs::create_dir_all(&action_dir).unwrap();
        fs::write(
            action_dir.join("Fibonacci.action"),
            "int32 order\n---\nint32[] sequence\n---\nint32[] partial_sequence\n",
        )
        .unwrap();

        Package::from_share_dir(share_dir).unwrap()
    }

    #[test]
    fn test_generate_message() {
        let temp_dir = tempfile::tempdir().unwrap();
        let package = create_test_package(temp_dir.path());
        let output_dir = temp_dir.path().join("output");

        let result = generate_package(&package, &output_dir);
        assert!(result.is_ok());

        let generated = result.unwrap();
        assert_eq!(generated.message_count, 1);
        assert_eq!(generated.service_count, 1);
        assert_eq!(generated.action_count, 1);

        // Check that files were created
        let pkg_dir = output_dir.join("test_pkg");
        assert!(pkg_dir.join("Cargo.toml").exists());
        assert!(pkg_dir.join("build.rs").exists());
        assert!(pkg_dir.join("src").join("lib.rs").exists());
    }

    #[test]
    fn test_generate_lib_rs_structure() {
        let temp_dir = tempfile::tempdir().unwrap();
        let package = create_test_package(temp_dir.path());
        let output_dir = temp_dir.path().join("output");
        std::fs::create_dir_all(&output_dir).unwrap();

        let deps = HashSet::new();
        generate_lib_rs(&output_dir, &package, &deps).unwrap();

        let lib_rs_content =
            std::fs::read_to_string(output_dir.join("src").join("lib.rs")).unwrap();
        assert!(lib_rs_content.contains("pub mod msg"));
        assert!(lib_rs_content.contains("pub mod srv"));
        assert!(lib_rs_content.contains("pub mod action"));
    }

    #[test]
    fn test_cargo_toml_generation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let deps = HashSet::new();
        generate_cargo_toml(temp_dir.path(), "test_pkg", &deps, false).unwrap();

        let cargo_toml = std::fs::read_to_string(temp_dir.path().join("Cargo.toml")).unwrap();
        assert!(cargo_toml.contains("name = \"test_pkg\""));
        assert!(cargo_toml.contains("serde"));
        assert!(!cargo_toml.contains("serde-big-array"));
    }

    #[test]
    fn test_cargo_toml_with_dependencies() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut deps = HashSet::new();
        deps.insert("std_msgs".to_string());
        deps.insert("geometry_msgs".to_string());

        generate_cargo_toml(temp_dir.path(), "test_pkg", &deps, false).unwrap();

        let cargo_toml = std::fs::read_to_string(temp_dir.path().join("Cargo.toml")).unwrap();
        assert!(cargo_toml.contains("name = \"test_pkg\""));
        assert!(cargo_toml.contains("serde"));
        assert!(cargo_toml.contains("std_msgs = { path = \"../std_msgs\" }"));
        assert!(cargo_toml.contains("geometry_msgs = { path = \"../geometry_msgs\" }"));
    }

    #[test]
    fn test_cargo_toml_with_big_array() {
        let temp_dir = tempfile::tempdir().unwrap();
        let deps = HashSet::new();
        generate_cargo_toml(temp_dir.path(), "test_pkg", &deps, true).unwrap();

        let cargo_toml = std::fs::read_to_string(temp_dir.path().join("Cargo.toml")).unwrap();
        assert!(cargo_toml.contains("name = \"test_pkg\""));
        assert!(cargo_toml.contains("serde"));
        assert!(cargo_toml.contains("serde-big-array"));
    }

    #[test]
    fn test_build_rs_generation() {
        let temp_dir = tempfile::tempdir().unwrap();
        generate_build_rs(temp_dir.path(), "test_pkg").unwrap();

        let build_rs = std::fs::read_to_string(temp_dir.path().join("build.rs")).unwrap();
        assert!(build_rs.contains("test_pkg__rosidl_typesupport_c"));
        assert!(build_rs.contains("test_pkg__rosidl_generator_c"));
    }

    #[test]
    fn test_invalid_message_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let share_dir = temp_dir.path().join("bad_pkg");
        let msg_dir = share_dir.join("msg");
        fs::create_dir_all(&msg_dir).unwrap();
        fs::write(msg_dir.join("Bad.msg"), "invalid syntax here!!! @#$%\n").unwrap();

        let package = Package::from_share_dir(share_dir).unwrap();
        let output_dir = temp_dir.path().join("output");

        let result = generate_package(&package, &output_dir);
        assert!(result.is_err());
    }
}
