//! `nros build` — Phase 111.A.9.
//!
//! Auto-detect the project flavor and delegate. Detection precedence
//! (highest first), evaluated in the project root (cwd or `--project`):
//!
//!   1. `prj.conf` present → Zephyr → `west build`
//!   2. `CMakeLists.txt` present + no `Cargo.toml` → `cmake -B build && cmake --build build`
//!   3. `Cargo.toml` present → `cargo build`
//!
//! Mixed projects (Cargo.toml AND CMakeLists.txt) — common when a Rust
//! crate produces a `staticlib` consumed by C/C++ — go through the
//! cmake path. Heuristic: if `[lib].crate-type` in Cargo.toml contains
//! `staticlib` AND CMakeLists.txt exists, prefer cmake.

use clap::Args as ClapArgs;
use eyre::{Result, WrapErr, eyre};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Path to the project root (default: cwd)
    #[arg(long)]
    pub project: Option<PathBuf>,

    /// Trailing arguments forwarded verbatim to the underlying tool
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub passthrough: Vec<String>,
}

pub fn run(args: Args) -> Result<()> {
    let root = match args.project {
        Some(p) => p,
        None => std::env::current_dir()?,
    };
    let flavor = detect_flavor(&root)?;
    eprintln!("nros build: flavor = {flavor:?} ({})", root.display());

    let mut cmd = match flavor {
        Flavor::West => {
            let mut c = Command::new("west");
            c.arg("build");
            c
        }
        Flavor::Cmake => {
            // `cmake -B build && cmake --build build` chained as one
            // shell, but we keep them as two child processes so we don't
            // need a shell.
            let configure = Command::new("cmake")
                .current_dir(&root)
                .args(["-B", "build", "-S", "."])
                .args(&args.passthrough)
                .status()
                .wrap_err("failed to invoke `cmake -B build`")?;
            if !configure.success() {
                return Err(eyre!(
                    "cmake configure failed (exit {})",
                    configure.code().unwrap_or(-1)
                ));
            }
            let mut c = Command::new("cmake");
            c.arg("--build").arg("build");
            c
        }
        Flavor::Cargo => {
            let mut c = Command::new("cargo");
            c.arg("build");
            c
        }
    };
    if !matches!(flavor, Flavor::Cmake) {
        cmd.args(&args.passthrough);
    }
    cmd.current_dir(&root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = cmd
        .status()
        .wrap_err_with(|| format!("failed to invoke build for {flavor:?}"))?;
    if !status.success() {
        return Err(eyre!(
            "build failed (exit {})",
            status.code().unwrap_or(-1)
        ));
    }
    Ok(())
}

#[derive(Debug)]
enum Flavor {
    West,
    Cmake,
    Cargo,
}

fn detect_flavor(root: &Path) -> Result<Flavor> {
    let has_prj_conf = root.join("prj.conf").is_file();
    let has_cmake = root.join("CMakeLists.txt").is_file();
    let cargo_toml = root.join("Cargo.toml");
    let has_cargo = cargo_toml.is_file();

    if has_prj_conf {
        return Ok(Flavor::West);
    }

    if has_cmake && has_cargo && produces_staticlib(&cargo_toml).unwrap_or(false) {
        return Ok(Flavor::Cmake);
    }
    if has_cargo {
        return Ok(Flavor::Cargo);
    }
    if has_cmake {
        return Ok(Flavor::Cmake);
    }
    Err(eyre!(
        "no build flavor detected at {}: expected prj.conf (Zephyr), \
         CMakeLists.txt (CMake), or Cargo.toml (Rust)",
        root.display()
    ))
}

fn produces_staticlib(cargo_toml: &Path) -> Result<bool> {
    let raw = fs::read_to_string(cargo_toml)?;
    let doc: toml::Value = toml::from_str(&raw)?;
    let Some(lib) = doc.get("lib") else {
        return Ok(false);
    };
    let Some(crate_type) = lib.get("crate-type").or_else(|| lib.get("crate_type")) else {
        return Ok(false);
    };
    Ok(match crate_type {
        toml::Value::Array(arr) => arr.iter().any(|v| v.as_str() == Some("staticlib")),
        toml::Value::String(s) => s == "staticlib",
        _ => false,
    })
}
