use cargo_ros2::workflow::WorkflowContext;
use clap::{Parser, Subcommand};
use eyre::{eyre, Result, WrapErr};
use std::env;
use std::path::{Path, PathBuf};

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

    // Create workflow context
    let ctx = WorkflowContext::new(project_root, args.verbose);

    match args.command {
        Ros2Command::Bindgen {
            package,
            output,
            package_path,
            verbose,
        } => {
            run_bindgen(&package, &output, package_path.as_deref(), verbose)?;
        }

        Ros2Command::Install {
            install_base,
            profile,
        } => {
            install_to_ament(&ctx, &install_base, &profile)?;
        }

        Ros2Command::Clean => {
            clean_bindings(&ctx)?;
            println!("✓ Cleaned bindings and cache!");
        }
    }

    Ok(())
}

/// Run rosidl-bindgen to generate bindings for a single package
fn run_bindgen(
    package: &str,
    output: &Path,
    package_path: Option<&Path>,
    verbose: bool,
) -> Result<()> {
    use std::process::Command;

    println!("Generating bindings for {}...", package);

    // Build rosidl-bindgen command
    let mut cmd = Command::new("rosidl-bindgen");
    cmd.arg("--package")
        .arg(package)
        .arg("--output")
        .arg(output);

    if let Some(path) = package_path {
        cmd.arg("--package-path").arg(path);
    }

    if verbose {
        cmd.arg("--verbose");
    }

    // Execute
    let status = cmd
        .status()
        .wrap_err("Failed to execute rosidl-bindgen. Is it installed?")?;

    if !status.success() {
        return Err(eyre!("rosidl-bindgen failed for package '{}'", package));
    }

    println!("✓ Bindings generated to {}", output.display());
    Ok(())
}

/// Install binaries and create ament layout
fn install_to_ament(_ctx: &WorkflowContext, install_base: &Path, profile: &str) -> Result<()> {
    use cargo_metadata::MetadataCommand;
    use cargo_ros2::ament_installer::{is_library_package, AmentInstaller};
    use std::env;

    println!("Installing package to ament layout...");

    // Read package metadata
    let metadata = MetadataCommand::new()
        .exec()
        .wrap_err("Failed to read Cargo metadata")?;

    let root_package = metadata
        .root_package()
        .ok_or_else(|| eyre!("No root package found in Cargo.toml"))?;

    let package_name = root_package.name.clone();
    let project_root = env::current_dir()?;

    // Check if this is a library-only package
    let is_lib_only = is_library_package(&project_root)?;

    // Create installer
    let installer = AmentInstaller::new(
        install_base.to_path_buf(),
        package_name.clone(),
        project_root,
        false, // verbose
        profile.to_string(),
    );

    // Run installation
    installer.install(is_lib_only)?;

    println!("✓ Package installed to {}", install_base.display());
    Ok(())
}

fn clean_bindings(ctx: &WorkflowContext) -> Result<()> {
    // Remove bindings directory
    if ctx.output_dir.exists() {
        std::fs::remove_dir_all(&ctx.output_dir)?;
        if ctx.verbose {
            eprintln!("Removed {}", ctx.output_dir.display());
        }
    }

    // Remove cache file
    if ctx.cache_file.exists() {
        std::fs::remove_file(&ctx.cache_file)?;
        if ctx.verbose {
            eprintln!("Removed {}", ctx.cache_file.display());
        }
    }

    // Remove .cargo/config.toml patches (TODO: only remove ROS patches, not entire file)
    let cargo_config = ctx.project_root.join(".cargo").join("config.toml");
    if cargo_config.exists() && ctx.verbose {
        eprintln!("Note: .cargo/config.toml patches not removed (would need selective removal)");
    }

    Ok(())
}
