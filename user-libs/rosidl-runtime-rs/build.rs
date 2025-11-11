fn main() {
    // Link against rosidl_runtime_c library
    // This library provides the generic ROS runtime functions for strings and primitive sequences
    println!("cargo:rustc-link-lib=rosidl_runtime_c");

    // Add ROS library search paths from AMENT_PREFIX_PATH (for system packages)
    if let Ok(ament_prefix_path) = std::env::var("AMENT_PREFIX_PATH") {
        for prefix in ament_prefix_path.split(':') {
            let lib_path = std::path::Path::new(prefix).join("lib");
            if lib_path.exists() {
                println!("cargo:rustc-link-search=native={}", lib_path.display());
            }
        }
    }

    // Also search for workspace-local install directory (for custom packages)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut search_dir = std::path::Path::new(&manifest_dir);

        // Walk up the directory tree to find workspace root
        for _ in 0..10 {
            let install_dir = search_dir.join("install");
            if install_dir.exists() && install_dir.is_dir() {
                // Add all package lib directories from install/
                if let Ok(entries) = std::fs::read_dir(&install_dir) {
                    for entry in entries.flatten() {
                        let lib_path = entry.path().join("lib");
                        if lib_path.exists() {
                            println!("cargo:rustc-link-search=native={}", lib_path.display());
                        }
                    }
                }
                break;
            }

            // Move up one directory
            if let Some(parent) = search_dir.parent() {
                search_dir = parent;
            } else {
                break;
            }
        }
    }

    // Rerun if AMENT_PREFIX_PATH changes
    println!("cargo:rerun-if-env-changed=AMENT_PREFIX_PATH");
}
