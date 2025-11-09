mod ament;
mod generator;

use clap::Parser;
use eyre::{eyre, Result, WrapErr};
use std::path::PathBuf;

/// Generate Rust bindings for ROS 2 interface packages
#[derive(Parser, Debug)]
#[command(name = "cargo-ros2-bindgen")]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the ROS 2 package to generate bindings for
    #[arg(short, long)]
    package: String,

    /// Output directory for generated bindings
    #[arg(short, long)]
    output: PathBuf,

    /// Direct path to package share directory (bypasses ament index)
    #[arg(long)]
    package_path: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.verbose {
        eprintln!("cargo-ros2-bindgen starting...");
        eprintln!("  Package: {}", args.package);
        eprintln!("  Output: {}", args.output.display());
    }

    // Get the package
    let package = if let Some(package_path) = args.package_path {
        // Direct path mode
        if args.verbose {
            eprintln!("  Using direct path: {}", package_path.display());
        }
        ament::Package::from_share_dir(package_path)
            .wrap_err("Failed to load package from direct path")?
    } else {
        // Ament index mode
        if args.verbose {
            eprintln!("  Discovering package via ament index...");
        }
        let index = ament::AmentIndex::from_env().wrap_err("Failed to create ament index")?;

        if args.verbose {
            eprintln!("  Found {} packages in ament index", index.package_count());
        }

        index
            .find_package(&args.package)
            .ok_or_else(|| eyre!("Package '{}' not found in ament index", args.package))?
            .clone()
    };

    if args.verbose {
        eprintln!("  Package share dir: {}", package.share_dir.display());
        eprintln!("  Messages: {}", package.interfaces.messages.len());
        eprintln!("  Services: {}", package.interfaces.services.len());
        eprintln!("  Actions: {}", package.interfaces.actions.len());
    }

    // Generate bindings
    if args.verbose {
        eprintln!("Generating Rust bindings...");
    }

    let generated = generator::generate_package(&package, &args.output)
        .wrap_err("Failed to generate package")?;

    if args.verbose {
        eprintln!("Generation complete!");
        eprintln!("  Output directory: {}", generated.output_dir.display());
        eprintln!("  Messages generated: {}", generated.message_count);
        eprintln!("  Services generated: {}", generated.service_count);
        eprintln!("  Actions generated: {}", generated.action_count);
    } else {
        // Minimal output for non-verbose mode
        println!(
            "Generated bindings for '{}' to {}",
            generated.name,
            generated.output_dir.display()
        );
    }

    Ok(())
}
