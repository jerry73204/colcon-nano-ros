use std::fmt::Debug;

use nros_cli_core::orchestration::{
    ComponentConfig, NrosPlan, SourceMetadata, SystemConfig,
    plan::PlanEntity,
    schema::{
        COMPONENT_CONFIG_VERSION, PLAN_VERSION, SOURCE_METADATA_VERSION, SYSTEM_CONFIG_VERSION,
    },
};
use serde::{Serialize, de::DeserializeOwned};

fn assert_json_fixture<T>(raw: &str) -> T
where
    T: DeserializeOwned + Serialize + PartialEq + Debug,
{
    let parsed: T = serde_json::from_str(raw).expect("fixture parses");
    let pretty = format!(
        "{}\n",
        serde_json::to_string_pretty(&parsed).expect("fixture formats")
    );
    assert_eq!(pretty, raw, "fixture must match stable pretty JSON");

    let reparsed: T = serde_json::from_str(&pretty).expect("pretty JSON parses");
    assert_eq!(reparsed, parsed);
    parsed
}

fn assert_toml_fixture<T>(raw: &str) -> T
where
    T: DeserializeOwned + Serialize + PartialEq + Debug,
{
    let parsed: T = toml::from_str(raw).expect("fixture parses");
    let formatted = toml::to_string_pretty(&parsed).expect("fixture formats");
    let reparsed: T = toml::from_str(&formatted).expect("pretty TOML parses");
    assert_eq!(reparsed, parsed);
    parsed
}

#[test]
fn source_metadata_json_round_trips() {
    let metadata: SourceMetadata = assert_json_fixture(include_str!(
        "fixtures/orchestration/source_metadata_talker.json"
    ));

    assert_eq!(metadata.version, SOURCE_METADATA_VERSION);
    assert_eq!(metadata.package, "demo_nodes_rs");
    assert_eq!(
        metadata.nodes[0].publishers[0].unresolved_topic.value,
        "chatter"
    );
    assert_eq!(metadata.nodes[0].timers[0].callback, "cb_timer");
}

#[test]
fn source_metadata_name_and_effect_edges_round_trip() {
    let metadata: SourceMetadata = assert_json_fixture(include_str!(
        "fixtures/orchestration/source_metadata_names_effects.json"
    ));

    let node = &metadata.nodes[0];
    assert_eq!(node.unresolved_name.value, "/robot/controller");
    assert_eq!(node.publishers[0].unresolved_topic.value, "/diagnostics");
    assert_eq!(node.subscribers[0].unresolved_topic.value, "~/cmd");
    assert_eq!(metadata.callbacks[0].effects.len(), 2);
    assert_eq!(metadata.callbacks[2].effects.len(), 2);
}

#[test]
fn component_nros_toml_round_trips() {
    let config: ComponentConfig =
        assert_toml_fixture(include_str!("fixtures/orchestration/component_nros.toml"));

    assert_eq!(config.version, COMPONENT_CONFIG_VERSION);
    assert_eq!(config.package, "demo_nodes_rs");
    assert_eq!(
        config.metadata.source_metadata,
        "target/nros/metadata/talker.json"
    );
}

#[test]
fn system_nros_toml_round_trips() {
    let config: SystemConfig =
        assert_toml_fixture(include_str!("fixtures/orchestration/system_nros.toml"));

    assert_eq!(config.version, SYSTEM_CONFIG_VERSION);
    assert_eq!(config.system, "demo_system");
    assert_eq!(config.scheduling.contexts[0].id, "default_executor");
}

#[test]
fn pub_sub_plan_json_round_trips() {
    let plan: NrosPlan =
        assert_json_fixture(include_str!("fixtures/orchestration/plan_pub_sub.json"));

    assert_eq!(plan.version, PLAN_VERSION);
    assert_eq!(plan.instances.len(), 2);
    let first_entity = &plan.instances[0].nodes[0].entities[0];
    let PlanEntity::Publisher { resolved_name, .. } = first_entity else {
        panic!("first entity should be publisher: {first_entity:?}");
    };
    assert_eq!(resolved_name, "/chatter");
    assert!(
        matches!(
            plan.instances[0].nodes[0].entities[1],
            PlanEntity::Timer { .. }
        ),
        "timer entity must not require graph name or interface"
    );
}

#[test]
fn multi_instance_plan_json_round_trips() {
    let plan: NrosPlan = assert_json_fixture(include_str!(
        "fixtures/orchestration/plan_multi_instance.json"
    ));

    assert_eq!(plan.version, PLAN_VERSION);
    assert_eq!(plan.instances.len(), 2);
    assert_ne!(plan.instances[0].id, plan.instances[1].id);
    assert_eq!(plan.instances[0].component, plan.instances[1].component);
}

#[test]
fn edge_instance_name_and_sched_variants_round_trip() {
    let plan: NrosPlan = assert_json_fixture(include_str!(
        "fixtures/orchestration/plan_edge_instances_names.json"
    ));

    assert_eq!(plan.version, PLAN_VERSION);
    assert_eq!(plan.instances.len(), 2);
    assert_eq!(plan.instances[0].component, plan.instances[1].component);
    assert_eq!(plan.instances[0].remaps[0].from, "~/cmd");
    assert_eq!(plan.instances[0].remaps[1].from, "/diagnostics");
    assert_eq!(plan.sched_contexts.len(), 4);
}

#[test]
fn service_action_plan_json_round_trips() {
    let plan: NrosPlan = assert_json_fixture(include_str!(
        "fixtures/orchestration/plan_service_action.json"
    ));

    assert_eq!(plan.version, PLAN_VERSION);
    assert!(matches!(
        plan.instances[0].nodes[0].entities[0],
        PlanEntity::ServiceServer { .. }
    ));
    assert!(matches!(
        plan.instances[0].nodes[0].entities[1],
        PlanEntity::ActionServer { .. }
    ));
}

#[test]
fn unknown_fields_are_rejected() {
    let mut raw = include_str!("fixtures/orchestration/source_metadata_talker.json").to_owned();
    raw = raw.replacen(
        "\"version\": 1,",
        "\"version\": 1,\n  \"unexpected\": true,",
        1,
    );

    let error = serde_json::from_str::<SourceMetadata>(&raw).expect_err("unknown field rejected");
    assert!(
        error.to_string().contains("unknown field"),
        "error should mention unknown field: {error}"
    );
}

#[test]
fn unknown_toml_fields_are_rejected() {
    let raw = include_str!("fixtures/orchestration/component_nros.toml").replacen(
        "component = \"talker\"",
        "component = \"talker\"\nunexpected = true",
        1,
    );

    let error = toml::from_str::<ComponentConfig>(&raw).expect_err("unknown field rejected");
    assert!(
        error.to_string().contains("unknown field"),
        "error should mention unknown field: {error}"
    );
}

#[test]
fn version_fields_are_required() {
    let raw = include_str!("fixtures/orchestration/plan_pub_sub.json").replacen(
        "  \"version\": 1,\n",
        "",
        1,
    );

    let error = serde_json::from_str::<NrosPlan>(&raw).expect_err("missing version rejected");
    assert!(
        error.to_string().contains("missing field `version`"),
        "error should mention missing version: {error}"
    );
}
