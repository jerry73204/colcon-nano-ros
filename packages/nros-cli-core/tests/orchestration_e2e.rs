use std::{
    fs,
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
    time::{SystemTime, UNIX_EPOCH},
};

use nros_cli_core::cmd::{build, check, metadata, plan};
use nros_cli_core::orchestration::{
    plan::{NrosPlan, PlanEntity},
    schema::ParameterValue,
};
use serde_json::Value;

#[test]
fn fixture_workspace_plans_checks_and_builds_generated_package() {
    let fixture = fixture_workspace();
    let output = temp_output("orchestration_e2e");
    let out_dir = output.join("build/e2e_system/nros");
    let generated_dir = out_dir.join("generated");
    let demo_pkg = fixture.join("src/demo_pkg");

    metadata::run(metadata::Args {
        system_pkg: "e2e_system".to_string(),
        workspace: Some(fixture.clone()),
        out_dir: Some(out_dir.clone()),
        metadata: vec![fixture.join("artifacts/talker.metadata.json")],
    })
    .expect("metadata command preserves fixture source metadata");

    plan::run(plan::Args {
        system_pkg: "e2e_system".to_string(),
        launch_file: demo_pkg.join("launch/system.launch.xml"),
        record: None,
        workspace: Some(fixture.clone()),
        out_dir: Some(out_dir.clone()),
        metadata: Vec::new(),
        manifests: vec![demo_pkg.join("manifest/system.launch.yaml")],
        nros_toml: Vec::new(),
        launch_args: Vec::new(),
    })
    .expect("plan command parses launch and writes checked artifacts");

    let plan_path = out_dir.join("nros-plan.json");
    check::run(check::Args {
        plan: plan_path.clone(),
    })
    .expect("check command validates generated plan");

    let plan: NrosPlan =
        serde_json::from_str(&fs::read_to_string(&plan_path).expect("read generated plan"))
            .expect("generated plan has canonical schema");
    assert_eq!(plan.system, "e2e_system");
    assert_eq!(plan.instances.len(), 1);
    assert_eq!(plan.instances[0].package, "demo_pkg");
    assert_eq!(plan.instances[0].parameters[0].name, "rate_hz");
    assert_eq!(
        plan.instances[0].parameters[0].value,
        ParameterValue::Integer(25)
    );

    let record: Value = serde_json::from_str(
        &fs::read_to_string(out_dir.join("record.json")).expect("read record"),
    )
    .expect("record is JSON");
    let nodes = record["node"].as_array().expect("record has node array");
    assert_eq!(nodes[0]["package"].as_str(), Some("demo_pkg"));
    assert_eq!(nodes[0]["executable"].as_str(), Some("talker"));

    build::run(build::Args {
        project: Some(fixture),
        system_plan: Some(plan_path),
        system_output: Some(generated_dir.clone()),
        system_package: Some("nros-e2e-generated".to_string()),
        nano_ros_workspace: Some(nano_ros_workspace()),
        release: false,
        target: None,
        passthrough: Vec::new(),
    })
    .expect("build command compiles generated package");

    assert!(generated_dir.join("Cargo.toml").is_file());
    assert!(generated_dir.join("src/main.rs").is_file());
    for lang in ["rust", "c", "cpp"] {
        let manifest_path = out_dir.join("interfaces").join(lang).join("manifest.json");
        let manifest: Value = serde_json::from_str(
            &fs::read_to_string(&manifest_path)
                .unwrap_or_else(|error| panic!("read {}: {error}", manifest_path.display())),
        )
        .unwrap_or_else(|error| panic!("parse {}: {error}", manifest_path.display()));
        assert_eq!(
            manifest["schema"].as_str(),
            Some("nano-ros/interface-cache/v1")
        );
        assert_eq!(manifest["system"].as_str(), Some("e2e_system"));
        assert_eq!(
            manifest["interfaces"][0]["id"].as_str(),
            Some("std_msgs/msg/String")
        );
    }

    let binary = out_dir
        .join("target")
        .join(&plan.build.target)
        .join("debug")
        .join("nros-e2e-generated");
    assert!(
        binary.is_file(),
        "generated binary exists at {}",
        binary.display()
    );

    let port = free_local_port();
    let _zenohd = start_zenohd(port);
    assert_generated_binary_spins(&binary, port);

    let multi_plan_path = out_dir.join("nros-plan-multi-instance.json");
    let mut multi_plan = plan.clone();
    add_second_instance(&mut multi_plan);
    fs::write(
        &multi_plan_path,
        serde_json::to_string_pretty(&multi_plan).expect("serialize multi-instance plan"),
    )
    .expect("write multi-instance plan");
    check::run(check::Args {
        plan: multi_plan_path.clone(),
    })
    .expect("check command validates generated multi-instance plan");
    let multi_generated_dir = out_dir.join("generated-multi");
    build::run(build::Args {
        project: Some(fixture_workspace()),
        system_plan: Some(multi_plan_path),
        system_output: Some(multi_generated_dir.clone()),
        system_package: Some("nros-e2e-generated-multi".to_string()),
        nano_ros_workspace: Some(nano_ros_workspace()),
        release: false,
        target: None,
        passthrough: Vec::new(),
    })
    .expect("build command compiles generated multi-instance package");
    assert!(
        multi_generated_dir
            .join("../target")
            .join(&multi_plan.build.target)
            .join("debug")
            .join("nros-e2e-generated-multi")
            .is_file()
    );
}

fn fixture_workspace() -> PathBuf {
    codegen_root().join("testing_workspaces/orchestration_e2e")
}

fn codegen_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("codegen root ancestor")
        .to_path_buf()
}

fn nano_ros_workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("nano-ros workspace ancestor")
        .to_path_buf()
}

fn add_second_instance(plan: &mut NrosPlan) {
    let mut instance = plan.instances[0].clone();
    let old_instance_id = instance.id.clone();
    let new_instance_id = "talker_clone";
    instance.id = new_instance_id.to_string();
    instance.launch_name = "talker_clone".to_string();
    instance.namespace = "/clone".to_string();
    for node in &mut instance.nodes {
        let old_node_id = node.id.clone();
        node.id = node.id.replacen(&old_instance_id, new_instance_id, 1);
        node.resolved_name = "/clone/talker".to_string();
        node.namespace = "/clone".to_string();
        for entity in &mut node.entities {
            rewrite_entity_id(entity, &old_instance_id, new_instance_id);
        }
        for parameter in &mut instance.parameters {
            if parameter.node == old_node_id {
                parameter.node = node.id.clone();
            }
        }
    }
    for callback in &mut instance.callbacks {
        callback.id = callback.id.replacen(&old_instance_id, new_instance_id, 1);
    }
    for binding in &mut instance.sched_bindings {
        binding.callback = binding
            .callback
            .replacen(&old_instance_id, new_instance_id, 1);
    }
    for interface in &mut plan.interfaces {
        let extra = interface
            .used_by
            .iter()
            .filter(|entity| entity.starts_with(&old_instance_id))
            .map(|entity| entity.replacen(&old_instance_id, new_instance_id, 1))
            .collect::<Vec<_>>();
        interface.used_by.extend(extra);
        interface.used_by.sort();
        interface.used_by.dedup();
    }
    plan.instances.push(instance);
}

fn rewrite_entity_id(entity: &mut PlanEntity, old_instance_id: &str, new_instance_id: &str) {
    match entity {
        PlanEntity::Publisher {
            id, resolved_name, ..
        }
        | PlanEntity::Subscriber {
            id, resolved_name, ..
        }
        | PlanEntity::ServiceServer {
            id, resolved_name, ..
        }
        | PlanEntity::ServiceClient {
            id, resolved_name, ..
        }
        | PlanEntity::ActionServer {
            id, resolved_name, ..
        }
        | PlanEntity::ActionClient {
            id, resolved_name, ..
        } => {
            *id = id.replacen(old_instance_id, new_instance_id, 1);
            *resolved_name = resolved_name.replacen("/talker", "/clone/talker", 1);
        }
        PlanEntity::Timer { id, .. } => {
            *id = id.replacen(old_instance_id, new_instance_id, 1);
        }
    }
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn free_local_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral localhost port");
    listener
        .local_addr()
        .expect("ephemeral listener local address")
        .port()
}

fn start_zenohd(port: u16) -> ChildGuard {
    let zenohd = nano_ros_workspace().join("build/zenohd/zenohd");
    assert!(
        zenohd.is_file(),
        "zenohd binary missing at {}; run `just build-zenohd`",
        zenohd.display()
    );

    let child = Command::new(&zenohd)
        .arg("--listen")
        .arg(format!("tcp/127.0.0.1:{port}"))
        .arg("--no-multicast-scouting")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn zenohd {}: {error}", zenohd.display()));
    let guard = ChildGuard(child);
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return guard;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("zenohd did not listen on tcp/127.0.0.1:{port}");
}

fn assert_generated_binary_spins(binary: &Path, port: u16) {
    let mut child = Command::new(binary)
        .env("NROS_LOCATOR", format!("tcp/127.0.0.1:{port}"))
        .env("NROS_SESSION_MODE", "client")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn generated binary {}: {error}", binary.display()));

    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if let Some(status) = child
            .try_wait()
            .expect("poll generated binary process status")
        {
            let output = child
                .wait_with_output()
                .expect("collect generated binary output");
            panic!(
                "generated binary exited early with {status}\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        thread::sleep(Duration::from_millis(50));
    }

    child.kill().expect("stop spinning generated binary");
    let status = child.wait().expect("wait for stopped generated binary");
    assert!(
        !status.success(),
        "generated binary should still be spinning until the test stops it"
    );
}

fn temp_output(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{name}-{}-{stamp}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}
