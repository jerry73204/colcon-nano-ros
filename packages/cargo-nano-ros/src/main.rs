//! cargo-nano-ros: Standalone build tool for nros
//!
//! Generate ROS 2 message bindings from package.xml dependencies.

use cargo_nano_ros::GenerateConfig;
use clap::{Parser, Subcommand};
use eyre::Result;
use std::path::PathBuf;

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
        }
        | NanoRosCommand::Generate {
            manifest_path,
            output,
            config,
            nano_ros_path,
            nano_ros_git,
            force,
            ros_edition,
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

        NanoRosCommand::Clean { output, config } => {
            cargo_nano_ros::clean_generated(&output, config, args.verbose)?;
            println!("✓ Cleaned successfully");
        }
    }

    Ok(())
}
