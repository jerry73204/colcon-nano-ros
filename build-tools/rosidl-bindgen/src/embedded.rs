//! Embedded rosidl-runtime-rs and rclrs source code.
//!
//! This module embeds the entire rosidl-runtime-rs and rclrs directories at compile time
//! and provides functions to extract them to disk during binding generation.

use eyre::{Result, WrapErr};
use include_dir::{include_dir, Dir};
use std::path::Path;

/// Embedded rosidl-runtime-rs source directory
static ROSIDL_RUNTIME_RS: Dir =
    include_dir!("$CARGO_MANIFEST_DIR/../../user-libs/rosidl-runtime-rs");

/// Embedded rclrs source directory
static RCLRS: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../user-libs/rclrs");

/// Extract the embedded rosidl-runtime-rs source to the specified output directory
pub fn extract_embedded_runtime_rs(output_dir: &Path) -> Result<()> {
    let target = output_dir.join("rosidl_runtime_rs");

    // Extract all files at root level
    for file in ROSIDL_RUNTIME_RS.files() {
        let file_path = file.path();

        // Skip certain files
        if file_path == Path::new(".gitignore") {
            continue;
        }

        let output_path = target.join(file_path);
        std::fs::create_dir_all(&target)
            .wrap_err_with(|| format!("Failed to create directory: {}", target.display()))?;

        std::fs::write(&output_path, file.contents())
            .wrap_err_with(|| format!("Failed to write file: {}", output_path.display()))?;
    }

    // Extract all directories and their files recursively
    for dir in ROSIDL_RUNTIME_RS.dirs() {
        let dir_path = dir.path();

        // Skip certain directories
        if dir_path.starts_with("target") || dir_path.starts_with(".cargo") {
            continue;
        }

        extract_dir_recursive(dir, &target)?;
    }

    // Fix Cargo.toml to remove workspace inheritance
    fix_cargo_toml_workspace_inheritance(&target)?;

    Ok(())
}

/// Recursively extract a directory and all its contents
fn extract_dir_recursive(dir: &include_dir::Dir, target_base: &Path) -> Result<()> {
    let dir_path = dir.path();
    let output_dir = target_base.join(dir_path);

    // Create directory
    std::fs::create_dir_all(&output_dir)
        .wrap_err_with(|| format!("Failed to create directory: {}", output_dir.display()))?;

    // Extract files in this directory
    for file in dir.files() {
        let file_path = file.path();
        let output_path = target_base.join(file_path);

        std::fs::write(&output_path, file.contents())
            .wrap_err_with(|| format!("Failed to write file: {}", output_path.display()))?;
    }

    // Recursively extract subdirectories
    for subdir in dir.dirs() {
        extract_dir_recursive(subdir, target_base)?;
    }

    Ok(())
}

/// Fix Cargo.toml by replacing workspace inheritance with explicit values
fn fix_cargo_toml_workspace_inheritance(crate_dir: &Path) -> Result<()> {
    let cargo_toml_path = crate_dir.join("Cargo.toml");
    let content =
        std::fs::read_to_string(&cargo_toml_path).wrap_err("Failed to read Cargo.toml")?;

    // Replace workspace inheritance with explicit values
    // Also fix package name to use underscore (rosidl_runtime_rs) instead of dash
    let fixed_content = content
        .replace(
            "name = \"rosidl-runtime-rs\"",
            "name = \"rosidl_runtime_rs\"",
        )
        .replace("version.workspace = true", "version = \"0.1.0\"")
        .replace("authors.workspace = true", "authors = []")
        .replace("edition.workspace = true", "edition = \"2021\"")
        .replace(
            "license.workspace = true",
            "license = \"MIT OR Apache-2.0\"",
        )
        .replace(
            "repository.workspace = true",
            "repository = \"https://github.com/your-org/cargo-ros2\"",
        );

    std::fs::write(&cargo_toml_path, fixed_content).wrap_err("Failed to write fixed Cargo.toml")?;

    Ok(())
}

/// Extract the embedded rclrs source to the specified output directory
pub fn extract_embedded_rclrs(output_dir: &Path) -> Result<()> {
    let target = output_dir.join("rclrs");

    // Extract all files at root level
    for file in RCLRS.files() {
        let file_path = file.path();

        // Skip certain files
        if file_path == Path::new(".gitignore") {
            continue;
        }

        let output_path = target.join(file_path);
        std::fs::create_dir_all(&target)
            .wrap_err_with(|| format!("Failed to create directory: {}", target.display()))?;

        std::fs::write(&output_path, file.contents())
            .wrap_err_with(|| format!("Failed to write file: {}", output_path.display()))?;
    }

    // Extract all directories and their files recursively
    for dir in RCLRS.dirs() {
        let dir_path = dir.path();

        // Skip certain directories
        if dir_path.starts_with("target") || dir_path.starts_with(".cargo") {
            continue;
        }

        extract_dir_recursive(dir, &target)?;
    }

    // Fix Cargo.toml to use embedded rosidl_runtime_rs
    fix_rclrs_cargo_toml(&target)?;

    Ok(())
}

/// Fix rclrs Cargo.toml to use path dependency for embedded rosidl_runtime_rs
fn fix_rclrs_cargo_toml(crate_dir: &Path) -> Result<()> {
    let cargo_toml_path = crate_dir.join("Cargo.toml");
    let content =
        std::fs::read_to_string(&cargo_toml_path).wrap_err("Failed to read rclrs Cargo.toml")?;

    // Replace rosidl_runtime_rs version dependency with path dependency
    let fixed_content = content.replace(
        "rosidl_runtime_rs = \"0.5\"",
        "rosidl_runtime_rs = { path = \"../rosidl_runtime_rs\" }",
    );

    std::fs::write(&cargo_toml_path, fixed_content)
        .wrap_err("Failed to write fixed rclrs Cargo.toml")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_runtime_rs_exists() {
        // Print all embedded entries for debugging
        eprintln!("Embedded files:");
        for file in ROSIDL_RUNTIME_RS.files() {
            eprintln!("  file: {}", file.path().display());
        }

        eprintln!("\nEmbedded directories:");
        for dir in ROSIDL_RUNTIME_RS.dirs() {
            eprintln!("  dir: {}", dir.path().display());
            for file in dir.files() {
                eprintln!("    file: {}", file.path().display());
            }
        }

        // Verify that the embedded directory is not empty
        assert!(
            ROSIDL_RUNTIME_RS.files().count() > 0,
            "rosidl-runtime-rs should have files"
        );

        // Verify key files exist
        assert!(
            ROSIDL_RUNTIME_RS.get_file("Cargo.toml").is_some(),
            "Cargo.toml should exist"
        );
        assert!(
            ROSIDL_RUNTIME_RS.get_file("src/lib.rs").is_some(),
            "src/lib.rs should exist"
        );
    }

    #[test]
    fn test_extract_embedded_runtime_rs() {
        let temp_dir = tempfile::tempdir().unwrap();
        let output_dir = temp_dir.path();

        // Extract embedded source
        extract_embedded_runtime_rs(output_dir).unwrap();

        // Verify extraction
        let runtime_rs_dir = output_dir.join("rosidl_runtime_rs");
        assert!(
            runtime_rs_dir.exists(),
            "rosidl_runtime_rs directory should exist"
        );
        assert!(
            runtime_rs_dir.join("Cargo.toml").exists(),
            "Cargo.toml should be extracted"
        );
        assert!(
            runtime_rs_dir.join("src/lib.rs").exists(),
            "src/lib.rs should be extracted"
        );

        // Verify Cargo.toml was fixed (no workspace inheritance)
        let cargo_toml_content =
            std::fs::read_to_string(runtime_rs_dir.join("Cargo.toml")).unwrap();
        assert!(
            !cargo_toml_content.contains(".workspace = true"),
            "Cargo.toml should not contain workspace inheritance"
        );
        assert!(
            cargo_toml_content.contains("name = \"rosidl_runtime_rs\""),
            "Package name should use underscores"
        );
    }
}
