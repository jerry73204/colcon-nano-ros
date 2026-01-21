use askama::Template;

// Custom Askama filters
pub mod filters {
    use crate::utils::to_snake_case;

    pub fn snake_case(s: &str) -> ::askama::Result<String> {
        Ok(to_snake_case(s))
    }
}

#[derive(Template)]
#[template(path = "cargo.toml.jinja", escape = "none")]
pub struct CargoTomlTemplate<'a> {
    pub package_name: &'a str,
    pub dependencies: &'a [String],
    pub needs_big_array: bool,
}

#[derive(Template)]
#[template(path = "build.rs.jinja", escape = "none")]
pub struct BuildRsTemplate;

#[derive(Template)]
#[template(path = "lib.rs.jinja", escape = "none")]
pub struct LibRsTemplate {
    pub has_messages: bool,
    pub has_services: bool,
    pub has_actions: bool,
}

#[derive(Template)]
#[template(path = "message_rmw.rs.jinja", escape = "none")]
pub struct MessageRmwTemplate<'a> {
    pub package_name: &'a str,
    pub message_name: &'a str,
    pub message_module: &'a str,
    pub fields: Vec<RmwField>,
    pub constants: Vec<MessageConstant>,
}

#[derive(Template)]
#[template(path = "message_idiomatic.rs.jinja", escape = "none")]
pub struct MessageIdiomaticTemplate<'a> {
    pub package_name: &'a str,
    pub message_name: &'a str,
    pub message_module: &'a str,
    pub fields: Vec<IdiomaticField>,
    pub constants: Vec<MessageConstant>,
}

pub struct RmwField {
    pub name: String,
    pub rust_type: String,
    pub default_value: String,
}

/// Exhaustive enum representing all possible ROS 2 IDL field types
/// This ensures compile-time checking that all cases are handled in templates
#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    // Scalar types (single values)
    Primitive,
    UnboundedString,
    BoundedString,
    UnboundedWString,
    BoundedWString,
    NestedMessage,

    // Array types (fixed-size)
    PrimitiveArray,
    UnboundedStringArray,
    BoundedStringArray,
    UnboundedWStringArray,
    BoundedWStringArray,
    NestedMessageArray,
    LargeArray, // Arrays > 32 elements (no Copy/Clone trait)

    // Bounded sequences (max_size specified: T[<=N])
    BoundedPrimitiveSequence,
    BoundedUnboundedStringSequence,  // string[<=N]
    BoundedBoundedStringSequence,    // string<=M[<=N]
    BoundedUnboundedWStringSequence, // wstring[<=N]
    BoundedBoundedWStringSequence,   // wstring<=M[<=N]
    BoundedNestedMessageSequence,

    // Unbounded sequences (no max_size: T[])
    UnboundedPrimitiveSequence,
    UnboundedUnboundedStringSequence,  // string[]
    UnboundedBoundedStringSequence,    // string<=M[]
    UnboundedUnboundedWStringSequence, // wstring[]
    UnboundedBoundedWStringSequence,   // wstring<=M[]
    UnboundedNestedMessageSequence,
}

pub struct IdiomaticField {
    pub name: String,
    pub rust_type: String,
    pub default_value: String,
    pub kind: FieldKind,
}

pub struct MessageConstant {
    pub name: String,
    pub rust_type: String,
    pub value: String,
}

#[derive(Template)]
#[template(path = "service_rmw.rs.jinja", escape = "none")]
pub struct ServiceRmwTemplate<'a> {
    pub package_name: &'a str,
    pub service_name: &'a str,
    pub request_fields: Vec<RmwField>,
    pub request_constants: Vec<MessageConstant>,
    pub response_fields: Vec<RmwField>,
    pub response_constants: Vec<MessageConstant>,
}

#[derive(Template)]
#[template(path = "service_idiomatic.rs.jinja", escape = "none")]
pub struct ServiceIdiomaticTemplate<'a> {
    pub package_name: &'a str,
    pub service_name: &'a str,
    pub request_fields: Vec<IdiomaticField>,
    pub request_constants: Vec<MessageConstant>,
    pub response_fields: Vec<IdiomaticField>,
    pub response_constants: Vec<MessageConstant>,
}

#[derive(Template)]
#[template(path = "action_rmw.rs.jinja", escape = "none")]
pub struct ActionRmwTemplate<'a> {
    pub package_name: &'a str,
    pub action_name: &'a str,
    pub goal_fields: Vec<RmwField>,
    pub goal_constants: Vec<MessageConstant>,
    pub result_fields: Vec<RmwField>,
    pub result_constants: Vec<MessageConstant>,
    pub feedback_fields: Vec<RmwField>,
    pub feedback_constants: Vec<MessageConstant>,
}

#[derive(Template)]
#[template(path = "action_idiomatic.rs.jinja", escape = "none")]
pub struct ActionIdiomaticTemplate<'a> {
    pub package_name: &'a str,
    pub action_name: &'a str,
    pub goal_fields: Vec<IdiomaticField>,
    pub goal_constants: Vec<MessageConstant>,
    pub result_fields: Vec<IdiomaticField>,
    pub result_constants: Vec<MessageConstant>,
    pub feedback_fields: Vec<IdiomaticField>,
    pub feedback_constants: Vec<MessageConstant>,
}

// ============================================================================
// nano-ros Templates
// ============================================================================

/// Field metadata for nano-ros code generation
#[derive(Debug, Clone)]
pub struct NanoRosField {
    pub name: String,
    pub rust_type: String,
    /// CDR primitive method name (e.g., "i32", "f64", "u8") - empty if not primitive
    pub primitive_method: String,
    /// For arrays/sequences: element primitive method - empty if not primitive element
    pub element_primitive_method: String,
    /// Array size for fixed arrays - 0 if not an array
    pub array_size: usize,

    // Type flags for template conditionals
    pub is_primitive: bool,
    pub is_string: bool,
    pub is_array: bool,
    pub is_sequence: bool,
    pub is_nested: bool,
    pub is_primitive_element: bool,
    pub is_string_element: bool,
}

#[derive(Template)]
#[template(path = "message_nano_ros.rs.jinja", escape = "none")]
pub struct MessageNanoRosTemplate<'a> {
    pub package_name: &'a str,
    pub message_name: &'a str,
    pub type_hash: &'a str,
    pub fields: Vec<NanoRosField>,
    pub constants: Vec<MessageConstant>,
    /// True if there are fields to serialize/deserialize
    pub has_fields: bool,
}

#[derive(Template)]
#[template(path = "service_nano_ros.rs.jinja", escape = "none")]
pub struct ServiceNanoRosTemplate<'a> {
    pub package_name: &'a str,
    pub service_name: &'a str,
    pub type_hash: &'a str,
    pub request_fields: Vec<NanoRosField>,
    pub request_constants: Vec<MessageConstant>,
    pub response_fields: Vec<NanoRosField>,
    pub response_constants: Vec<MessageConstant>,
    /// True if request has fields to serialize/deserialize
    pub has_request_fields: bool,
    /// True if response has fields to serialize/deserialize
    pub has_response_fields: bool,
}

#[derive(Template)]
#[template(path = "cargo_nano_ros.toml.jinja", escape = "none")]
pub struct CargoNanoRosTomlTemplate<'a> {
    pub package_name: &'a str,
    pub package_version: &'a str,
    pub dependencies: &'a [String],
}

#[derive(Template)]
#[template(path = "lib_nano_ros.rs.jinja", escape = "none")]
pub struct LibNanoRosRsTemplate {
    pub has_messages: bool,
    pub has_services: bool,
    pub has_actions: bool,
}

#[derive(Template)]
#[template(path = "action_nano_ros.rs.jinja", escape = "none")]
pub struct ActionNanoRosTemplate<'a> {
    pub package_name: &'a str,
    pub action_name: &'a str,
    pub type_hash: &'a str,
    pub goal_fields: Vec<NanoRosField>,
    pub goal_constants: Vec<MessageConstant>,
    pub result_fields: Vec<NanoRosField>,
    pub result_constants: Vec<MessageConstant>,
    pub feedback_fields: Vec<NanoRosField>,
    pub feedback_constants: Vec<MessageConstant>,
    /// True if goal has fields to serialize/deserialize
    pub has_goal_fields: bool,
    /// True if result has fields to serialize/deserialize
    pub has_result_fields: bool,
    /// True if feedback has fields to serialize/deserialize
    pub has_feedback_fields: bool,
}
