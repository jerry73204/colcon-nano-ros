//! Draft host planner for Phase 126.C.

use super::manifest::{ManifestArtifact, endpoint_requirements, load_manifest};
use super::names;
use super::params::{ParameterInputs, effective_parameters, load_toml_values};
use super::workspace::{Workspace, unique_paths};
use eyre::{Context, Result, eyre};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PlanOptions {
    pub system_pkg: String,
    pub workspace_root: PathBuf,
    pub launch_file: PathBuf,
    pub record_file: Option<PathBuf>,
    pub out_root: PathBuf,
    pub metadata_files: Vec<PathBuf>,
    pub manifest_files: Vec<PathBuf>,
    pub nros_toml_files: Vec<PathBuf>,
    pub launch_args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PlanningOutput {
    pub record_path: PathBuf,
    pub plan_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CheckReport {
    pub errors: usize,
    pub warnings: usize,
}

#[derive(Debug, Clone)]
struct JsonArtifact {
    path: PathBuf,
    value: Value,
}

pub fn plan_system(options: PlanOptions) -> Result<PlanningOutput> {
    fs::create_dir_all(&options.out_root)?;
    let metadata_dir = options.out_root.join("metadata");
    fs::create_dir_all(&metadata_dir)?;

    let workspace = Workspace::discover(&options.workspace_root)?;
    let launch_args = parse_launch_args(&options.launch_args)?;
    let record = load_or_parse_record(
        &options.launch_file,
        options.record_file.as_deref(),
        launch_args,
    )?;

    let record_path = options.out_root.join("record.json");
    fs::write(&record_path, serde_json::to_string_pretty(&record)?)?;

    let metadata_paths = metadata_paths(&options, &workspace, &metadata_dir);
    let metadata = load_json_artifacts(&metadata_paths, "source metadata")?;
    preserve_metadata(&metadata, &metadata_dir)?;

    let manifest_paths = if options.manifest_files.is_empty() {
        workspace.manifest_files()
    } else {
        unique_paths(options.manifest_files.clone())
    };
    let manifests = manifest_paths
        .iter()
        .map(|path| load_manifest(path))
        .collect::<Result<Vec<_>>>()?;

    let mut nros_toml = options.nros_toml_files.clone();
    if let Some(system_toml) = workspace.package_nros_toml(&options.system_pkg) {
        nros_toml.push(system_toml);
    }
    let overlays = load_toml_values(&unique_paths(nros_toml))?;

    let (instances, mut diagnostics) =
        build_instances(&record, &metadata, &workspace, &overlays, &record_path);
    diagnostics.extend(check_manifest_endpoints(
        &instances,
        &manifests,
        &metadata,
        &record_path,
    ));

    let plan = json!({
        "format": "nros.plan.draft",
        "schema_owner": "phase-126A",
        "system_package": options.system_pkg,
        "workspace_root": options.workspace_root,
        "launch_file": options.launch_file,
        "record": {
            "path": record_path,
            "node_count": record_array(&record, "node").len(),
            "load_node_count": record_array(&record, "load_node").len(),
            "container_count": record_array(&record, "container").len(),
        },
        "source_metadata": artifact_summaries(&metadata),
        "launch_manifests": manifest_summaries(&manifests),
        "manifest_requirements": endpoint_requirements(&manifests),
        "instances": instances,
        "diagnostics": diagnostics,
    });

    let plan_path = options.out_root.join("nros-plan.json");
    fs::write(&plan_path, serde_json::to_string_pretty(&plan)?)?;
    Ok(PlanningOutput {
        record_path,
        plan_path,
    })
}

pub fn check_plan_file(path: &Path) -> Result<CheckReport> {
    let raw = fs::read_to_string(path)
        .wrap_err_with(|| format!("failed to read plan {}", path.display()))?;
    let plan: Value = serde_json::from_str(&raw)
        .wrap_err_with(|| format!("invalid plan JSON {}", path.display()))?;
    let diagnostics = plan
        .get("diagnostics")
        .and_then(Value::as_array)
        .ok_or_else(|| eyre!("{} is missing diagnostics[]", path.display()))?;
    let errors = diagnostics
        .iter()
        .filter(|diag| diag.get("severity").and_then(Value::as_str) == Some("error"))
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|diag| diag.get("severity").and_then(Value::as_str) == Some("warning"))
        .count();
    if errors > 0 {
        let details = diagnostics
            .iter()
            .filter(|diag| diag.get("severity").and_then(Value::as_str) == Some("error"))
            .map(|diag| {
                let code = diag.get("code").and_then(Value::as_str).unwrap_or("error");
                let message = diag.get("message").and_then(Value::as_str).unwrap_or("");
                format!("{code}: {message}")
            })
            .collect::<Vec<_>>()
            .join("\n");
        return Err(eyre!(
            "nros check failed for {} with {} error(s):\n{}",
            path.display(),
            errors,
            details
        ));
    }
    Ok(CheckReport { errors, warnings })
}

fn parse_launch_args(args: &[String]) -> Result<HashMap<String, String>> {
    let mut out = HashMap::new();
    for arg in args {
        let Some((key, value)) = arg.split_once(":=").or_else(|| arg.split_once('=')) else {
            return Err(eyre!(
                "invalid launch argument `{arg}`: expected name:=value or name=value"
            ));
        };
        out.insert(key.to_string(), value.to_string());
    }
    Ok(out)
}

fn load_or_parse_record(
    launch_file: &Path,
    record_file: Option<&Path>,
    launch_args: HashMap<String, String>,
) -> Result<Value> {
    if let Some(record_file) = record_file {
        let raw = fs::read_to_string(record_file)
            .wrap_err_with(|| format!("failed to read record {}", record_file.display()))?;
        return serde_json::from_str(&raw)
            .wrap_err_with(|| format!("invalid record JSON {}", record_file.display()));
    }
    parse_launch_file_record(launch_file, launch_args)
}

#[cfg(feature = "play-launch-parser")]
fn parse_launch_file_record(
    launch_file: &Path,
    launch_args: HashMap<String, String>,
) -> Result<Value> {
    let record =
        play_launch_parser::parse_launch_file(launch_file, launch_args).map_err(|err| {
            eyre!(
                "failed to parse launch file {}: {err}",
                launch_file.display()
            )
        })?;
    Ok(serde_json::to_value(record)?)
}

#[cfg(not(feature = "play-launch-parser"))]
fn parse_launch_file_record(
    launch_file: &Path,
    _launch_args: HashMap<String, String>,
) -> Result<Value> {
    Err(eyre!(
        "play_launch_parser adapter is disabled for this build; pass --record <record.json> or build nros-cli-core with feature `play-launch-parser` (launch file: {})",
        launch_file.display()
    ))
}

fn record_array<'a>(record: &'a Value, key: &str) -> &'a [Value] {
    record
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn metadata_paths(
    options: &PlanOptions,
    workspace: &Workspace,
    metadata_dir: &Path,
) -> Vec<PathBuf> {
    let mut paths = options.metadata_files.clone();
    paths.extend(workspace.source_metadata_files());
    if metadata_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(metadata_dir) {
            paths.extend(
                entries
                    .flatten()
                    .map(|entry| entry.path())
                    .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json")),
            );
        }
    }
    unique_paths(paths)
}

fn load_json_artifacts(paths: &[PathBuf], label: &str) -> Result<Vec<JsonArtifact>> {
    paths
        .iter()
        .map(|path| {
            let raw = fs::read_to_string(path)
                .wrap_err_with(|| format!("failed to read {label} {}", path.display()))?;
            let value = serde_json::from_str(&raw)
                .wrap_err_with(|| format!("invalid {label} JSON {}", path.display()))?;
            Ok(JsonArtifact {
                path: path.clone(),
                value,
            })
        })
        .collect()
}

fn preserve_metadata(metadata: &[JsonArtifact], metadata_dir: &Path) -> Result<()> {
    for artifact in metadata {
        let Some(file_name) = artifact.path.file_name() else {
            continue;
        };
        let dest = metadata_dir.join(file_name);
        if dest != artifact.path {
            fs::write(dest, serde_json::to_string_pretty(&artifact.value)?)?;
        }
    }
    Ok(())
}

fn build_instances(
    record: &Value,
    metadata: &[JsonArtifact],
    workspace: &Workspace,
    overlays: &[Value],
    record_path: &Path,
) -> (Vec<Value>, Vec<Value>) {
    let mut counts = HashMap::<(String, String), usize>::new();
    let mut diagnostics = Vec::new();
    let mut instances = Vec::new();

    for node in record_array(record, "node") {
        let package = string_field(node, &["package"]).unwrap_or_default();
        if package.is_empty() {
            diagnostics.push(diagnostic(
                "error",
                "missing-package",
                "launch node has no package",
                None,
                None,
                None,
                record_path,
            ));
            continue;
        }
        let executable = string_field(node, &["executable"]).unwrap_or_default();
        let params = pairs_field(node, "params");
        let remaps = pairs_field(node, "remaps");
        let param_files = string_list_field(node, "params_files");
        instances.push(build_node_instance(
            package,
            executable,
            string_field(node, &["name"]),
            string_field(node, &["namespace"]),
            &params,
            &param_files,
            &remaps,
            "node",
            metadata,
            workspace,
            overlays,
            record_path,
            &mut counts,
            &mut diagnostics,
        ));
    }

    for load_node in record_array(record, "load_node") {
        let package = string_field(load_node, &["package"]).unwrap_or_default();
        let plugin = string_field(load_node, &["plugin"]).unwrap_or_default();
        let executable = plugin.split("::").last().unwrap_or(plugin);
        let params = pairs_field(load_node, "params");
        let remaps = pairs_field(load_node, "remaps");
        instances.push(build_node_instance(
            package,
            executable,
            string_field(load_node, &["node_name"]),
            string_field(load_node, &["namespace"]),
            &params,
            &[],
            &remaps,
            "load_node",
            metadata,
            workspace,
            overlays,
            record_path,
            &mut counts,
            &mut diagnostics,
        ));
    }

    (instances, diagnostics)
}

#[allow(clippy::too_many_arguments)]
fn build_node_instance(
    package: &str,
    executable: &str,
    name: Option<&str>,
    namespace: Option<&str>,
    params: &[(String, String)],
    param_files: &[String],
    remaps: &[(String, String)],
    launch_kind: &str,
    metadata: &[JsonArtifact],
    workspace: &Workspace,
    overlays: &[Value],
    record_path: &Path,
    counts: &mut HashMap<(String, String), usize>,
    diagnostics: &mut Vec<Value>,
) -> Value {
    let index = next_instance_index(counts, package, executable);
    let instance_id = format!(
        "{}.{}.{}",
        sanitize_id(package),
        sanitize_id(executable),
        index
    );
    let node_name = names::node_fqn(namespace, name, executable);
    let namespace = names::normalize_namespace(namespace);
    let source_metadata = find_source_metadata(metadata, package, executable);
    if source_metadata.is_none() {
        diagnostics.push(diagnostic(
            "error",
            "missing-source-metadata",
            format!("missing source metadata for {package}/{executable}"),
            Some(package),
            Some(&instance_id),
            None,
            record_path,
        ));
    }

    let package_nros = workspace
        .package_nros_toml(package)
        .and_then(|path| load_toml_values(&[path]).ok())
        .and_then(|mut values| values.pop());
    let parameters = effective_parameters(ParameterInputs {
        source_metadata: source_metadata.map(|artifact| &artifact.value),
        package_nros: package_nros.as_ref(),
        launch_params: params,
        param_files,
        overlays,
    });
    let entities = source_metadata
        .map(|artifact| {
            source_entities(
                &artifact.value,
                &artifact.path,
                &namespace,
                node_name.trim_start_matches('/'),
                remaps,
            )
        })
        .unwrap_or_default();

    json!({
        "id": instance_id,
        "telemetry_id": format!("{package}::{executable}#{index}"),
        "package": package,
        "executable": executable,
        "launch_kind": launch_kind,
        "node_name": node_name,
        "namespace": namespace,
        "remaps": remaps,
        "parameters": parameters,
        "source_metadata": source_metadata.map(|artifact| artifact.path.to_string_lossy().to_string()),
        "entities": entities,
    })
}

fn source_entities(
    metadata: &Value,
    path: &Path,
    namespace: &str,
    node_name: &str,
    remaps: &[(String, String)],
) -> Vec<Value> {
    let mut out = Vec::new();
    collect_entity_array(
        metadata.get("entities"),
        "entity",
        path,
        namespace,
        node_name,
        remaps,
        &mut out,
    );
    collect_entity_array(
        metadata.get("publishers"),
        "publisher",
        path,
        namespace,
        node_name,
        remaps,
        &mut out,
    );
    collect_entity_array(
        metadata.get("subscriptions"),
        "subscriber",
        path,
        namespace,
        node_name,
        remaps,
        &mut out,
    );
    collect_entity_array(
        metadata.get("subscribers"),
        "subscriber",
        path,
        namespace,
        node_name,
        remaps,
        &mut out,
    );
    collect_entity_array(
        metadata.get("services"),
        "service_server",
        path,
        namespace,
        node_name,
        remaps,
        &mut out,
    );
    collect_entity_array(
        metadata.get("clients"),
        "service_client",
        path,
        namespace,
        node_name,
        remaps,
        &mut out,
    );
    collect_entity_array(
        metadata.get("actions"),
        "action",
        path,
        namespace,
        node_name,
        remaps,
        &mut out,
    );
    out
}

#[allow(clippy::too_many_arguments)]
fn collect_entity_array(
    value: Option<&Value>,
    default_role: &str,
    path: &Path,
    namespace: &str,
    node_name: &str,
    remaps: &[(String, String)],
    out: &mut Vec<Value>,
) {
    let Some(Value::Array(items)) = value else {
        return;
    };
    for item in items {
        let role = item
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or(default_role);
        let source_name = item
            .get("name")
            .or_else(|| item.get("topic"))
            .or_else(|| item.get("service"))
            .or_else(|| item.get("action"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let resolved = names::resolve_entity_name(namespace, node_name, source_name, remaps);
        out.push(json!({
            "source_artifact": path,
            "source_id": item.get("id"),
            "role": normalize_role(role),
            "source_name": resolved.source,
            "resolved_name": resolved.resolved,
            "remapped_from": resolved.remapped_from,
            "type": item.get("type")
                .or_else(|| item.get("interface_type"))
                .or_else(|| item.get("message_type")),
        }));
    }
}

fn check_manifest_endpoints(
    instances: &[Value],
    manifests: &[ManifestArtifact],
    metadata: &[JsonArtifact],
    record_path: &Path,
) -> Vec<Value> {
    let mut diagnostics = Vec::new();
    if manifests.is_empty() {
        diagnostics.push(diagnostic(
            "warning",
            "missing-launch-manifest",
            "no ROS launch manifest files were loaded",
            None,
            None,
            None,
            record_path,
        ));
        return diagnostics;
    }
    let requirements = endpoint_requirements(manifests);
    for requirement in requirements {
        if !entity_matches_requirement(instances, &requirement) {
            diagnostics.push(diagnostic(
                "error",
                "manifest-endpoint-unmatched",
                format!(
                    "manifest endpoint did not match source metadata: role={} name={} type={}",
                    requirement
                        .get("role")
                        .and_then(Value::as_str)
                        .unwrap_or("?"),
                    requirement
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("?"),
                    requirement
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("?")
                ),
                None,
                None,
                Some(&artifact_list(metadata)),
                requirement
                    .get("source_artifact")
                    .and_then(Value::as_str)
                    .map(PathBuf::from)
                    .as_deref()
                    .unwrap_or(record_path),
            ));
        }
    }
    diagnostics
}

fn entity_matches_requirement(instances: &[Value], requirement: &Value) -> bool {
    let role = requirement
        .get("role")
        .and_then(Value::as_str)
        .map(normalize_role)
        .unwrap_or_default();
    let name = requirement
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("");
    let interface_type = requirement.get("type").and_then(Value::as_str);
    instances
        .iter()
        .filter(|instance| requirement_node_matches(instance, requirement))
        .any(|instance| {
            instance
                .get("entities")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .any(|entity| {
                    entity.get("role").and_then(Value::as_str) == Some(role.as_str())
                        && endpoint_name_matches(entity, name)
                        && interface_type.is_none_or(|ty| entity_type(entity) == Some(ty))
                })
        })
}

fn requirement_node_matches(instance: &Value, requirement: &Value) -> bool {
    let Some(required_node) = requirement.get("node").and_then(Value::as_str) else {
        return true;
    };
    let Some(instance_node) = instance.get("node_name").and_then(Value::as_str) else {
        return false;
    };
    instance_node == required_node
        || instance_node.trim_start_matches('/') == required_node.trim_start_matches('/')
}

fn endpoint_name_matches(entity: &Value, name: &str) -> bool {
    let Some(resolved) = entity.get("resolved_name").and_then(Value::as_str) else {
        return false;
    };
    resolved == name || resolved.trim_start_matches('/') == name.trim_start_matches('/')
}

fn entity_type(entity: &Value) -> Option<&str> {
    entity.get("type").and_then(|ty| {
        if let Value::String(s) = ty {
            Some(s.as_str())
        } else {
            ty.as_str()
        }
    })
}

fn find_source_metadata<'a>(
    metadata: &'a [JsonArtifact],
    package: &str,
    executable: &str,
) -> Option<&'a JsonArtifact> {
    metadata
        .iter()
        .find(|artifact| metadata_matches(&artifact.value, package, executable))
}

fn metadata_matches(value: &Value, package: &str, executable: &str) -> bool {
    let package_match = string_field(value, &["package", "package_name"])
        .is_none_or(|candidate| candidate == package);
    let executable_match = string_field(value, &["executable", "executable_name", "component"])
        .is_none_or(|candidate| candidate == executable);
    package_match && executable_match
}

fn string_field<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
}

fn pairs_field(value: &Value, key: &str) -> Vec<(String, String)> {
    match value.get(key) {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| match item {
                Value::Array(pair) if pair.len() == 2 => Some((
                    pair[0].as_str().unwrap_or_default().to_string(),
                    pair[1].as_str().unwrap_or_default().to_string(),
                )),
                Value::Object(map) => {
                    let key = map
                        .get("name")
                        .or_else(|| map.get("from"))
                        .or_else(|| map.get("key"))
                        .and_then(Value::as_str)?;
                    let value = map
                        .get("value")
                        .or_else(|| map.get("to"))
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    Some((key.to_string(), value.to_string()))
                }
                _ => None,
            })
            .collect(),
        Some(Value::Object(map)) => map
            .iter()
            .map(|(key, value)| (key.clone(), scalar_to_string(value)))
            .collect(),
        _ => Vec::new(),
    }
}

fn string_list_field(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn scalar_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Bool(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        other => other.to_string(),
    }
}

fn next_instance_index(
    counts: &mut HashMap<(String, String), usize>,
    package: &str,
    executable: &str,
) -> usize {
    let key = (package.to_string(), executable.to_string());
    let index = *counts.get(&key).unwrap_or(&0);
    counts.insert(key, index + 1);
    index
}

fn artifact_summaries(artifacts: &[JsonArtifact]) -> Vec<Value> {
    artifacts
        .iter()
        .map(|artifact| json!({"path": artifact.path, "package": string_field(&artifact.value, &["package", "package_name"]), "executable": string_field(&artifact.value, &["executable", "executable_name", "component"])}))
        .collect()
}

fn manifest_summaries(manifests: &[ManifestArtifact]) -> Vec<Value> {
    manifests
        .iter()
        .map(|artifact| json!({"path": artifact.path, "version": artifact.value.get("version")}))
        .collect()
}

fn artifact_list(artifacts: &[JsonArtifact]) -> String {
    artifacts
        .iter()
        .map(|artifact| artifact.path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn diagnostic(
    severity: &str,
    code: &str,
    message: impl Into<String>,
    package: Option<&str>,
    instance: Option<&str>,
    entity: Option<&str>,
    artifact: &Path,
) -> Value {
    let mut object = Map::new();
    object.insert("severity".to_string(), Value::String(severity.to_string()));
    object.insert("code".to_string(), Value::String(code.to_string()));
    object.insert("message".to_string(), Value::String(message.into()));
    object.insert(
        "source_artifact".to_string(),
        Value::String(artifact.display().to_string()),
    );
    if let Some(package) = package {
        object.insert("package".to_string(), Value::String(package.to_string()));
    }
    if let Some(instance) = instance {
        object.insert("instance".to_string(), Value::String(instance.to_string()));
    }
    if let Some(entity) = entity {
        object.insert("entity".to_string(), Value::String(entity.to_string()));
    }
    Value::Object(object)
}

fn normalize_role(role: &str) -> String {
    match role {
        "pub" | "publisher" => "publisher",
        "sub" | "subscriber" | "subscription" => "subscriber",
        "srv" | "server" | "service_server" => "service_server",
        "cli" | "client" | "service_client" => "service_client",
        "action_server" => "action_server",
        "action_client" => "action_client",
        other => other,
    }
    .to_string()
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_launch_args_in_ros_and_shell_forms() {
        let args = vec!["robot:=alpha".to_string(), "debug=true".to_string()];
        let parsed = parse_launch_args(&args).unwrap();
        assert_eq!(parsed["robot"], "alpha");
        assert_eq!(parsed["debug"], "true");
    }

    #[test]
    fn assigns_distinct_instance_indices() {
        let mut counts = HashMap::new();
        assert_eq!(next_instance_index(&mut counts, "pkg", "talker"), 0);
        assert_eq!(next_instance_index(&mut counts, "pkg", "talker"), 1);
    }

    #[cfg(feature = "play-launch-parser")]
    #[test]
    fn plan_system_parses_launch_and_keeps_distinct_instances() {
        let root = temp_workspace("nros-plan-two-instances");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("package.xml"),
            r#"<package format="3"><name>system_pkg</name><version>0.1.0</version></package>"#,
        )
        .unwrap();
        let launch = root.join("system.launch.xml");
        fs::write(
            &launch,
            r#"<launch>
  <node pkg="demo_pkg" exec="talker" name="talker_a" />
  <node pkg="demo_pkg" exec="talker" name="talker_b" />
</launch>"#,
        )
        .unwrap();
        let metadata = root.join("talker.metadata.json");
        fs::write(
            &metadata,
            r#"{
  "package": "demo_pkg",
  "executable": "talker",
  "publishers": [{"id": "pub.chatter", "name": "chatter", "type": "std_msgs/msg/String"}]
}"#,
        )
        .unwrap();

        let output = plan_system(PlanOptions {
            system_pkg: "system_pkg".to_string(),
            workspace_root: root.clone(),
            launch_file: launch,
            record_file: None,
            out_root: root.join("build/system_pkg/nros"),
            metadata_files: vec![metadata],
            manifest_files: vec![],
            nros_toml_files: vec![],
            launch_args: vec![],
        })
        .unwrap();
        let plan: Value =
            serde_json::from_str(&fs::read_to_string(output.plan_path).unwrap()).unwrap();
        let instances = plan["instances"].as_array().unwrap();
        assert_eq!(plan["record"]["node_count"], 2);
        assert_eq!(instances.len(), 2);
        assert_eq!(instances[0]["id"], "demo_pkg.talker.0");
        assert_eq!(instances[1]["id"], "demo_pkg.talker.1");
    }

    #[cfg(feature = "play-launch-parser")]
    #[test]
    fn plan_system_resolves_private_remap_and_matches_manifest() {
        let root = temp_workspace("nros-plan-private-remap");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("package.xml"),
            r#"<package format="3"><name>system_pkg</name><version>0.1.0</version></package>"#,
        )
        .unwrap();
        let launch = root.join("system.launch.xml");
        fs::write(
            &launch,
            r#"<launch>
  <node pkg="demo_pkg" exec="driver" name="driver" namespace="/robot">
    <remap from="~/cmd" to="/mux/cmd" />
  </node>
</launch>"#,
        )
        .unwrap();
        let metadata = root.join("driver.metadata.json");
        fs::write(
            &metadata,
            r#"{
  "package": "demo_pkg",
  "executable": "driver",
  "publishers": [{"id": "pub.cmd", "name": "~/cmd", "type": "std_msgs/msg/String"}]
}"#,
        )
        .unwrap();
        let manifest = root.join("manifest.launch.yaml");
        fs::write(
            &manifest,
            r#"version: 1
topics:
  /mux/cmd:
    type: std_msgs/msg/String
    pub: [/robot/driver]
"#,
        )
        .unwrap();

        let output = plan_system(PlanOptions {
            system_pkg: "system_pkg".to_string(),
            workspace_root: root.clone(),
            launch_file: launch,
            record_file: None,
            out_root: root.join("build/system_pkg/nros"),
            metadata_files: vec![metadata],
            manifest_files: vec![manifest],
            nros_toml_files: vec![],
            launch_args: vec![],
        })
        .unwrap();
        let plan: Value =
            serde_json::from_str(&fs::read_to_string(output.plan_path).unwrap()).unwrap();
        assert_eq!(
            plan["instances"][0]["entities"][0]["resolved_name"],
            "/mux/cmd"
        );
        assert!(
            plan["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .all(|diag| diag["severity"] != "error"),
            "unexpected diagnostics: {}",
            plan["diagnostics"]
        );
    }

    #[cfg(feature = "play-launch-parser")]
    fn temp_workspace(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{}-{stamp}", std::process::id()))
    }
}
