//! `nros check` - validate a generated nros-plan.json.

use crate::orchestration::planner::check_plan_file;
use clap::Args as ClapArgs;
use eyre::Result;
use std::path::PathBuf;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Path to nros-plan.json
    #[arg(default_value = "build/nros/nros-plan.json")]
    pub plan: PathBuf,
}

pub fn run(args: Args) -> Result<()> {
    let report = check_plan_file(&args.plan)?;
    if report.errors == 0 {
        eprintln!(
            "nros check: ok ({} warning(s), {})",
            report.warnings,
            args.plan.display()
        );
    }
    Ok(())
}
