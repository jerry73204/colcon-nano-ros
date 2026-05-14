//! `nros plan` - generate host-side orchestration plan.

use crate::orchestration::planner::{PlanOptions, plan_system};
use clap::Args as ClapArgs;
use eyre::Result;
use std::path::PathBuf;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// System package name used for build/<system_pkg>/nros output
    pub system_pkg: String,

    /// ROS 2 launch file to parse
    pub launch_file: PathBuf,

    /// Precomputed play_launch record.json to use instead of parsing launch_file
    #[arg(long)]
    pub record: Option<PathBuf>,

    /// Workspace root containing colcon-like src/* packages
    #[arg(long)]
    pub workspace: Option<PathBuf>,

    /// Output root for orchestration artifacts
    #[arg(long)]
    pub out_dir: Option<PathBuf>,

    /// Existing source metadata JSON artifact
    #[arg(long = "metadata")]
    pub metadata: Vec<PathBuf>,

    /// ROS launch manifest YAML artifact
    #[arg(long = "manifest")]
    pub manifests: Vec<PathBuf>,

    /// nano-ros deployment overlay TOML
    #[arg(long = "nros-toml")]
    pub nros_toml: Vec<PathBuf>,

    /// Launch arguments forwarded as name:=value or name=value
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub launch_args: Vec<String>,
}

pub fn run(args: Args) -> Result<()> {
    let workspace_root = args.workspace.unwrap_or(std::env::current_dir()?);
    let out_root = args.out_dir.unwrap_or_else(|| {
        workspace_root
            .join("build")
            .join(&args.system_pkg)
            .join("nros")
    });
    let output = plan_system(PlanOptions {
        system_pkg: args.system_pkg,
        workspace_root,
        launch_file: args.launch_file,
        record_file: args.record,
        out_root,
        metadata_files: args.metadata,
        manifest_files: args.manifests,
        nros_toml_files: args.nros_toml,
        launch_args: args.launch_args,
    })?;

    eprintln!(
        "nros plan: wrote {} and {}",
        output.record_path.display(),
        output.plan_path.display()
    );
    Ok(())
}
