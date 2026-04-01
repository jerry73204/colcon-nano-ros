//! cargo-nano-ros: Standalone build tool for nros
//!
//! Generate ROS 2 message bindings from package.xml dependencies.

use cargo_nano_ros::GenerateConfig;
use clap::{Parser, Subcommand};
use eyre::Result;
use std::path::{Path, PathBuf};

/// Standalone build tool for nros
#[derive(Parser, Debug)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum CargoCli {
    NanoRos(NanoRosArgs),
}

#[derive(Debug, Parser)]
#[command(name = "nano-ros")]
#[command(about = "Standalone build tool for nano-ros", long_about = None)]
struct NanoRosArgs {
    #[command(subcommand)]
    command: NanoRosCommand,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Debug, Subcommand)]
enum NanoRosCommand {
    /// Generate Rust bindings from package.xml dependencies
    ///
    /// Reads package.xml to discover ROS 2 interface dependencies,
    /// resolves transitive dependencies, and generates nros bindings.
    GenerateRust {
        /// Path to package.xml (default: ./package.xml)
        #[arg(long, default_value = "package.xml")]
        manifest_path: PathBuf,

        /// Output directory for generated bindings (default: ./generated)
        #[arg(long, short, default_value = "generated")]
        output: PathBuf,

        /// Generate .cargo/config.toml with [patch.crates-io] entries
        #[arg(long)]
        config: bool,

        /// Path to nros crates directory (for config patches)
        /// If not specified, nros crates will use crates.io (requires published crates)
        #[arg(long, conflicts_with = "nano_ros_git")]
        nano_ros_path: Option<PathBuf>,

        /// Use nros git repository for config patches
        /// Generates [patch.crates-io] entries pointing to the nros git repository
        #[arg(long, conflicts_with = "nano_ros_path")]
        nano_ros_git: bool,

        /// Overwrite existing bindings
        #[arg(long)]
        force: bool,

        /// ROS 2 edition for type hash format (humble or iron)
        #[arg(long, default_value = "humble")]
        ros_edition: String,

        /// Rename a generated package: --rename old_pkg=new_crate_name
        /// Can be specified multiple times. Affects crate name, directory name,
        /// cross-package use statements, and Cargo.toml dependencies.
        #[arg(long, value_parser = parse_rename)]
        rename: Vec<(String, String)>,
    },

    /// (Hidden) Backward-compatible alias for generate-rust
    #[command(hide = true)]
    Generate {
        #[arg(long, default_value = "package.xml")]
        manifest_path: PathBuf,
        #[arg(long, short, default_value = "generated")]
        output: PathBuf,
        #[arg(long)]
        config: bool,
        #[arg(long, conflicts_with = "nano_ros_git")]
        nano_ros_path: Option<PathBuf>,
        #[arg(long, conflicts_with = "nano_ros_path")]
        nano_ros_git: bool,
        #[arg(long)]
        force: bool,
        #[arg(long, default_value = "humble")]
        ros_edition: String,
        #[arg(long, value_parser = parse_rename)]
        rename: Vec<(String, String)>,
    },

    /// Generate C bindings for interface files (.msg, .srv, .action)
    ///
    /// Generates C code for use with nros-c library. Called by
    /// nano_ros_generate_interfaces() CMake function.
    GenerateC {
        /// Path to JSON arguments file
        #[arg(long)]
        args_file: PathBuf,
    },

    /// Generate C++ bindings for interface files (.msg, .srv, .action)
    ///
    /// Generates C++ headers + Rust FFI glue for use with nros-cpp.
    /// Called by nano_ros_generate_interfaces(LANGUAGE CPP).
    GenerateCpp {
        /// Path to JSON arguments file
        #[arg(long)]
        args_file: PathBuf,
    },

    /// Generate bindings for a single ROS 2 package (low-level)
    Bindgen {
        /// ROS package name
        #[arg(long)]
        package: String,

        /// Output directory for generated bindings
        #[arg(long)]
        output: PathBuf,

        /// Direct path to package share directory (bypasses ament index)
        #[arg(long)]
        package_path: Option<PathBuf>,
    },

    /// Create a new nano-ros package (colcon-compatible)
    ///
    /// Scaffolds a complete package with package.xml, Cargo.toml or
    /// CMakeLists.txt, config.toml, and source files.
    New {
        /// Package name
        name: String,

        /// Language: rust, c, or cpp
        #[arg(long, default_value = "rust")]
        lang: String,

        /// Target platform: native, freertos, zephyr, nuttx, baremetal, threadx
        #[arg(long, default_value = "native")]
        platform: String,
    },

    /// Clean generated bindings
    Clean {
        /// Output directory to clean (default: ./generated)
        #[arg(long, short, default_value = "generated")]
        output: PathBuf,

        /// Also remove .cargo/config.toml patches
        #[arg(long)]
        config: bool,
    },
}

use cargo_nano_ros::parse_rename;

fn run_generate(cfg: GenerateConfig) -> Result<()> {
    cargo_nano_ros::generate_from_package_xml(cfg)
}

fn main() -> Result<()> {
    let CargoCli::NanoRos(args) = CargoCli::parse();

    match args.command {
        NanoRosCommand::GenerateRust {
            manifest_path,
            output,
            config,
            nano_ros_path,
            nano_ros_git,
            force,
            ros_edition,
            rename,
        }
        | NanoRosCommand::Generate {
            manifest_path,
            output,
            config,
            nano_ros_path,
            nano_ros_git,
            force,
            ros_edition,
            rename,
        } => {
            run_generate(GenerateConfig {
                manifest_path,
                output_dir: output,
                generate_config: config,
                nano_ros_path,
                nano_ros_git,
                force,
                verbose: args.verbose,
                ros_edition,
                renames: rename.into_iter().collect(),
            })?;
        }

        NanoRosCommand::GenerateC { args_file } => {
            let cfg = cargo_nano_ros::GenerateCConfig {
                args_file,
                verbose: args.verbose,
            };
            cargo_nano_ros::generate_c_from_args_file(cfg)?;
            println!("✓ C bindings generated successfully");
        }

        NanoRosCommand::GenerateCpp { args_file } => {
            let cfg = cargo_nano_ros::GenerateCppConfig {
                args_file,
                verbose: args.verbose,
            };
            cargo_nano_ros::generate_cpp_from_args_file(cfg)?;
            println!("✓ C++ bindings generated successfully");
        }

        NanoRosCommand::Bindgen {
            package,
            output,
            package_path,
        } => {
            let cfg = cargo_nano_ros::BindgenConfig {
                package_name: package,
                package_path,
                output_dir: output,
                verbose: args.verbose,
            };
            cargo_nano_ros::generate_bindings(cfg)?;
            println!("✓ Bindings generated successfully");
        }

        NanoRosCommand::New {
            name,
            lang,
            platform,
        } => {
            scaffold_package(&name, &lang, &platform)?;
        }

        NanoRosCommand::Clean { output, config } => {
            cargo_nano_ros::clean_generated(&output, config, args.verbose)?;
            println!("✓ Cleaned successfully");
        }
    }

    Ok(())
}

fn scaffold_package(name: &str, lang: &str, platform: &str) -> Result<()> {
    use std::fs;

    let dir = PathBuf::from(name);
    if dir.exists() {
        eyre::bail!("Directory '{}' already exists", name);
    }

    let build_type = format!("nros.{lang}.{platform}");

    // Create directory structure
    fs::create_dir_all(dir.join("src"))?;

    // package.xml
    let package_xml = format!(
        r#"<?xml version="1.0"?>
<package format="3">
  <name>{name}</name>
  <version>0.1.0</version>
  <description>{name} — nano-ros {platform} package</description>
  <maintainer email="TODO@todo.com">TODO</maintainer>
  <license>Apache-2.0</license>
  <depend>std_msgs</depend>
  <export>
    <build_type>{build_type}</build_type>
  </export>
</package>
"#
    );
    fs::write(dir.join("package.xml"), package_xml)?;

    match lang {
        "rust" => scaffold_rust(name, platform, &dir)?,
        "c" => scaffold_c(name, platform, &dir)?,
        "cpp" => scaffold_cpp(name, platform, &dir)?,
        _ => eyre::bail!("Unknown language: {lang}. Use rust, c, or cpp."),
    }

    println!("✓ Created nano-ros package '{name}'");
    println!("  Language: {lang}");
    println!("  Platform: {platform}");
    println!("  Build type: {build_type}");
    println!();
    println!("Next steps:");
    println!("  cd {name}");
    println!("  colcon build --packages-select {name}");

    Ok(())
}

fn scaffold_rust(name: &str, platform: &str, dir: &Path) -> Result<()> {
    use std::fs;

    // Cargo.toml
    let mut deps = String::new();
    let is_embedded = platform != "native";

    if is_embedded {
        deps.push_str(&format!(
            "nros = {{ version = \"0.1\", default-features = false, features = [\"rmw-zenoh\", \"platform-{platform}\", \"ros-humble\"] }}\n"
        ));
        // Default board crate based on platform
        let board_crate = match platform {
            "freertos" => "nros-mps2-an385-freertos",
            "baremetal" => "nros-mps2-an385",
            "nuttx" => "nros-nuttx-qemu-arm",
            _ => "# TODO: add board crate for this platform",
        };
        deps.push_str(&format!("{board_crate} = {{ version = \"0.1\" }}\n"));
        deps.push_str("panic-semihosting = \"0.6\"\n");
    } else {
        // Native hello-world has no nros dependency — add it when needed:
        // nros = { version = "0.1", features = ["std", "rmw-zenoh", "platform-posix", "ros-humble"] }
        deps.push_str("# nros = { version = \"0.1\", features = [\"std\", \"rmw-zenoh\", \"platform-posix\", \"ros-humble\"] }\n");
    }

    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[workspace]

[[bin]]
name = "{name}"
path = "src/main.rs"

[dependencies]
{deps}"#
    );
    fs::write(dir.join("Cargo.toml"), cargo_toml)?;

    // src/main.rs
    let main_rs = if is_embedded {
        format!(
            r#"#![no_std]
#![no_main]

use nros::prelude::*;
// TODO: import your board crate
// use nros_mps2_an385_freertos::{{Config, run, println}};
use panic_semihosting as _;

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {{
    // TODO: replace with your board crate's run()
    loop {{}}
}}
"#
        )
    } else {
        format!(
            r#"fn main() {{
    println!("Hello from {name}!");
}}
"#
        )
    };
    fs::write(dir.join("src/main.rs"), main_rs)?;

    // config.toml (for embedded platforms)
    if is_embedded {
        let config_toml = r#"[network]
ip = "10.0.2.20"
mac = "02:00:00:00:00:00"
gateway = "10.0.2.2"
netmask = "255.255.255.0"

[zenoh]
locator = "tcp/10.0.2.2:7447"
domain_id = 0
"#;
        fs::write(dir.join("config.toml"), config_toml)?;
    }

    Ok(())
}

fn scaffold_c(name: &str, platform: &str, dir: &Path) -> Result<()> {
    use std::fs;

    let cmake = format!(
        r#"cmake_minimum_required(VERSION 3.16)
project({name} VERSION 0.1.0 LANGUAGES C)

set(CMAKE_C_STANDARD 11)

find_package(NanoRos REQUIRED CONFIG)

add_executable({name} src/main.c)
target_link_libraries({name} PRIVATE NanoRos::NanoRos)

install(TARGETS {name} RUNTIME DESTINATION lib/{name})
"#
    );
    fs::write(dir.join("CMakeLists.txt"), cmake)?;

    let main_c = format!(
        r#"#include <stdio.h>

int main(void) {{
    printf("Hello from {name}!\\n");
    return 0;
}}
"#
    );
    fs::write(dir.join("src/main.c"), main_c)?;

    if platform != "native" {
        let config_toml = r#"[network]
ip = "10.0.2.20"
mac = "02:00:00:00:00:00"
gateway = "10.0.2.2"
netmask = "255.255.255.0"

[zenoh]
locator = "tcp/10.0.2.2:7447"
domain_id = 0
"#;
        fs::write(dir.join("config.toml"), config_toml)?;
    }

    Ok(())
}

fn scaffold_cpp(name: &str, platform: &str, dir: &Path) -> Result<()> {
    use std::fs;

    let cmake = format!(
        r#"cmake_minimum_required(VERSION 3.16)
project({name} VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 14)

find_package(NanoRos REQUIRED CONFIG)

add_executable({name} src/main.cpp)
target_link_libraries({name} PRIVATE NanoRos::NanoRosCpp)

install(TARGETS {name} RUNTIME DESTINATION lib/{name})
"#
    );
    fs::write(dir.join("CMakeLists.txt"), cmake)?;

    let main_cpp = format!(
        r#"#include <cstdio>

int main() {{
    printf("Hello from {name}!\\n");
    return 0;
}}
"#
    );
    fs::write(dir.join("src/main.cpp"), main_cpp)?;

    if platform != "native" {
        let config_toml = r#"[network]
ip = "10.0.2.20"
mac = "02:00:00:00:00:00"
gateway = "10.0.2.2"
netmask = "255.255.255.0"

[zenoh]
locator = "tcp/10.0.2.2:7447"
domain_id = 0
"#;
        fs::write(dir.join("config.toml"), config_toml)?;
    }

    Ok(())
}
