//! Generated orchestration package writer.
//!
//! This module deliberately treats `nros-plan.json` as an opaque input path.
//! Agent A owns the final plan schema; generated package `build.rs` is the
//! host-side adapter that will be tightened once that schema lands.

use eyre::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use super::{
    NrosPlan,
    plan::PlanSchedContext,
    schema::{DeadlinePolicy, ParameterValue, SchedClass},
};

const CARGO_TEMPLATE: &str = include_str!("../../templates/orchestration/Cargo.toml.jinja");
const BUILD_TEMPLATE: &str = include_str!("../../templates/orchestration/build.rs.jinja");
const MAIN_TEMPLATE: &str = include_str!("../../templates/orchestration/main.rs.jinja");

#[derive(Debug, Clone)]
pub struct GenerateOptions {
    pub package_name: String,
    pub output_dir: PathBuf,
    pub plan_path: PathBuf,
    pub nros_path: PathBuf,
    pub nros_orchestration_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct GeneratedPackage {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub plan_path: PathBuf,
}

pub fn generate_package(options: &GenerateOptions) -> Result<GeneratedPackage> {
    let src_dir = options.output_dir.join("src");
    fs::create_dir_all(&src_dir).wrap_err_with(|| {
        format!(
            "failed to create generated package src dir {}",
            src_dir.display()
        )
    })?;

    let cargo_toml = render_cargo_toml(options);
    let build_rs = render_build_rs(options);

    write_if_changed(&options.output_dir.join("Cargo.toml"), &cargo_toml)?;
    write_if_changed(&options.output_dir.join("build.rs"), &build_rs)?;
    write_if_changed(&src_dir.join("main.rs"), MAIN_TEMPLATE)?;

    Ok(GeneratedPackage {
        root: options.output_dir.clone(),
        manifest_path: options.output_dir.join("Cargo.toml"),
        plan_path: options.plan_path.clone(),
    })
}

fn render_cargo_toml(options: &GenerateOptions) -> String {
    CARGO_TEMPLATE
        .replace("{{ package_name }}", &options.package_name)
        .replace("{{ nros_path }}", &path_for_template(&options.nros_path))
        .replace(
            "{{ nros_orchestration_path }}",
            &path_for_template(&options.nros_orchestration_path),
        )
}

fn render_build_rs(options: &GenerateOptions) -> String {
    let plan = load_plan(&options.plan_path).expect("load nros-plan.json for generated package");
    let generated_tables = render_generated_tables(&plan);
    BUILD_TEMPLATE
        .replace("{{ plan_path }}", &path_for_template(&options.plan_path))
        .replace(
            "{{ generated_tables_literal }}",
            &format!("{generated_tables:?}"),
        )
}

fn path_for_template(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

fn write_if_changed(path: &Path, contents: &str) -> Result<()> {
    if fs::read_to_string(path).ok().as_deref() == Some(contents) {
        return Ok(());
    }
    fs::write(path, contents).wrap_err_with(|| format!("failed to write {}", path.display()))
}

fn load_plan(path: &Path) -> Result<NrosPlan> {
    let raw =
        fs::read_to_string(path).wrap_err_with(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).wrap_err_with(|| format!("failed to parse {}", path.display()))
}

fn render_generated_tables(plan: &NrosPlan) -> String {
    let schema = format!("nano-ros/plan/v{}", plan.version);
    let callback_count = plan
        .instances
        .iter()
        .map(|instance| instance.callbacks.len())
        .sum::<usize>();
    let max_nodes = plan
        .instances
        .iter()
        .map(|instance| instance.nodes.len())
        .sum::<usize>();
    let max_sched_contexts = plan.sched_contexts.len() + 1;
    let max_parameters = plan
        .instances
        .iter()
        .map(|instance| instance.parameters.len())
        .sum::<usize>();
    let max_interfaces = plan.interfaces.len();

    let mut out = String::new();
    out.push_str("use nros_orchestration::{CallbackBindingSpec, CapacitySpec, ComponentLanguage, PlanId, SchedClassSpec, SchedContextSpec, SystemSpec};\n");
    out.push_str("use nros_orchestration::{CallbackHandleTable, ComponentSpec, InstanceSpec, ParameterSpec, ParameterValue};\n");
    out.push_str("use nros_orchestration::{DeadlinePolicySpec, PrioritySpec};\n\n");
    out.push_str(&format!(
        "pub const CALLBACK_COUNT: usize = {callback_count};\n"
    ));
    out.push_str(&format!(
        "pub const SCHED_CONTEXT_COUNT: usize = {};\n\n",
        plan.sched_contexts.len()
    ));
    render_components(&mut out, plan);
    render_instances(&mut out, plan);
    render_parameters(&mut out, plan);
    out.push_str(&format!(
        "pub static SCHED_CONTEXTS: [SchedContextSpec; {}] = [\n",
        plan.sched_contexts.len()
    ));
    for sc in &plan.sched_contexts {
        out.push_str(&render_sched_context(sc));
    }
    out.push_str("];\n\n");
    let bindings = collect_callback_bindings(plan);
    out.push_str(&format!(
        "pub static CALLBACK_BINDINGS: [CallbackBindingSpec; {}] = [\n",
        bindings.len()
    ));
    for (callback_index, sched_context_index) in bindings {
        out.push_str(&format!(
            "    CallbackBindingSpec {{ callback_index: {callback_index}, sched_context_index: {sched_context_index} }},\n"
        ));
    }
    out.push_str("];\n\n");
    out.push_str(&format!(
        "pub static SYSTEM: SystemSpec = SystemSpec {{ schema: {schema:?}, plan_id: PlanId({plan_id}), capacities: CapacitySpec {{ max_nodes: {max_nodes}, max_callbacks: {callback_count}, max_sched_contexts: {max_sched_contexts}, max_parameters: {max_parameters}, max_interfaces: {max_interfaces} }}, components: &COMPONENTS, instances: &INSTANCES, parameters: &PARAMETERS, sched_contexts: &SCHED_CONTEXTS, callback_bindings: &CALLBACK_BINDINGS }};\n\n",
        plan_id = stable_plan_id(plan),
    ));
    out.push_str("pub fn instantiate_components(_executor: &mut nros::Executor, _handles: &mut CallbackHandleTable<CALLBACK_COUNT>) -> Result<(), nros::NodeError> {\n");
    out.push_str("    Ok(())\n");
    out.push_str("}\n");
    out
}

fn render_components(out: &mut String, plan: &NrosPlan) {
    out.push_str(&format!(
        "pub static COMPONENTS: [ComponentSpec; {}] = [\n",
        plan.components.len()
    ));
    for component in &plan.components {
        out.push_str(&format!(
            "    ComponentSpec {{ id: {id:?}, package: {package:?}, symbol: {symbol:?}, language: ComponentLanguage::{language} }},\n",
            id = component.id,
            package = component.package,
            symbol = component.component,
            language = component_language(&component.language),
        ));
    }
    out.push_str("];\n\n");
}

fn render_instances(out: &mut String, plan: &NrosPlan) {
    out.push_str(&format!(
        "pub static INSTANCES: [InstanceSpec; {}] = [\n",
        plan.instances.len()
    ));
    let mut parameter_start = 0usize;
    for instance in &plan.instances {
        let parameter_len = instance.parameters.len();
        let node_name = instance
            .nodes
            .first()
            .map(|node| node.resolved_name.as_str())
            .unwrap_or(instance.launch_name.as_str());
        out.push_str(&format!(
            "    InstanceSpec {{ id: {id:?}, component_id: {component:?}, node_name: {node_name:?}, namespace: {namespace:?}, domain_id: None, parameter_start: {parameter_start}, parameter_len: {parameter_len} }},\n",
            id = instance.id,
            component = instance.component,
            namespace = instance.namespace,
        ));
        parameter_start += parameter_len;
    }
    out.push_str("];\n\n");
}

fn render_parameters(out: &mut String, plan: &NrosPlan) {
    let rendered_parameters = plan
        .instances
        .iter()
        .flat_map(|instance| {
            instance.parameters.iter().filter_map(move |parameter| {
                render_parameter_value(&parameter.value).map(|value| {
                    format!(
                        "    ParameterSpec {{ instance_id: {instance_id:?}, name: {name:?}, value: {value} }},\n",
                        instance_id = instance.id,
                        name = parameter.name,
                    )
                })
            })
        })
        .collect::<Vec<_>>();
    out.push_str(&format!(
        "pub static PARAMETERS: [ParameterSpec; {}] = [\n",
        rendered_parameters.len()
    ));
    for parameter in rendered_parameters {
        out.push_str(&parameter);
    }
    out.push_str("];\n\n");
}

fn render_sched_context(sc: &PlanSchedContext) -> String {
    format!(
        "    SchedContextSpec {{ id: {id:?}, class: SchedClassSpec::{class}, priority: PrioritySpec::{priority}, period_us: {period}, budget_us: {budget}, deadline_us: {deadline}, deadline_policy: DeadlinePolicySpec::{deadline_policy}, os_pri: {os_pri}, tt_window_offset_us: {tt_offset}, tt_window_duration_us: {tt_duration} }},\n",
        id = sc.id,
        class = sched_class(&sc.class),
        priority = priority(sc.priority),
        period = option_ms_to_us(sc.period_ms),
        budget = option_ms_to_us(sc.budget_ms),
        deadline = option_ms_to_us(sc.deadline_ms),
        deadline_policy = deadline_policy(&sc.deadline_policy),
        os_pri = sc.priority.unwrap_or(0),
        tt_offset = "None",
        tt_duration = option_ms_to_us(match sc.class {
            SchedClass::TimeTriggered => sc.period_ms,
            _ => None,
        }),
    )
}

fn collect_callback_bindings(plan: &NrosPlan) -> Vec<(usize, usize)> {
    let mut bindings = Vec::new();
    let mut callback_index = 0usize;
    for instance in &plan.instances {
        for callback in &instance.callbacks {
            let sched_context_index = plan
                .sched_contexts
                .iter()
                .position(|context| context.id == callback.sched_context)
                .map(|index| index + 1)
                .unwrap_or(0);
            bindings.push((callback_index, sched_context_index));
            callback_index += 1;
        }
    }
    bindings
}

fn component_language(raw: &str) -> &'static str {
    match raw {
        "rust" | "Rust" => "Rust",
        "c" | "C" => "C",
        "cpp" | "c++" | "Cpp" => "Cpp",
        _ => "Rust",
    }
}

fn render_parameter_value(value: &ParameterValue) -> Option<String> {
    match value {
        ParameterValue::Bool(value) => Some(format!("ParameterValue::Bool({value})")),
        ParameterValue::Integer(value) => Some(format!("ParameterValue::I64({value})")),
        ParameterValue::Float(value) => Some(format!("ParameterValue::F64({value:?})")),
        ParameterValue::String(value) => Some(format!("ParameterValue::Str({value:?})")),
        _ => None,
    }
}

fn sched_class(class: &SchedClass) -> &'static str {
    match class {
        SchedClass::BestEffort => "BestEffort",
        SchedClass::RealTime => "Fifo",
        SchedClass::TimeTriggered => "Fifo",
        SchedClass::Interrupt => "Fifo",
    }
}

fn priority(priority: Option<u8>) -> &'static str {
    match priority {
        Some(0..=63) => "BestEffort",
        Some(64..=191) => "Normal",
        Some(_) => "Critical",
        None => "Normal",
    }
}

fn deadline_policy(policy: &DeadlinePolicy) -> &'static str {
    match policy {
        DeadlinePolicy::Ignore => "Activated",
        DeadlinePolicy::Warn => "Activated",
        DeadlinePolicy::Skip => "Activated",
        DeadlinePolicy::Fault => "Activated",
    }
}

fn option_ms_to_us(value: Option<u64>) -> String {
    match value
        .and_then(|ms| ms.checked_mul(1_000))
        .and_then(|us| u32::try_from(us).ok())
    {
        Some(us) => format!("Some({us})"),
        None => "None".to_string(),
    }
}

fn stable_plan_id(plan: &NrosPlan) -> u32 {
    let mut hash = 0x811c9dc5u32;
    for byte in plan.system.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}
