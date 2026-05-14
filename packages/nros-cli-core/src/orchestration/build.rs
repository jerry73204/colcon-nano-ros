//! Build orchestration for generated system packages.

use super::generate::{GenerateOptions, GeneratedPackage, generate_package};
use eyre::{Context, Result, eyre};
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub package_name: String,
    pub output_dir: PathBuf,
    pub plan_path: PathBuf,
    pub workspace_root: PathBuf,
    pub release: bool,
    pub target: Option<String>,
    pub cargo_args: Vec<String>,
}

pub fn build_generated_package(options: &BuildOptions) -> Result<GeneratedPackage> {
    let generated = generate_package(&GenerateOptions {
        package_name: options.package_name.clone(),
        output_dir: options.output_dir.clone(),
        plan_path: options.plan_path.clone(),
        nros_path: options.workspace_root.join("packages/core/nros"),
        nros_cli_core_path: options
            .workspace_root
            .join("packages/codegen/packages/nros-cli-core"),
        nros_orchestration_path: options
            .workspace_root
            .join("packages/core/nros-orchestration"),
    })?;

    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--manifest-path")
        .arg(&generated.manifest_path);

    if options.release {
        cmd.arg("--release");
    }
    if let Some(target) = &options.target {
        cmd.arg("--target").arg(target);
    }
    cmd.args(&options.cargo_args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = cmd
        .status()
        .wrap_err("failed to invoke generated cargo build")?;
    if !status.success() {
        return Err(eyre!(
            "generated package build failed (exit {})",
            status.code().unwrap_or(-1)
        ));
    }

    Ok(generated)
}
