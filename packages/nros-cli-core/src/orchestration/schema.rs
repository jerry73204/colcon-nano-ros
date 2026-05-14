use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const SOURCE_METADATA_VERSION: u32 = 1;
pub const COMPONENT_CONFIG_VERSION: u32 = 1;
pub const SYSTEM_CONFIG_VERSION: u32 = 1;
pub const PLAN_VERSION: u32 = 1;

pub type ParameterTable = BTreeMap<String, ParameterValue>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterfaceRef {
    pub package: String,
    pub name: String,
    pub kind: InterfaceKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceKind {
    Message,
    Service,
    Action,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParameterValue {
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    BoolArray(Vec<bool>),
    IntegerArray(Vec<i64>),
    FloatArray(Vec<f64>),
    StringArray(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QosProfile {
    pub reliability: QosReliability,
    pub durability: QosDurability,
    pub history: QosHistory,
    pub depth: u32,
    pub deadline_ms: Option<u64>,
    pub lifespan_ms: Option<u64>,
    pub liveliness: QosLiveliness,
    pub liveliness_lease_duration_ms: Option<u64>,
    pub extensions: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QosReliability {
    SystemDefault,
    Reliable,
    BestEffort,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QosDurability {
    SystemDefault,
    Volatile,
    TransientLocal,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QosHistory {
    SystemDefault,
    KeepLast,
    KeepAll,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QosLiveliness {
    SystemDefault,
    Automatic,
    ManualByTopic,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceLocation {
    pub artifact: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemapRule {
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceName {
    pub value: String,
    pub kind: SourceNameKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceNameKind {
    Absolute,
    Relative,
    Private,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedClass {
    BestEffort,
    RealTime,
    TimeTriggered,
    Interrupt,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeadlinePolicy {
    Ignore,
    Warn,
    Skip,
    Fault,
}
