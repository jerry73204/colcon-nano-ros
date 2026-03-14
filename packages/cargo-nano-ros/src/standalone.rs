//! nano-ros: Standalone build tool for nano-ros
//!
//! This is the standalone binary that can be invoked directly as `nano-ros`.
//! It provides the same functionality as `cargo nano-ros` but without
//! requiring Cargo's subcommand infrastructure.

use cargo_nano_ros::GenerateConfig;
use clap::{Parser, Subcommand};
use eyre::Result;
use std::path::PathBuf;

/// Standalone build tool for nano-ros
#[derive(Debug, Parser)]
#[command(name = "nano-ros")]
#[command(about = "Build tool for nano-ros: generate message bindings from package.xml")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
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

    /// Generate C bindings from package.xml or JSON arguments file
    ///
    /// In standalone mode (default): reads package.xml, resolves interface
    /// dependencies, and generates C code in the output directory.
    /// With --args-file: reads a JSON arguments file (for CMake integration).
    GenerateC {
        /// Path to JSON arguments file (for CMake integration).
        /// When provided, --manifest-path and --output are ignored.
        #[arg(long)]
        args_file: Option<PathBuf>,

        /// Path to package.xml (default: ./package.xml)
        #[arg(long, default_value = "package.xml")]
        manifest_path: PathBuf,

        /// Output directory for generated bindings (default: ./generated)
        #[arg(long, short, default_value = "generated")]
        output: PathBuf,

        /// Overwrite existing bindings
        #[arg(long)]
        force: bool,

        /// ROS 2 edition for type hash format (humble or iron)
        #[arg(long, default_value = "humble")]
        ros_edition: String,
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
    let cli = Cli::parse();

    match cli.command {
        Command::GenerateRust {
            manifest_path,
            output,
            config,
            nano_ros_path,
            nano_ros_git,
            force,
            ros_edition,
        }
        | Command::Generate {
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
                verbose: cli.verbose,
                ros_edition,
            })?;
        }

        Command::GenerateC {
            args_file,
            manifest_path,
            output,
            force,
            ros_edition,
        } => {
            if let Some(args_file) = args_file {
                // CMake mode: use JSON args file
                let cfg = cargo_nano_ros::GenerateCConfig {
                    args_file,
                    verbose: cli.verbose,
                };
                cargo_nano_ros::generate_c_from_args_file(cfg)?;
                println!("✓ C bindings generated successfully");
            } else {
                // Standalone mode: read package.xml
                let cfg = cargo_nano_ros::GenerateCStandaloneConfig {
                    manifest_path,
                    output_dir: output,
                    force,
                    verbose: cli.verbose,
                    ros_edition,
                };
                cargo_nano_ros::generate_c_from_package_xml(cfg)?;
            }
        }

        Command::Bindgen {
            package,
            output,
            package_path,
        } => {
            let cfg = cargo_nano_ros::BindgenConfig {
                package_name: package,
                package_path,
                output_dir: output,
                verbose: cli.verbose,
            };
            cargo_nano_ros::generate_bindings(cfg)?;
            println!("✓ Bindings generated successfully");
        }

        Command::Clean { output, config } => {
            cargo_nano_ros::clean_generated(&output, config, cli.verbose)?;
            println!("✓ Cleaned successfully");
        }
    }

    Ok(())
}
