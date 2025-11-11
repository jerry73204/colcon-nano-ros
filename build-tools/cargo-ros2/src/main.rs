use cargo_ros2::{BindgenConfig, InstallConfig};
use clap::{Parser, Subcommand};
use eyre::Result;
use std::env;
use std::path::PathBuf;

/// All-in-one build tool for ROS 2 Rust projects
#[derive(Parser, Debug)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum CargoCli {
    Ros2(Ros2Args),
}

#[derive(Debug, Parser)]
#[command(name = "ros2")]
#[command(about = "Build tool for ROS 2 Rust projects", long_about = None)]
struct Ros2Args {
    #[command(subcommand)]
    command: Ros2Command,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Debug, Subcommand)]
enum Ros2Command {
    /// Generate Rust bindings for a ROS 2 interface package
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

        /// Verbose output
        #[arg(long)]
        verbose: bool,
    },

    /// Install binaries and libraries to ament layout
    Install {
        /// Install base directory (install/<package>/)
        #[arg(long)]
        install_base: PathBuf,

        /// Build profile (debug or release)
        #[arg(long, default_value = "debug")]
        profile: String,
    },

    /// Clean generated bindings and cache
    Clean,
}

fn main() -> Result<()> {
    let CargoCli::Ros2(args) = CargoCli::parse();

    // Get project root (current directory)
    let project_root = env::current_dir()?;

    match args.command {
        Ros2Command::Bindgen {
            package,
            output,
            package_path,
            verbose,
        } => {
            let config = BindgenConfig {
                package_name: package,
                package_path,
                output_dir: output,
                verbose: verbose || args.verbose,
            };
            cargo_ros2::generate_bindings(config)?;
            println!("✓ Bindings generated successfully");
        }

        Ros2Command::Install {
            install_base,
            profile,
        } => {
            let config = InstallConfig {
                project_root,
                install_base,
                profile,
                verbose: args.verbose,
            };
            cargo_ros2::install_to_ament(config)?;
            println!("✓ Package installed successfully");
        }

        Ros2Command::Clean => {
            cargo_ros2::clean_bindings(&project_root, args.verbose)?;
            println!("✓ Cleaned bindings and cache!");
        }
    }

    Ok(())
}
