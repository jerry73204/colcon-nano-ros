//! Build orchestration for generated system packages.

use super::generate::{GenerateOptions, GeneratedPackage, generate_package};
use super::{NrosPlan, plan::PlanBuildOptions};
use eyre::{Context, Result, eyre};
use std::fs;
use std::path::{Path, PathBuf};
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
    let plan = load_plan(&options.plan_path)?;
    let generated = generate_package(&GenerateOptions {
        package_name: options.package_name.clone(),
        output_dir: options.output_dir.clone(),
        plan_path: options.plan_path.clone(),
        nros_path: options.workspace_root.join("packages/core/nros"),
        nros_orchestration_path: options
            .workspace_root
            .join("packages/core/nros-orchestration"),
    })?;

    let mut cmd = Command::new("cargo");
    cmd.args(generated_cargo_args(
        &generated.manifest_path,
        &generated_target_dir(&generated.root),
        &plan.build,
        options.release,
        options.target.as_deref(),
        &options.cargo_args,
    ))
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

fn load_plan(path: &Path) -> Result<NrosPlan> {
    let raw =
        fs::read_to_string(path).wrap_err_with(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).wrap_err_with(|| format!("failed to parse {}", path.display()))
}

fn generated_cargo_args(
    manifest_path: &Path,
    target_dir: &Path,
    build: &PlanBuildOptions,
    release_override: bool,
    target_override: Option<&str>,
    passthrough: &[String],
) -> Vec<String> {
    let mut args = vec![
        "build".to_string(),
        "--manifest-path".to_string(),
        manifest_path.display().to_string(),
        "--target-dir".to_string(),
        target_dir.display().to_string(),
    ];

    match target_override {
        Some(target) if !target.is_empty() => {
            args.push("--target".to_string());
            args.push(target.to_string());
        }
        _ if !build.target.is_empty() => {
            args.push("--target".to_string());
            args.push(build.target.clone());
        }
        _ => {}
    }

    if release_override || build.profile == "release" {
        args.push("--release".to_string());
    } else if !matches!(build.profile.as_str(), "" | "debug" | "dev") {
        args.push("--profile".to_string());
        args.push(build.profile.clone());
    }

    args.extend(passthrough.iter().cloned());
    args
}

fn generated_target_dir(generated_root: &Path) -> PathBuf {
    generated_root
        .parent()
        .map(|parent| parent.join("target"))
        .unwrap_or_else(|| generated_root.join("target"))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::orchestration::NrosPlan;

    use super::{generated_cargo_args, generated_target_dir};

    fn fixture_plan(name: &str) -> NrosPlan {
        let raw = std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures")
                .join("orchestration")
                .join(name),
        )
        .expect("read plan fixture");
        serde_json::from_str(&raw).expect("parse plan fixture")
    }

    #[test]
    fn generated_cargo_args_use_plan_target_and_profile() {
        let plan = fixture_plan("plan_pub_sub.json");

        assert_eq!(
            generated_cargo_args(
                PathBuf::from("/tmp/generated/Cargo.toml").as_path(),
                PathBuf::from("/tmp/target").as_path(),
                &plan.build,
                false,
                None,
                &[],
            ),
            [
                "build",
                "--manifest-path",
                "/tmp/generated/Cargo.toml",
                "--target-dir",
                "/tmp/target",
                "--target",
                "x86_64-unknown-linux-gnu",
                "--release",
            ]
        );
    }

    #[test]
    fn generated_cargo_args_allow_cli_target_and_passthrough_overrides() {
        let plan = fixture_plan("plan_pub_sub.json");

        assert_eq!(
            generated_cargo_args(
                PathBuf::from("/tmp/generated/Cargo.toml").as_path(),
                PathBuf::from("/tmp/target").as_path(),
                &plan.build,
                false,
                Some("thumbv7em-none-eabihf"),
                &["--offline".to_string(), "--quiet".to_string()],
            ),
            [
                "build",
                "--manifest-path",
                "/tmp/generated/Cargo.toml",
                "--target-dir",
                "/tmp/target",
                "--target",
                "thumbv7em-none-eabihf",
                "--release",
                "--offline",
                "--quiet",
            ]
        );
    }

    #[test]
    fn generated_cargo_args_emit_custom_plan_profile() {
        let mut plan = fixture_plan("plan_pub_sub.json");
        plan.build.profile = "size".to_string();

        assert_eq!(
            generated_cargo_args(
                PathBuf::from("/tmp/generated/Cargo.toml").as_path(),
                PathBuf::from("/tmp/target").as_path(),
                &plan.build,
                false,
                None,
                &[],
            ),
            [
                "build",
                "--manifest-path",
                "/tmp/generated/Cargo.toml",
                "--target-dir",
                "/tmp/target",
                "--target",
                "x86_64-unknown-linux-gnu",
                "--profile",
                "size",
            ]
        );
    }

    #[test]
    fn generated_target_dir_matches_system_layout() {
        assert_eq!(
            generated_target_dir(PathBuf::from("/tmp/system/nros/generated").as_path()),
            PathBuf::from("/tmp/system/nros/target")
        );
    }
}
