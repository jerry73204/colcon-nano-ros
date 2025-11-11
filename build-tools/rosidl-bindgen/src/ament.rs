//! Ament index integration for discovering ROS 2 packages and interface files.
//!
//! This module provides functionality to:
//! - Parse AMENT_PREFIX_PATH environment variable
//! - Discover ROS 2 packages in the ament index
//! - Locate interface files (.msg, .srv, .action) within packages

use eyre::{eyre, Result, WrapErr};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Represents a ROS 2 package discovered in the ament index
#[derive(Debug, Clone)]
pub struct Package {
    /// Package name (e.g., "std_msgs", "geometry_msgs")
    pub name: String,
    /// Path to the package's share directory
    pub share_dir: PathBuf,
    /// Interface files found in the package
    pub interfaces: InterfaceFiles,
}

/// Interface files discovered in a package
#[derive(Debug, Clone, Default)]
pub struct InterfaceFiles {
    /// .msg files (relative to share_dir/msg/)
    pub messages: Vec<String>,
    /// .srv files (relative to share_dir/srv/)
    pub services: Vec<String>,
    /// .action files (relative to share_dir/action/)
    pub actions: Vec<String>,
}

impl Package {
    /// Create a new package from a share directory path
    pub fn from_share_dir(share_dir: PathBuf) -> Result<Self> {
        let name = share_dir
            .file_name()
            .ok_or_else(|| eyre!("Invalid share directory path"))?
            .to_string_lossy()
            .to_string();

        let mut interfaces = InterfaceFiles::default();

        // Discover .msg files
        let msg_dir = share_dir.join("msg");
        if msg_dir.exists() {
            interfaces.messages = discover_interface_files(&msg_dir, "msg")?;
        }

        // Discover .srv files
        let srv_dir = share_dir.join("srv");
        if srv_dir.exists() {
            interfaces.services = discover_interface_files(&srv_dir, "srv")?;
        }

        // Discover .action files
        let action_dir = share_dir.join("action");
        if action_dir.exists() {
            interfaces.actions = discover_interface_files(&action_dir, "action")?;
        }

        Ok(Package {
            name,
            share_dir,
            interfaces,
        })
    }

    /// Get the absolute path to a message file
    pub fn get_message_path(&self, name: &str) -> PathBuf {
        self.share_dir.join("msg").join(format!("{}.msg", name))
    }

    /// Get the absolute path to a service file
    pub fn get_service_path(&self, name: &str) -> PathBuf {
        self.share_dir.join("srv").join(format!("{}.srv", name))
    }

    /// Get the absolute path to an action file
    pub fn get_action_path(&self, name: &str) -> PathBuf {
        self.share_dir
            .join("action")
            .join(format!("{}.action", name))
    }

    /// Check if package has any interface files
    pub fn has_interfaces(&self) -> bool {
        !self.interfaces.messages.is_empty()
            || !self.interfaces.services.is_empty()
            || !self.interfaces.actions.is_empty()
    }
}

/// Discover interface files in a directory with a specific extension
fn discover_interface_files(dir: &Path, extension: &str) -> Result<Vec<String>> {
    let mut files = Vec::new();

    if !dir.exists() {
        return Ok(files);
    }

    for entry in std::fs::read_dir(dir)
        .wrap_err_with(|| format!("Failed to read directory: {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == extension {
                    if let Some(name) = path.file_stem() {
                        files.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    files.sort();
    Ok(files)
}

/// Ament index for discovering ROS 2 packages
pub struct AmentIndex {
    /// Map of package name to Package
    packages: HashMap<String, Package>,
}

impl AmentIndex {
    /// Create a new AmentIndex by parsing AMENT_PREFIX_PATH environment variable
    pub fn from_env() -> Result<Self> {
        let ament_prefix_path = std::env::var("AMENT_PREFIX_PATH")
            .wrap_err("AMENT_PREFIX_PATH environment variable not set")?;

        Self::from_path_string(&ament_prefix_path)
    }

    /// Create a new AmentIndex from a path string (colon-separated paths)
    pub fn from_path_string(path_string: &str) -> Result<Self> {
        let mut packages = HashMap::new();

        // Split by ':' (Unix) or ';' (Windows)
        let separator = if cfg!(windows) { ';' } else { ':' };
        let paths: Vec<&str> = path_string.split(separator).collect();

        for prefix_path in paths {
            if prefix_path.is_empty() {
                continue;
            }

            let prefix = PathBuf::from(prefix_path);
            if !prefix.exists() {
                eprintln!(
                    "Warning: AMENT_PREFIX_PATH entry does not exist: {}",
                    prefix.display()
                );
                continue;
            }

            // Look for packages in share/
            let share_root = prefix.join("share");
            if !share_root.exists() {
                continue;
            }

            // Scan for packages
            if let Ok(entries) = std::fs::read_dir(&share_root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        // Try to create a Package from this directory
                        if let Ok(package) = Package::from_share_dir(path) {
                            // Only add if it has interface files
                            if package.has_interfaces() {
                                packages.insert(package.name.clone(), package);
                            }
                        }
                    }
                }
            }
        }

        Ok(AmentIndex { packages })
    }

    /// Find a package by name
    pub fn find_package(&self, name: &str) -> Option<&Package> {
        self.packages.get(name)
    }

    /// Get all discovered packages
    #[allow(dead_code)] // Used by cargo-ros2 crate
    pub fn packages(&self) -> &HashMap<String, Package> {
        &self.packages
    }

    /// Get package count
    pub fn package_count(&self) -> usize {
        self.packages.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Helper to create a test ament prefix with packages
    fn create_test_prefix(temp_dir: &Path, prefix_name: &str) -> PathBuf {
        let prefix = temp_dir.join(prefix_name);
        fs::create_dir_all(&prefix).unwrap();
        prefix
    }

    /// Helper to create a test package with interface files
    fn create_test_package(
        prefix: &Path,
        package_name: &str,
        msgs: &[&str],
        srvs: &[&str],
        actions: &[&str],
    ) {
        let share_dir = prefix.join("share").join(package_name);

        // Create msg files
        if !msgs.is_empty() {
            let msg_dir = share_dir.join("msg");
            fs::create_dir_all(&msg_dir).unwrap();
            for msg in msgs {
                fs::write(msg_dir.join(format!("{}.msg", msg)), "# Test message\n").unwrap();
            }
        }

        // Create srv files
        if !srvs.is_empty() {
            let srv_dir = share_dir.join("srv");
            fs::create_dir_all(&srv_dir).unwrap();
            for srv in srvs {
                fs::write(
                    srv_dir.join(format!("{}.srv", srv)),
                    "# Test service\n---\n",
                )
                .unwrap();
            }
        }

        // Create action files
        if !actions.is_empty() {
            let action_dir = share_dir.join("action");
            fs::create_dir_all(&action_dir).unwrap();
            for action in actions {
                fs::write(
                    action_dir.join(format!("{}.action", action)),
                    "# Test action\n---\n---\n",
                )
                .unwrap();
            }
        }
    }

    #[test]
    fn test_parse_empty_path_string() {
        let index = AmentIndex::from_path_string("").unwrap();
        assert_eq!(index.package_count(), 0);
    }

    #[test]
    fn test_discover_package_with_messages() {
        let temp_dir = tempfile::tempdir().unwrap();
        let prefix = create_test_prefix(temp_dir.path(), "test_ws");
        create_test_package(&prefix, "test_msgs", &["Point", "Header"], &[], &[]);

        let path_string = prefix.to_str().unwrap();
        let index = AmentIndex::from_path_string(path_string).unwrap();

        assert_eq!(index.package_count(), 1);

        let pkg = index.find_package("test_msgs").unwrap();
        assert_eq!(pkg.name, "test_msgs");
        assert_eq!(pkg.interfaces.messages.len(), 2);
        assert!(pkg.interfaces.messages.contains(&"Header".to_string()));
        assert!(pkg.interfaces.messages.contains(&"Point".to_string()));
    }

    #[test]
    fn test_discover_package_with_services() {
        let temp_dir = tempfile::tempdir().unwrap();
        let prefix = create_test_prefix(temp_dir.path(), "test_ws");
        create_test_package(&prefix, "test_srvs", &[], &["AddTwoInts", "SetBool"], &[]);

        let path_string = prefix.to_str().unwrap();
        let index = AmentIndex::from_path_string(path_string).unwrap();

        let pkg = index.find_package("test_srvs").unwrap();
        assert_eq!(pkg.interfaces.services.len(), 2);
        assert!(pkg.interfaces.services.contains(&"AddTwoInts".to_string()));
    }

    #[test]
    fn test_discover_package_with_actions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let prefix = create_test_prefix(temp_dir.path(), "test_ws");
        create_test_package(
            &prefix,
            "test_actions",
            &[],
            &[],
            &["Fibonacci", "Navigate"],
        );

        let path_string = prefix.to_str().unwrap();
        let index = AmentIndex::from_path_string(path_string).unwrap();

        let pkg = index.find_package("test_actions").unwrap();
        assert_eq!(pkg.interfaces.actions.len(), 2);
        assert!(pkg.interfaces.actions.contains(&"Fibonacci".to_string()));
    }

    #[test]
    fn test_multiple_prefixes() {
        let temp_dir = tempfile::tempdir().unwrap();

        let prefix1 = create_test_prefix(temp_dir.path(), "ws1");
        create_test_package(&prefix1, "pkg1", &["Msg1"], &[], &[]);

        let prefix2 = create_test_prefix(temp_dir.path(), "ws2");
        create_test_package(&prefix2, "pkg2", &["Msg2"], &[], &[]);

        let path_string = format!(
            "{}:{}",
            prefix1.to_str().unwrap(),
            prefix2.to_str().unwrap()
        );
        let index = AmentIndex::from_path_string(&path_string).unwrap();

        assert_eq!(index.package_count(), 2);
        assert!(index.find_package("pkg1").is_some());
        assert!(index.find_package("pkg2").is_some());
    }
}
