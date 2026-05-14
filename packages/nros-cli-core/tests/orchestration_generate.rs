use std::{
    fs,
    path::{Path, PathBuf},
};

use nros_cli_core::orchestration::generate::{GenerateOptions, generate_package};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("orchestration")
        .join(name)
}

fn temp_output(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nros_cli_core_{name}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn generate_fixture(name: &str, plan_fixture: &str) -> PathBuf {
    let output_dir = temp_output(name);
    generate_package(&GenerateOptions {
        package_name: "nros-generated-test".to_string(),
        output_dir: output_dir.clone(),
        plan_path: fixture(plan_fixture),
        nros_path: PathBuf::from("/workspace/packages/core/nros"),
        nros_orchestration_path: PathBuf::from("/workspace/packages/core/nros-orchestration"),
    })
    .expect("generated package writes");
    output_dir
}

#[test]
fn generated_package_writes_manifest_build_script_and_main() {
    let output_dir = generate_fixture("generated_package_writes_files", "plan_pub_sub.json");

    let cargo_toml = fs::read_to_string(output_dir.join("Cargo.toml")).expect("read Cargo.toml");
    assert!(cargo_toml.contains("name = \"nros-generated-test\""));
    assert!(cargo_toml.contains("nros = { path = \"/workspace/packages/core/nros\""));
    assert!(
        cargo_toml.contains(
            "nros-orchestration = { path = \"/workspace/packages/core/nros-orchestration\""
        )
    );
    assert!(!cargo_toml.contains("nros-cli-core"));
    assert!(!cargo_toml.contains("serde_json"));

    let build_rs = fs::read_to_string(output_dir.join("build.rs")).expect("read build.rs");
    assert!(build_rs.contains("const PLAN_PATH: &str ="));
    assert!(build_rs.contains("// Generated from: "));
    assert!(build_rs.contains("pub const CALLBACK_COUNT: usize = 2;"));
    assert!(build_rs.contains("pub const SCHED_CONTEXT_COUNT: usize = 1;"));
    assert!(build_rs.contains("pub static COMPONENTS: [ComponentSpec; 2]"));
    assert!(build_rs.contains("pub static INSTANCES: [InstanceSpec; 2]"));
    assert!(build_rs.contains("pub static PARAMETERS: [ParameterSpec; 1]"));
    assert!(build_rs.contains("pub static SCHED_CONTEXTS: [SchedContextSpec; 1]"));
    assert!(build_rs.contains("pub static CALLBACK_BINDINGS: [CallbackBindingSpec; 2]"));
    assert!(build_rs.contains("pub static SYSTEM: SystemSpec"));
    assert!(build_rs.contains("PlanId("));
    assert!(build_rs.contains("SchedClassSpec::Fifo"));
    assert!(build_rs.contains("PrioritySpec::BestEffort"));
    assert!(build_rs.contains("deadline_policy: DeadlinePolicySpec::Activated"));
    assert!(!build_rs.contains("serde_json"));
    assert!(!build_rs.contains("nros_cli_core"));

    let main_rs = fs::read_to_string(output_dir.join("src/main.rs")).expect("read main.rs");
    assert!(main_rs.contains("Executor::open"));
    assert!(main_rs.contains("create_sched_context(spec.to_nros_node())"));
    assert!(main_rs.contains("instantiate_components"));
    assert!(main_rs.contains("bind_handle_to_sched_context"));
    assert!(main_rs.contains("spin_blocking(SpinOptions::default())"));
}

#[test]
fn generated_package_output_is_stable() {
    let output_dir = generate_fixture("generated_package_output_is_stable", "plan_pub_sub.json");
    let first_cargo = fs::read_to_string(output_dir.join("Cargo.toml")).expect("read Cargo.toml");
    let first_build = fs::read_to_string(output_dir.join("build.rs")).expect("read build.rs");
    let first_main = fs::read_to_string(output_dir.join("src/main.rs")).expect("read main.rs");

    generate_package(&GenerateOptions {
        package_name: "nros-generated-test".to_string(),
        output_dir: output_dir.clone(),
        plan_path: fixture("plan_pub_sub.json"),
        nros_path: PathBuf::from("/workspace/packages/core/nros"),
        nros_orchestration_path: PathBuf::from("/workspace/packages/core/nros-orchestration"),
    })
    .expect("second generated package write");

    assert_eq!(
        first_cargo,
        fs::read_to_string(output_dir.join("Cargo.toml")).expect("reread Cargo.toml")
    );
    assert_eq!(
        first_build,
        fs::read_to_string(output_dir.join("build.rs")).expect("reread build.rs")
    );
    assert_eq!(
        first_main,
        fs::read_to_string(output_dir.join("src/main.rs")).expect("reread main.rs")
    );
}

#[test]
fn generated_tables_cover_multiple_instances_of_same_component() {
    let output_dir = generate_fixture(
        "generated_tables_multi_instance",
        "plan_multi_instance.json",
    );
    let build_rs = fs::read_to_string(output_dir.join("build.rs")).expect("read build.rs");

    assert!(build_rs.contains("pub const CALLBACK_COUNT: usize = 2;"));
    assert!(build_rs.contains("pub static COMPONENTS: [ComponentSpec; 1]"));
    assert!(build_rs.contains("pub static INSTANCES: [InstanceSpec; 2]"));
    assert!(build_rs.contains("pub static PARAMETERS: [ParameterSpec; 2]"));
    assert!(build_rs.contains("left_talker"));
    assert!(build_rs.contains("right_talker"));
    assert!(build_rs.contains("/left/talker"));
    assert!(build_rs.contains("/right/talker"));
    assert!(build_rs.contains("parameter_start: 0, parameter_len: 1"));
    assert!(build_rs.contains("parameter_start: 1, parameter_len: 1"));
    assert!(build_rs.contains("value: ParameterValue::I64(5)"));
    assert!(build_rs.contains("value: ParameterValue::I64(2)"));
    assert!(build_rs.contains("CallbackBindingSpec { callback_index: 0, sched_context_index: 1 }"));
    assert!(build_rs.contains("CallbackBindingSpec { callback_index: 1, sched_context_index: 1 }"));
}
