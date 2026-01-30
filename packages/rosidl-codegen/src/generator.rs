use crate::templates::{
    ActionCHeaderTemplate, ActionCSourceTemplate, ActionIdiomaticTemplate, ActionNanoRosTemplate,
    ActionRmwTemplate, BuildRsTemplate, CConstant, CField, CargoNanoRosTomlTemplate,
    CargoTomlTemplate, FieldKind, IdiomaticField, LibNanoRosRsTemplate, LibRsTemplate,
    MessageCHeaderTemplate, MessageCSourceTemplate, MessageConstant, MessageIdiomaticTemplate,
    MessageNanoRosTemplate, MessageRmwTemplate, NanoRosField, RmwField, ServiceCHeaderTemplate,
    ServiceCSourceTemplate, ServiceIdiomaticTemplate, ServiceNanoRosTemplate, ServiceRmwTemplate,
};
use crate::types::{
    c_array_suffix_for_field, c_cdr_read_method, c_cdr_write_method, c_type_for_constant,
    c_type_for_field, constant_value_to_rust, escape_keyword, nano_ros_type_for_constant,
    nano_ros_type_for_field, rust_type_for_constant, rust_type_for_field, to_c_package_name,
    C_DEFAULT_SEQUENCE_CAPACITY,
};
use crate::utils::{extract_dependencies, needs_big_array, to_snake_case};
use askama::Template;
use rosidl_parser::{Action, FieldType, Message, Service};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GeneratorError {
    #[error("Template rendering failed: {0}")]
    TemplateError(#[from] askama::Error),

    #[error("Invalid message structure: {0}")]
    InvalidMessage(String),
}

pub struct GeneratedPackage {
    pub cargo_toml: String,
    pub build_rs: String,
    pub lib_rs: String,
    pub message_rmw: String,
    pub message_idiomatic: String,
}

/// Determine the exhaustive FieldKind enum variant for a given ROS 2 field type
/// This function provides compile-time guarantees that all field type combinations are handled
fn determine_field_kind(field_type: &FieldType) -> FieldKind {
    match field_type {
        // Scalar types
        FieldType::Primitive(_) => FieldKind::Primitive,

        FieldType::String => FieldKind::UnboundedString,
        FieldType::BoundedString(_) => FieldKind::BoundedString,

        FieldType::WString => FieldKind::UnboundedWString,
        FieldType::BoundedWString(_) => FieldKind::BoundedWString,

        FieldType::NamespacedType { .. } => FieldKind::NestedMessage,

        // Array types
        FieldType::Array { element_type, size } => {
            // Arrays > 32 elements don't impl Copy/Clone in Rust
            if *size > 32 {
                return FieldKind::LargeArray;
            }

            match element_type.as_ref() {
                FieldType::Primitive(_) => FieldKind::PrimitiveArray,

                FieldType::String => FieldKind::UnboundedStringArray,
                FieldType::BoundedString(_) => FieldKind::BoundedStringArray,

                FieldType::WString => FieldKind::UnboundedWStringArray,
                FieldType::BoundedWString(_) => FieldKind::BoundedWStringArray,

                _ => FieldKind::NestedMessageArray,
            }
        }

        // Bounded sequences (T[<=N])
        FieldType::BoundedSequence { element_type, .. } => match element_type.as_ref() {
            FieldType::Primitive(_) => FieldKind::BoundedPrimitiveSequence,

            FieldType::String => FieldKind::BoundedUnboundedStringSequence,
            FieldType::BoundedString(_) => FieldKind::BoundedBoundedStringSequence,

            FieldType::WString => FieldKind::BoundedUnboundedWStringSequence,
            FieldType::BoundedWString(_) => FieldKind::BoundedBoundedWStringSequence,

            _ => FieldKind::BoundedNestedMessageSequence,
        },

        // Unbounded sequences (T[])
        FieldType::Sequence { element_type } => match element_type.as_ref() {
            FieldType::Primitive(_) => FieldKind::UnboundedPrimitiveSequence,

            FieldType::String => FieldKind::UnboundedUnboundedStringSequence,
            FieldType::BoundedString(_) => FieldKind::UnboundedBoundedStringSequence,

            FieldType::WString => FieldKind::UnboundedUnboundedWStringSequence,
            FieldType::BoundedWString(_) => FieldKind::UnboundedBoundedWStringSequence,

            _ => FieldKind::UnboundedNestedMessageSequence,
        },
    }
}

/// Generate a complete ROS 2 message package with both RMW and idiomatic layers
pub fn generate_message_package(
    package_name: &str,
    message_name: &str,
    message: &Message,
    all_dependencies: &HashSet<String>,
) -> Result<GeneratedPackage, GeneratorError> {
    // Extract dependencies from this specific message
    let msg_deps = extract_dependencies(message);

    // Combine with externally provided dependencies
    let mut all_deps: Vec<String> = all_dependencies.iter().cloned().collect();
    all_deps.extend(msg_deps);
    all_deps.sort();
    all_deps.dedup();

    // Check if we need serde's big-array feature
    let needs_big_array_feature = needs_big_array(message);

    // Generate Cargo.toml
    let cargo_toml_template = CargoTomlTemplate {
        package_name,
        dependencies: &all_deps,
        needs_big_array: needs_big_array_feature,
    };
    let cargo_toml = cargo_toml_template.render()?;

    // Generate build.rs
    let build_rs_template = BuildRsTemplate;
    let build_rs = build_rs_template.render()?;

    // Generate lib.rs
    let lib_rs_template = LibRsTemplate {
        has_messages: true,
        has_services: false,
        has_actions: false,
    };
    let lib_rs = lib_rs_template.render()?;

    // Generate RMW layer message
    let rmw_fields: Vec<RmwField> = message
        .fields
        .iter()
        .map(|f| RmwField {
            name: escape_keyword(&f.name),
            rust_type: rust_type_for_field(&f.field_type, true, Some(package_name)),
            default_value: f
                .default_value
                .as_ref()
                .map(constant_value_to_rust)
                .unwrap_or_default(),
        })
        .collect();

    let rmw_constants: Vec<MessageConstant> = message
        .constants
        .iter()
        .map(|c| MessageConstant {
            name: c.name.clone(),
            rust_type: rust_type_for_constant(&c.constant_type),
            value: constant_value_to_rust(&c.value),
        })
        .collect();

    let message_module = &to_snake_case(message_name);

    let message_rmw_template = MessageRmwTemplate {
        package_name,
        message_name,
        message_module,
        fields: rmw_fields,
        constants: rmw_constants,
    };
    let message_rmw = message_rmw_template.render()?;

    // Generate idiomatic layer message
    let idiomatic_fields: Vec<IdiomaticField> = message
        .fields
        .iter()
        .map(|f| IdiomaticField {
            name: escape_keyword(&f.name),
            rust_type: rust_type_for_field(&f.field_type, false, Some(package_name)),
            default_value: f
                .default_value
                .as_ref()
                .map(constant_value_to_rust)
                .unwrap_or_default(),
            kind: determine_field_kind(&f.field_type),
        })
        .collect();

    let idiomatic_constants: Vec<MessageConstant> = message
        .constants
        .iter()
        .map(|c| MessageConstant {
            name: c.name.clone(),
            rust_type: rust_type_for_constant(&c.constant_type),
            value: constant_value_to_rust(&c.value),
        })
        .collect();

    let message_idiomatic_template = MessageIdiomaticTemplate {
        package_name,
        message_name,
        message_module,
        fields: idiomatic_fields,
        constants: idiomatic_constants,
    };
    let message_idiomatic = message_idiomatic_template.render()?;

    Ok(GeneratedPackage {
        cargo_toml,
        build_rs,
        lib_rs,
        message_rmw,
        message_idiomatic,
    })
}

pub struct GeneratedServicePackage {
    pub cargo_toml: String,
    pub build_rs: String,
    pub lib_rs: String,
    pub service_rmw: String,
    pub service_idiomatic: String,
}

/// Generate a complete ROS 2 service package with both RMW and idiomatic layers
pub fn generate_service_package(
    package_name: &str,
    service_name: &str,
    service: &Service,
    all_dependencies: &HashSet<String>,
) -> Result<GeneratedServicePackage, GeneratorError> {
    // Extract dependencies from request and response
    let mut req_deps = extract_dependencies(&service.request);
    let resp_deps = extract_dependencies(&service.response);
    req_deps.extend(resp_deps);

    // Combine with externally provided dependencies
    let mut all_deps: Vec<String> = all_dependencies.iter().cloned().collect();
    all_deps.extend(req_deps);
    all_deps.sort();
    all_deps.dedup();

    // Check if we need serde's big-array feature
    let needs_big_array_feature =
        needs_big_array(&service.request) || needs_big_array(&service.response);

    // Generate Cargo.toml
    let cargo_toml_template = CargoTomlTemplate {
        package_name,
        dependencies: &all_deps,
        needs_big_array: needs_big_array_feature,
    };
    let cargo_toml = cargo_toml_template.render()?;

    // Generate build.rs
    let build_rs_template = BuildRsTemplate;
    let build_rs = build_rs_template.render()?;

    // Generate lib.rs
    let lib_rs_template = LibRsTemplate {
        has_messages: false,
        has_services: true,
        has_actions: false,
    };
    let lib_rs = lib_rs_template.render()?;

    // Helper functions to convert Message to field vectors
    let message_to_rmw_fields = |msg: &Message| {
        msg.fields
            .iter()
            .map(|f| RmwField {
                name: escape_keyword(&f.name),
                rust_type: rust_type_for_field(&f.field_type, true, Some(package_name)),
                default_value: f
                    .default_value
                    .as_ref()
                    .map(constant_value_to_rust)
                    .unwrap_or_default(),
            })
            .collect()
    };

    let message_to_idiomatic_fields = |msg: &Message| {
        msg.fields
            .iter()
            .map(|f| IdiomaticField {
                name: escape_keyword(&f.name),
                rust_type: rust_type_for_field(&f.field_type, false, Some(package_name)),
                default_value: f
                    .default_value
                    .as_ref()
                    .map(constant_value_to_rust)
                    .unwrap_or_default(),
                kind: determine_field_kind(&f.field_type),
            })
            .collect()
    };

    let message_to_constants = |msg: &Message, _rmw_layer: bool| {
        msg.constants
            .iter()
            .map(|c| MessageConstant {
                name: c.name.clone(),
                rust_type: rust_type_for_constant(&c.constant_type),
                value: constant_value_to_rust(&c.value),
            })
            .collect()
    };

    // Generate RMW layer service
    let service_rmw_template = ServiceRmwTemplate {
        package_name,
        service_name,
        request_fields: message_to_rmw_fields(&service.request),
        request_constants: message_to_constants(&service.request, true),
        response_fields: message_to_rmw_fields(&service.response),
        response_constants: message_to_constants(&service.response, true),
    };
    let service_rmw = service_rmw_template.render()?;

    // Generate idiomatic layer service
    let service_idiomatic_template = ServiceIdiomaticTemplate {
        package_name,
        service_name,
        request_fields: message_to_idiomatic_fields(&service.request),
        request_constants: message_to_constants(&service.request, false),
        response_fields: message_to_idiomatic_fields(&service.response),
        response_constants: message_to_constants(&service.response, false),
    };
    let service_idiomatic = service_idiomatic_template.render()?;

    Ok(GeneratedServicePackage {
        cargo_toml,
        build_rs,
        lib_rs,
        service_rmw,
        service_idiomatic,
    })
}

pub struct GeneratedActionPackage {
    pub cargo_toml: String,
    pub build_rs: String,
    pub lib_rs: String,
    pub action_rmw: String,
    pub action_idiomatic: String,
}

/// Generate a complete ROS 2 action package with both RMW and idiomatic layers
pub fn generate_action_package(
    package_name: &str,
    action_name: &str,
    action: &Action,
    all_dependencies: &HashSet<String>,
) -> Result<GeneratedActionPackage, GeneratorError> {
    // Extract dependencies from goal, result, and feedback
    let mut goal_deps = extract_dependencies(&action.spec.goal);
    let result_deps = extract_dependencies(&action.spec.result);
    let feedback_deps = extract_dependencies(&action.spec.feedback);
    goal_deps.extend(result_deps);
    goal_deps.extend(feedback_deps);

    // Combine with externally provided dependencies
    let mut all_deps: Vec<String> = all_dependencies.iter().cloned().collect();
    all_deps.extend(goal_deps);
    all_deps.sort();
    all_deps.dedup();

    // Check if we need serde's big-array feature
    let needs_big_array_feature = needs_big_array(&action.spec.goal)
        || needs_big_array(&action.spec.result)
        || needs_big_array(&action.spec.feedback);

    // Generate Cargo.toml
    let cargo_toml_template = CargoTomlTemplate {
        package_name,
        dependencies: &all_deps,
        needs_big_array: needs_big_array_feature,
    };
    let cargo_toml = cargo_toml_template.render()?;

    // Generate build.rs
    let build_rs_template = BuildRsTemplate;
    let build_rs = build_rs_template.render()?;

    // Generate lib.rs
    let lib_rs_template = LibRsTemplate {
        has_messages: false,
        has_services: false,
        has_actions: true,
    };
    let lib_rs = lib_rs_template.render()?;

    // Helper functions to convert Message to field vectors
    let message_to_rmw_fields = |msg: &Message| {
        msg.fields
            .iter()
            .map(|f| RmwField {
                name: escape_keyword(&f.name),
                rust_type: rust_type_for_field(&f.field_type, true, Some(package_name)),
                default_value: f
                    .default_value
                    .as_ref()
                    .map(constant_value_to_rust)
                    .unwrap_or_default(),
            })
            .collect()
    };

    let message_to_idiomatic_fields = |msg: &Message| {
        msg.fields
            .iter()
            .map(|f| IdiomaticField {
                name: escape_keyword(&f.name),
                rust_type: rust_type_for_field(&f.field_type, false, Some(package_name)),
                default_value: f
                    .default_value
                    .as_ref()
                    .map(constant_value_to_rust)
                    .unwrap_or_default(),
                kind: determine_field_kind(&f.field_type),
            })
            .collect()
    };

    let message_to_constants = |msg: &Message, _rmw_layer: bool| {
        msg.constants
            .iter()
            .map(|c| MessageConstant {
                name: c.name.clone(),
                rust_type: rust_type_for_constant(&c.constant_type),
                value: constant_value_to_rust(&c.value),
            })
            .collect()
    };

    // Generate RMW layer action
    let action_rmw_template = ActionRmwTemplate {
        package_name,
        action_name,
        goal_fields: message_to_rmw_fields(&action.spec.goal),
        goal_constants: message_to_constants(&action.spec.goal, true),
        result_fields: message_to_rmw_fields(&action.spec.result),
        result_constants: message_to_constants(&action.spec.result, true),
        feedback_fields: message_to_rmw_fields(&action.spec.feedback),
        feedback_constants: message_to_constants(&action.spec.feedback, true),
    };
    let action_rmw = action_rmw_template.render()?;

    // Generate idiomatic layer action
    let action_idiomatic_template = ActionIdiomaticTemplate {
        package_name,
        action_name,
        goal_fields: message_to_idiomatic_fields(&action.spec.goal),
        goal_constants: message_to_constants(&action.spec.goal, false),
        result_fields: message_to_idiomatic_fields(&action.spec.result),
        result_constants: message_to_constants(&action.spec.result, false),
        feedback_fields: message_to_idiomatic_fields(&action.spec.feedback),
        feedback_constants: message_to_constants(&action.spec.feedback, false),
    };
    let action_idiomatic = action_idiomatic_template.render()?;

    Ok(GeneratedActionPackage {
        cargo_toml,
        build_rs,
        lib_rs,
        action_rmw,
        action_idiomatic,
    })
}

// ============================================================================
// nano-ros Backend Generator Functions
// ============================================================================

/// Generated nano-ros message package
pub struct GeneratedNanoRosPackage {
    pub cargo_toml: String,
    pub lib_rs: String,
    pub message_rs: String,
}

/// Generated nano-ros service package
pub struct GeneratedNanoRosServicePackage {
    pub cargo_toml: String,
    pub lib_rs: String,
    pub service_rs: String,
}

/// Get the CDR primitive method name for a primitive type
fn primitive_to_cdr_method(prim: &rosidl_parser::PrimitiveType) -> String {
    use rosidl_parser::PrimitiveType;
    match prim {
        PrimitiveType::Bool => "bool".to_string(),
        PrimitiveType::Byte => "u8".to_string(),
        PrimitiveType::Char => "u8".to_string(),
        PrimitiveType::Int8 => "i8".to_string(),
        PrimitiveType::UInt8 => "u8".to_string(),
        PrimitiveType::Int16 => "i16".to_string(),
        PrimitiveType::UInt16 => "u16".to_string(),
        PrimitiveType::Int32 => "i32".to_string(),
        PrimitiveType::UInt32 => "u32".to_string(),
        PrimitiveType::Int64 => "i64".to_string(),
        PrimitiveType::UInt64 => "u64".to_string(),
        PrimitiveType::Float32 => "f32".to_string(),
        PrimitiveType::Float64 => "f64".to_string(),
    }
}

/// Convert a Message field to NanoRosField
fn field_to_nano_ros_field(field: &rosidl_parser::Field, package_name: &str) -> NanoRosField {
    let name = escape_keyword(&field.name);
    let rust_type = nano_ros_type_for_field(&field.field_type, Some(package_name));

    // Determine field properties
    let (is_primitive, primitive_method) = match &field.field_type {
        FieldType::Primitive(prim) => (true, primitive_to_cdr_method(prim)),
        _ => (false, String::new()),
    };

    let is_string = matches!(
        &field.field_type,
        FieldType::String
            | FieldType::BoundedString(_)
            | FieldType::WString
            | FieldType::BoundedWString(_)
    );

    let (is_array, array_size) = match &field.field_type {
        FieldType::Array { size, .. } => (true, *size),
        _ => (false, 0),
    };

    let is_sequence = matches!(
        &field.field_type,
        FieldType::Sequence { .. } | FieldType::BoundedSequence { .. }
    );

    let is_nested = matches!(&field.field_type, FieldType::NamespacedType { .. });

    // Element type info for arrays and sequences
    let (is_primitive_element, is_string_element, element_primitive_method) =
        match &field.field_type {
            FieldType::Array { element_type, .. }
            | FieldType::Sequence { element_type }
            | FieldType::BoundedSequence { element_type, .. } => match element_type.as_ref() {
                FieldType::Primitive(prim) => (true, false, primitive_to_cdr_method(prim)),
                FieldType::String
                | FieldType::BoundedString(_)
                | FieldType::WString
                | FieldType::BoundedWString(_) => (false, true, String::new()),
                _ => (false, false, String::new()),
            },
            _ => (false, false, String::new()),
        };

    NanoRosField {
        name,
        rust_type,
        primitive_method,
        element_primitive_method,
        array_size,
        is_primitive,
        is_string,
        is_array,
        is_sequence,
        is_nested,
        is_primitive_element,
        is_string_element,
    }
}

/// Generate a nano-ros message package
pub fn generate_nano_ros_message_package(
    package_name: &str,
    message_name: &str,
    message: &Message,
    all_dependencies: &HashSet<String>,
    package_version: &str,
) -> Result<GeneratedNanoRosPackage, GeneratorError> {
    // Extract dependencies from this specific message
    let msg_deps = extract_dependencies(message);

    // Combine with externally provided dependencies
    let mut all_deps: Vec<String> = all_dependencies.iter().cloned().collect();
    all_deps.extend(msg_deps);
    all_deps.sort();
    all_deps.dedup();

    // Generate Cargo.toml
    let cargo_toml_template = CargoNanoRosTomlTemplate {
        package_name,
        package_version,
        dependencies: &all_deps,
    };
    let cargo_toml = cargo_toml_template.render()?;

    // Generate lib.rs
    let lib_rs_template = LibNanoRosRsTemplate {
        has_messages: true,
        has_services: false,
        has_actions: false,
    };
    let lib_rs = lib_rs_template.render()?;

    // Generate message fields
    let fields: Vec<NanoRosField> = message
        .fields
        .iter()
        .map(|f| field_to_nano_ros_field(f, package_name))
        .collect();

    // Generate constants
    let constants: Vec<MessageConstant> = message
        .constants
        .iter()
        .map(|c| MessageConstant {
            name: c.name.clone(),
            rust_type: nano_ros_type_for_constant(&c.constant_type),
            value: constant_value_to_rust(&c.value),
        })
        .collect();

    // For now, use a placeholder type hash (in production, compute from IDL)
    let type_hash = "0000000000000000000000000000000000000000000000000000000000000000";

    let has_fields = !fields.is_empty();
    let message_template = MessageNanoRosTemplate {
        package_name,
        message_name,
        type_hash,
        fields,
        constants,
        has_fields,
    };
    let message_rs = message_template.render()?;

    Ok(GeneratedNanoRosPackage {
        cargo_toml,
        lib_rs,
        message_rs,
    })
}

/// Generate a nano-ros service package
pub fn generate_nano_ros_service_package(
    package_name: &str,
    service_name: &str,
    service: &Service,
    all_dependencies: &HashSet<String>,
    package_version: &str,
) -> Result<GeneratedNanoRosServicePackage, GeneratorError> {
    // Extract dependencies from request and response
    let mut req_deps = extract_dependencies(&service.request);
    let resp_deps = extract_dependencies(&service.response);
    req_deps.extend(resp_deps);

    // Combine with externally provided dependencies
    let mut all_deps: Vec<String> = all_dependencies.iter().cloned().collect();
    all_deps.extend(req_deps);
    all_deps.sort();
    all_deps.dedup();

    // Generate Cargo.toml
    let cargo_toml_template = CargoNanoRosTomlTemplate {
        package_name,
        package_version,
        dependencies: &all_deps,
    };
    let cargo_toml = cargo_toml_template.render()?;

    // Generate lib.rs
    let lib_rs_template = LibNanoRosRsTemplate {
        has_messages: false,
        has_services: true,
        has_actions: false,
    };
    let lib_rs = lib_rs_template.render()?;

    // Generate request fields
    let request_fields: Vec<NanoRosField> = service
        .request
        .fields
        .iter()
        .map(|f| field_to_nano_ros_field(f, package_name))
        .collect();

    let request_constants: Vec<MessageConstant> = service
        .request
        .constants
        .iter()
        .map(|c| MessageConstant {
            name: c.name.clone(),
            rust_type: nano_ros_type_for_constant(&c.constant_type),
            value: constant_value_to_rust(&c.value),
        })
        .collect();

    // Generate response fields
    let response_fields: Vec<NanoRosField> = service
        .response
        .fields
        .iter()
        .map(|f| field_to_nano_ros_field(f, package_name))
        .collect();

    let response_constants: Vec<MessageConstant> = service
        .response
        .constants
        .iter()
        .map(|c| MessageConstant {
            name: c.name.clone(),
            rust_type: nano_ros_type_for_constant(&c.constant_type),
            value: constant_value_to_rust(&c.value),
        })
        .collect();

    // For now, use a placeholder type hash
    let type_hash = "0000000000000000000000000000000000000000000000000000000000000000";

    let has_request_fields = !request_fields.is_empty();
    let has_response_fields = !response_fields.is_empty();
    let service_template = ServiceNanoRosTemplate {
        package_name,
        service_name,
        type_hash,
        request_fields,
        request_constants,
        response_fields,
        response_constants,
        has_request_fields,
        has_response_fields,
    };
    let service_rs = service_template.render()?;

    Ok(GeneratedNanoRosServicePackage {
        cargo_toml,
        lib_rs,
        service_rs,
    })
}

/// Generated nano-ros action package
pub struct GeneratedNanoRosActionPackage {
    pub cargo_toml: String,
    pub lib_rs: String,
    pub action_rs: String,
}

/// Generate a nano-ros action package
pub fn generate_nano_ros_action_package(
    package_name: &str,
    action_name: &str,
    action: &Action,
    all_dependencies: &HashSet<String>,
    package_version: &str,
) -> Result<GeneratedNanoRosActionPackage, GeneratorError> {
    // Extract dependencies from goal, result, and feedback
    let mut goal_deps = extract_dependencies(&action.spec.goal);
    let result_deps = extract_dependencies(&action.spec.result);
    let feedback_deps = extract_dependencies(&action.spec.feedback);
    goal_deps.extend(result_deps);
    goal_deps.extend(feedback_deps);

    // Combine with externally provided dependencies
    let mut all_deps: Vec<String> = all_dependencies.iter().cloned().collect();
    all_deps.extend(goal_deps);
    all_deps.sort();
    all_deps.dedup();

    // Generate Cargo.toml
    let cargo_toml_template = CargoNanoRosTomlTemplate {
        package_name,
        package_version,
        dependencies: &all_deps,
    };
    let cargo_toml = cargo_toml_template.render()?;

    // Generate lib.rs
    let lib_rs_template = LibNanoRosRsTemplate {
        has_messages: false,
        has_services: false,
        has_actions: true,
    };
    let lib_rs = lib_rs_template.render()?;

    // Generate goal fields
    let goal_fields: Vec<NanoRosField> = action
        .spec
        .goal
        .fields
        .iter()
        .map(|f| field_to_nano_ros_field(f, package_name))
        .collect();

    let goal_constants: Vec<MessageConstant> = action
        .spec
        .goal
        .constants
        .iter()
        .map(|c| MessageConstant {
            name: c.name.clone(),
            rust_type: nano_ros_type_for_constant(&c.constant_type),
            value: constant_value_to_rust(&c.value),
        })
        .collect();

    // Generate result fields
    let result_fields: Vec<NanoRosField> = action
        .spec
        .result
        .fields
        .iter()
        .map(|f| field_to_nano_ros_field(f, package_name))
        .collect();

    let result_constants: Vec<MessageConstant> = action
        .spec
        .result
        .constants
        .iter()
        .map(|c| MessageConstant {
            name: c.name.clone(),
            rust_type: nano_ros_type_for_constant(&c.constant_type),
            value: constant_value_to_rust(&c.value),
        })
        .collect();

    // Generate feedback fields
    let feedback_fields: Vec<NanoRosField> = action
        .spec
        .feedback
        .fields
        .iter()
        .map(|f| field_to_nano_ros_field(f, package_name))
        .collect();

    let feedback_constants: Vec<MessageConstant> = action
        .spec
        .feedback
        .constants
        .iter()
        .map(|c| MessageConstant {
            name: c.name.clone(),
            rust_type: nano_ros_type_for_constant(&c.constant_type),
            value: constant_value_to_rust(&c.value),
        })
        .collect();

    // For now, use a placeholder type hash
    let type_hash = "0000000000000000000000000000000000000000000000000000000000000000";

    let has_goal_fields = !goal_fields.is_empty();
    let has_result_fields = !result_fields.is_empty();
    let has_feedback_fields = !feedback_fields.is_empty();

    let action_template = ActionNanoRosTemplate {
        package_name,
        action_name,
        type_hash,
        goal_fields,
        goal_constants,
        result_fields,
        result_constants,
        feedback_fields,
        feedback_constants,
        has_goal_fields,
        has_result_fields,
        has_feedback_fields,
    };
    let action_rs = action_template.render()?;

    Ok(GeneratedNanoRosActionPackage {
        cargo_toml,
        lib_rs,
        action_rs,
    })
}

// ============================================================================
// C Code Generation (for nano-ros-c)
// ============================================================================

/// Generated C message package
pub struct GeneratedCPackage {
    /// Header file content (.h)
    pub header: String,
    /// Source file content (.c)
    pub source: String,
    /// Header filename
    pub header_name: String,
    /// Source filename
    pub source_name: String,
}

/// Generate C code for a message type
pub fn generate_c_message_package(
    package_name: &str,
    message_name: &str,
    message: &Message,
    type_hash: &str,
) -> Result<GeneratedCPackage, GeneratorError> {
    let c_pkg_name = to_c_package_name(package_name);
    let msg_snake = to_snake_case(message_name);

    // Build struct and guard names
    let struct_name = format!("{}_msg_{}", c_pkg_name, msg_snake);
    let guard_name = format!(
        "{}_MSG_{}_H",
        c_pkg_name.to_uppercase(),
        msg_snake.to_uppercase()
    );
    let constant_prefix = format!(
        "{}_MSG_{}",
        c_pkg_name.to_uppercase(),
        msg_snake.to_uppercase()
    );
    let header_name = format!("{}_msg_{}.h", c_pkg_name, msg_snake);
    let source_name = format!("{}_msg_{}.c", c_pkg_name, msg_snake);

    // Extract dependencies
    let mut dependencies = Vec::new();
    for field in &message.fields {
        if let FieldType::NamespacedType {
            package: Some(pkg), ..
        } = &field.field_type
        {
            let dep = to_c_package_name(pkg);
            if !dependencies.contains(&dep) {
                dependencies.push(dep);
            }
        }
    }
    dependencies.sort();

    // Build C fields
    let fields: Vec<CField> = message
        .fields
        .iter()
        .map(|field| build_c_field(&field.name, &field.field_type, Some(package_name)))
        .collect();

    // Build C constants
    let constants: Vec<CConstant> = message
        .constants
        .iter()
        .map(|constant| CConstant {
            name: constant.name.clone(),
            c_type: c_type_for_constant(&constant.constant_type),
            value: constant_value_to_rust(&constant.value),
        })
        .collect();

    let has_fields = !fields.is_empty();

    // Generate header
    let header_template = MessageCHeaderTemplate {
        package_name,
        message_name,
        type_hash,
        guard_name,
        struct_name: struct_name.clone(),
        constant_prefix,
        fields: fields.clone(),
        constants,
        dependencies,
        has_fields,
    };
    let header = header_template.render()?;

    // Generate source
    let source_template = MessageCSourceTemplate {
        package_name,
        message_name,
        type_hash,
        header_name: header_name.clone(),
        struct_name,
        fields,
        has_fields,
    };
    let source = source_template.render()?;

    Ok(GeneratedCPackage {
        header,
        source,
        header_name,
        source_name,
    })
}

/// Generated C service package
pub struct GeneratedCServicePackage {
    /// Header file content (.h)
    pub header: String,
    /// Source file content (.c)
    pub source: String,
    /// Header filename
    pub header_name: String,
    /// Source filename
    pub source_name: String,
}

/// Generate C code for a service type
pub fn generate_c_service_package(
    package_name: &str,
    service_name: &str,
    service: &Service,
    type_hash: &str,
) -> Result<GeneratedCServicePackage, GeneratorError> {
    let c_pkg_name = to_c_package_name(package_name);
    let srv_snake = to_snake_case(service_name);

    // Build struct and guard names
    let service_struct_name = format!("{}_srv_{}", c_pkg_name, srv_snake);
    let request_struct_name = format!("{}_srv_{}_request", c_pkg_name, srv_snake);
    let response_struct_name = format!("{}_srv_{}_response", c_pkg_name, srv_snake);
    let guard_name = format!(
        "{}_SRV_{}_H",
        c_pkg_name.to_uppercase(),
        srv_snake.to_uppercase()
    );
    let constant_prefix = format!(
        "{}_SRV_{}",
        c_pkg_name.to_uppercase(),
        srv_snake.to_uppercase()
    );
    let header_name = format!("{}_srv_{}.h", c_pkg_name, srv_snake);
    let source_name = format!("{}_srv_{}.c", c_pkg_name, srv_snake);

    // Extract dependencies from both request and response
    let mut dependencies = Vec::new();
    for field in service
        .request
        .fields
        .iter()
        .chain(service.response.fields.iter())
    {
        if let FieldType::NamespacedType {
            package: Some(pkg), ..
        } = &field.field_type
        {
            let dep = to_c_package_name(pkg);
            if !dependencies.contains(&dep) {
                dependencies.push(dep);
            }
        }
    }
    dependencies.sort();

    // Build C fields for request
    let request_fields: Vec<CField> = service
        .request
        .fields
        .iter()
        .map(|field| build_c_field(&field.name, &field.field_type, Some(package_name)))
        .collect();

    let request_constants: Vec<CConstant> = service
        .request
        .constants
        .iter()
        .map(|constant| CConstant {
            name: constant.name.clone(),
            c_type: c_type_for_constant(&constant.constant_type),
            value: constant_value_to_rust(&constant.value),
        })
        .collect();

    // Build C fields for response
    let response_fields: Vec<CField> = service
        .response
        .fields
        .iter()
        .map(|field| build_c_field(&field.name, &field.field_type, Some(package_name)))
        .collect();

    let response_constants: Vec<CConstant> = service
        .response
        .constants
        .iter()
        .map(|constant| CConstant {
            name: constant.name.clone(),
            c_type: c_type_for_constant(&constant.constant_type),
            value: constant_value_to_rust(&constant.value),
        })
        .collect();

    let has_request_fields = !request_fields.is_empty();
    let has_response_fields = !response_fields.is_empty();

    // Generate header
    let header_template = ServiceCHeaderTemplate {
        package_name,
        service_name,
        type_hash,
        guard_name,
        service_struct_name: service_struct_name.clone(),
        request_struct_name: request_struct_name.clone(),
        response_struct_name: response_struct_name.clone(),
        constant_prefix,
        request_fields: request_fields.clone(),
        request_constants,
        response_fields: response_fields.clone(),
        response_constants,
        dependencies,
        has_request_fields,
        has_response_fields,
    };
    let header = header_template.render()?;

    // Generate source
    let source_template = ServiceCSourceTemplate {
        package_name,
        service_name,
        type_hash,
        header_name: header_name.clone(),
        service_struct_name,
        request_struct_name,
        response_struct_name,
        request_fields,
        response_fields,
        has_request_fields,
        has_response_fields,
    };
    let source = source_template.render()?;

    Ok(GeneratedCServicePackage {
        header,
        source,
        header_name,
        source_name,
    })
}

/// Generated C action package
pub struct GeneratedCActionPackage {
    /// Header file content (.h)
    pub header: String,
    /// Source file content (.c)
    pub source: String,
    /// Header filename
    pub header_name: String,
    /// Source filename
    pub source_name: String,
}

/// Generate C code for an action type
pub fn generate_c_action_package(
    package_name: &str,
    action_name: &str,
    action: &Action,
    type_hash: &str,
) -> Result<GeneratedCActionPackage, GeneratorError> {
    let c_pkg_name = to_c_package_name(package_name);
    let action_snake = to_snake_case(action_name);

    // Build struct and guard names
    let action_struct_name = format!("{}_action_{}", c_pkg_name, action_snake);
    let goal_struct_name = format!("{}_action_{}_goal", c_pkg_name, action_snake);
    let result_struct_name = format!("{}_action_{}_result", c_pkg_name, action_snake);
    let feedback_struct_name = format!("{}_action_{}_feedback", c_pkg_name, action_snake);
    let guard_name = format!(
        "{}_ACTION_{}_H",
        c_pkg_name.to_uppercase(),
        action_snake.to_uppercase()
    );
    let constant_prefix = format!(
        "{}_ACTION_{}",
        c_pkg_name.to_uppercase(),
        action_snake.to_uppercase()
    );
    let header_name = format!("{}_action_{}.h", c_pkg_name, action_snake);
    let source_name = format!("{}_action_{}.c", c_pkg_name, action_snake);

    // Extract dependencies from goal, result, and feedback
    let mut dependencies = Vec::new();
    for field in action
        .spec
        .goal
        .fields
        .iter()
        .chain(action.spec.result.fields.iter())
        .chain(action.spec.feedback.fields.iter())
    {
        if let FieldType::NamespacedType {
            package: Some(pkg), ..
        } = &field.field_type
        {
            let dep = to_c_package_name(pkg);
            if !dependencies.contains(&dep) {
                dependencies.push(dep);
            }
        }
    }
    dependencies.sort();

    // Build C fields for goal
    let goal_fields: Vec<CField> = action
        .spec
        .goal
        .fields
        .iter()
        .map(|field| build_c_field(&field.name, &field.field_type, Some(package_name)))
        .collect();

    let goal_constants: Vec<CConstant> = action
        .spec
        .goal
        .constants
        .iter()
        .map(|constant| CConstant {
            name: constant.name.clone(),
            c_type: c_type_for_constant(&constant.constant_type),
            value: constant_value_to_rust(&constant.value),
        })
        .collect();

    // Build C fields for result
    let result_fields: Vec<CField> = action
        .spec
        .result
        .fields
        .iter()
        .map(|field| build_c_field(&field.name, &field.field_type, Some(package_name)))
        .collect();

    let result_constants: Vec<CConstant> = action
        .spec
        .result
        .constants
        .iter()
        .map(|constant| CConstant {
            name: constant.name.clone(),
            c_type: c_type_for_constant(&constant.constant_type),
            value: constant_value_to_rust(&constant.value),
        })
        .collect();

    // Build C fields for feedback
    let feedback_fields: Vec<CField> = action
        .spec
        .feedback
        .fields
        .iter()
        .map(|field| build_c_field(&field.name, &field.field_type, Some(package_name)))
        .collect();

    let feedback_constants: Vec<CConstant> = action
        .spec
        .feedback
        .constants
        .iter()
        .map(|constant| CConstant {
            name: constant.name.clone(),
            c_type: c_type_for_constant(&constant.constant_type),
            value: constant_value_to_rust(&constant.value),
        })
        .collect();

    let has_goal_fields = !goal_fields.is_empty();
    let has_result_fields = !result_fields.is_empty();
    let has_feedback_fields = !feedback_fields.is_empty();

    // Generate header
    let header_template = ActionCHeaderTemplate {
        package_name,
        action_name,
        type_hash,
        guard_name,
        action_struct_name: action_struct_name.clone(),
        goal_struct_name: goal_struct_name.clone(),
        result_struct_name: result_struct_name.clone(),
        feedback_struct_name: feedback_struct_name.clone(),
        constant_prefix,
        goal_fields: goal_fields.clone(),
        goal_constants,
        result_fields: result_fields.clone(),
        result_constants,
        feedback_fields: feedback_fields.clone(),
        feedback_constants,
        dependencies,
        has_goal_fields,
        has_result_fields,
        has_feedback_fields,
    };
    let header = header_template.render()?;

    // Generate source
    let source_template = ActionCSourceTemplate {
        package_name,
        action_name,
        type_hash,
        header_name: header_name.clone(),
        action_struct_name,
        goal_struct_name,
        result_struct_name,
        feedback_struct_name,
        goal_fields,
        result_fields,
        feedback_fields,
        has_goal_fields,
        has_result_fields,
        has_feedback_fields,
    };
    let source = source_template.render()?;

    Ok(GeneratedCActionPackage {
        header,
        source,
        header_name,
        source_name,
    })
}

/// Build a CField from a field type
fn build_c_field(name: &str, field_type: &FieldType, current_package: Option<&str>) -> CField {
    let escaped_name = escape_keyword(name);
    let c_type = c_type_for_field(field_type, current_package);
    let array_suffix = c_array_suffix_for_field(field_type);

    // Determine type characteristics
    let (is_primitive, primitive_type) = match field_type {
        FieldType::Primitive(prim) => (true, Some(prim)),
        _ => (false, None),
    };

    let is_string = matches!(
        field_type,
        FieldType::String
            | FieldType::BoundedString(_)
            | FieldType::WString
            | FieldType::BoundedWString(_)
    );

    let is_array = matches!(field_type, FieldType::Array { .. });
    let is_sequence = matches!(
        field_type,
        FieldType::Sequence { .. } | FieldType::BoundedSequence { .. }
    );
    let is_nested = matches!(field_type, FieldType::NamespacedType { .. });

    // Get array/sequence info
    let (array_size, sequence_capacity) = match field_type {
        FieldType::Array { size, .. } => (*size, 0),
        FieldType::Sequence { .. } => (0, C_DEFAULT_SEQUENCE_CAPACITY),
        FieldType::BoundedSequence { max_size, .. } => (0, *max_size),
        _ => (0, 0),
    };

    // Get element info for arrays/sequences
    let (is_primitive_element, is_string_element, element_type) = match field_type {
        FieldType::Array { element_type, .. }
        | FieldType::Sequence { element_type }
        | FieldType::BoundedSequence { element_type, .. } => {
            let is_prim = matches!(element_type.as_ref(), FieldType::Primitive(_));
            let is_str = matches!(
                element_type.as_ref(),
                FieldType::String
                    | FieldType::BoundedString(_)
                    | FieldType::WString
                    | FieldType::BoundedWString(_)
            );
            (is_prim, is_str, Some(element_type.as_ref()))
        }
        _ => (false, false, None),
    };

    // Get CDR methods
    let (cdr_write_method, cdr_read_method) = if let Some(prim) = primitive_type {
        (
            c_cdr_write_method(prim).to_string(),
            c_cdr_read_method(prim).to_string(),
        )
    } else {
        (String::new(), String::new())
    };

    let (element_cdr_write_method, element_cdr_read_method) =
        if let Some(FieldType::Primitive(prim)) = element_type {
            (
                c_cdr_write_method(prim).to_string(),
                c_cdr_read_method(prim).to_string(),
            )
        } else {
            (String::new(), String::new())
        };

    // Get nested struct names
    let nested_struct_name = if let FieldType::NamespacedType { package, name } = field_type {
        if let Some(pkg) = package {
            format!("{}_msg_{}", to_c_package_name(pkg), to_snake_case(name))
        } else {
            format!("msg_{}", to_snake_case(name))
        }
    } else {
        String::new()
    };

    let element_struct_name =
        if let Some(FieldType::NamespacedType { package, name }) = element_type {
            if let Some(pkg) = package {
                format!("{}_msg_{}", to_c_package_name(pkg), to_snake_case(name))
            } else {
                format!("msg_{}", to_snake_case(name))
            }
        } else {
            String::new()
        };

    CField {
        name: escaped_name,
        c_type,
        array_suffix,
        cdr_write_method,
        cdr_read_method,
        element_cdr_write_method,
        element_cdr_read_method,
        array_size,
        sequence_capacity,
        nested_struct_name,
        element_struct_name,
        is_primitive,
        is_string,
        is_array,
        is_sequence,
        is_nested,
        is_primitive_element,
        is_string_element,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rosidl_parser::{
        parse_action, parse_message, parse_service, Field, FieldType, PrimitiveType,
    };

    #[test]
    fn test_simple_message_generation() {
        let msg = parse_message("int32 x\nfloat64 y\n").unwrap();
        let deps = HashSet::new();

        let result = generate_message_package("test_msgs", "Point", &msg, &deps);
        assert!(result.is_ok());

        let pkg = result.unwrap();
        assert!(pkg.cargo_toml.contains("test_msgs"));
        assert!(pkg.message_rmw.contains("i32"));
        assert!(pkg.message_rmw.contains("f64"));
    }

    #[test]
    fn test_message_with_dependencies() {
        let msg = parse_message("geometry_msgs/Point position\n").unwrap();
        let deps = HashSet::new();

        let result = generate_message_package("nav_msgs", "Odometry", &msg, &deps);
        assert!(result.is_ok());

        let pkg = result.unwrap();
        assert!(pkg.cargo_toml.contains("geometry_msgs"));
    }

    #[test]
    fn test_message_with_large_array() {
        let mut msg = Message::new();
        msg.fields.push(Field {
            field_type: FieldType::Array {
                element_type: Box::new(FieldType::Primitive(PrimitiveType::Int32)),
                size: 64,
            },
            name: "data".to_string(),
            default_value: None,
        });

        let deps = HashSet::new();
        let result = generate_message_package("test_msgs", "LargeArray", &msg, &deps);
        assert!(result.is_ok());

        let pkg = result.unwrap();
        assert!(pkg.cargo_toml.contains("big-array"));
    }

    #[test]
    fn test_message_with_keyword_field() {
        let msg = parse_message("int32 type\nfloat64 match\n").unwrap();
        let deps = HashSet::new();

        let result = generate_message_package("test_msgs", "Keywords", &msg, &deps);
        assert!(result.is_ok());

        let pkg = result.unwrap();
        assert!(pkg.message_rmw.contains("type_"));
        assert!(pkg.message_rmw.contains("match_"));
    }

    #[test]
    fn test_simple_service_generation() {
        let srv = parse_service("int32 a\nint32 b\n---\nint32 sum\n").unwrap();
        let deps = HashSet::new();

        let result = generate_service_package("example_interfaces", "AddTwoInts", &srv, &deps);
        assert!(result.is_ok());

        let pkg = result.unwrap();
        assert!(pkg.cargo_toml.contains("example_interfaces"));
        assert!(pkg.lib_rs.contains("pub mod srv"));
        assert!(pkg.service_rmw.contains("AddTwoIntsRequest"));
        assert!(pkg.service_rmw.contains("AddTwoIntsResponse"));
        assert!(pkg.service_idiomatic.contains("AddTwoIntsRequest"));
        assert!(pkg.service_idiomatic.contains("AddTwoIntsResponse"));
    }

    #[test]
    fn test_service_with_dependencies() {
        let srv = parse_service("geometry_msgs/Point position\n---\nbool success\n").unwrap();
        let deps = HashSet::new();

        let result = generate_service_package("test_srvs", "CheckPoint", &srv, &deps);
        assert!(result.is_ok());

        let pkg = result.unwrap();
        assert!(pkg.cargo_toml.contains("geometry_msgs"));
    }

    #[test]
    fn test_simple_action_generation() {
        let action =
            parse_action("int32 order\n---\nint32[] sequence\n---\nint32[] partial_sequence\n")
                .unwrap();
        let deps = HashSet::new();

        let result = generate_action_package("example_interfaces", "Fibonacci", &action, &deps);
        assert!(result.is_ok());

        let pkg = result.unwrap();
        assert!(pkg.cargo_toml.contains("example_interfaces"));
        assert!(pkg.lib_rs.contains("pub mod action"));
        assert!(pkg.action_rmw.contains("FibonacciGoal"));
        assert!(pkg.action_rmw.contains("FibonacciResult"));
        assert!(pkg.action_rmw.contains("FibonacciFeedback"));
        assert!(pkg.action_idiomatic.contains("FibonacciGoal"));
        assert!(pkg.action_idiomatic.contains("FibonacciResult"));
        assert!(pkg.action_idiomatic.contains("FibonacciFeedback"));
    }

    #[test]
    fn test_action_with_dependencies() {
        let action = parse_action(
            "geometry_msgs/Point target\n---\nfloat64 distance\n---\nfloat64 current_distance\n",
        )
        .unwrap();
        let deps = HashSet::new();

        let result = generate_action_package("test_actions", "Navigate", &action, &deps);
        assert!(result.is_ok());

        let pkg = result.unwrap();
        assert!(pkg.cargo_toml.contains("geometry_msgs"));
    }

    // ========================================================================
    // nano-ros Backend Tests
    // ========================================================================

    #[test]
    fn test_nano_ros_simple_message_generation() {
        let msg = parse_message("int32 x\nfloat64 y\nstring name\n").unwrap();
        let deps = HashSet::new();

        let result = generate_nano_ros_message_package("test_msgs", "Point", &msg, &deps, "0.1.0");
        assert!(result.is_ok());

        let pkg = result.unwrap();

        // Check Cargo.toml has nano-ros dependencies
        assert!(pkg.cargo_toml.contains("nano-ros-core"));
        assert!(pkg.cargo_toml.contains("nano-ros-serdes"));
        assert!(pkg.cargo_toml.contains("heapless"));

        // Check lib.rs is no_std
        assert!(pkg.lib_rs.contains("#![no_std]"));
        assert!(pkg.lib_rs.contains("pub mod msg"));

        // Check message contains proper types
        assert!(pkg.message_rs.contains("pub x: i32"));
        assert!(pkg.message_rs.contains("pub y: f64"));
        assert!(pkg.message_rs.contains("heapless::String<256>"));

        // Check it has Serialize/Deserialize implementations
        assert!(pkg.message_rs.contains("impl Serialize for Point"));
        assert!(pkg.message_rs.contains("impl Deserialize for Point"));
        assert!(pkg.message_rs.contains("impl RosMessage for Point"));
    }

    #[test]
    fn test_nano_ros_message_with_sequence() {
        let msg = parse_message("int32[] data\n").unwrap();
        let deps = HashSet::new();

        let result =
            generate_nano_ros_message_package("test_msgs", "IntArray", &msg, &deps, "0.1.0");
        assert!(result.is_ok());

        let pkg = result.unwrap();
        // Check sequence uses heapless::Vec
        assert!(pkg.message_rs.contains("heapless::Vec<i32"));
    }

    #[test]
    fn test_nano_ros_service_generation() {
        let srv = parse_service("int64 a\nint64 b\n---\nint64 sum\n").unwrap();
        let deps = HashSet::new();

        let result =
            generate_nano_ros_service_package("test_srvs", "AddTwoInts", &srv, &deps, "0.1.0");
        assert!(result.is_ok());

        let pkg = result.unwrap();

        // Check Cargo.toml
        assert!(pkg.cargo_toml.contains("nano-ros-core"));

        // Check lib.rs
        assert!(pkg.lib_rs.contains("pub mod srv"));

        // Check service types
        assert!(pkg.service_rs.contains("AddTwoIntsRequest"));
        assert!(pkg.service_rs.contains("AddTwoIntsResponse"));
        assert!(pkg.service_rs.contains("pub a: i64"));
        assert!(pkg.service_rs.contains("pub b: i64"));
        assert!(pkg.service_rs.contains("pub sum: i64"));

        // Check RosService impl
        assert!(pkg.service_rs.contains("impl RosService for AddTwoInts"));
    }

    #[test]
    fn test_nano_ros_action_generation() {
        let action =
            parse_action("int32 order\n---\nint32[] sequence\n---\nint32[] partial_sequence\n")
                .unwrap();
        let deps = HashSet::new();

        let result = generate_nano_ros_action_package(
            "example_interfaces",
            "Fibonacci",
            &action,
            &deps,
            "0.1.0",
        );
        assert!(result.is_ok());

        let pkg = result.unwrap();

        // Check Cargo.toml
        assert!(pkg.cargo_toml.contains("nano-ros-core"));

        // Check lib.rs
        assert!(pkg.lib_rs.contains("pub mod action"));

        // Check action types
        assert!(pkg.action_rs.contains("FibonacciGoal"));
        assert!(pkg.action_rs.contains("FibonacciResult"));
        assert!(pkg.action_rs.contains("FibonacciFeedback"));
        assert!(pkg.action_rs.contains("pub order: i32"));

        // Check RosAction impl
        assert!(pkg.action_rs.contains("impl RosAction for Fibonacci"));
        assert!(pkg.action_rs.contains("type Goal = FibonacciGoal"));
        assert!(pkg.action_rs.contains("type Result = FibonacciResult"));
        assert!(pkg.action_rs.contains("type Feedback = FibonacciFeedback"));
    }

    // ========================================================================
    // C Code Generation Tests
    // ========================================================================

    #[test]
    fn test_c_simple_message_generation() {
        let msg = parse_message("int32 x\nfloat64 y\nbool flag\n").unwrap();
        let type_hash = "abc123";

        let result = generate_c_message_package("test_msgs", "Point", &msg, type_hash);
        assert!(result.is_ok());

        let pkg = result.unwrap();

        // Check header file
        assert!(pkg.header.contains("#ifndef TEST_MSGS_MSG_POINT_H"));
        assert!(pkg.header.contains("typedef struct test_msgs_msg_point"));
        assert!(pkg.header.contains("int32_t x"));
        assert!(pkg.header.contains("double y"));
        assert!(pkg.header.contains("bool flag"));
        assert!(pkg.header.contains("test_msgs_msg_point_init"));
        assert!(pkg.header.contains("test_msgs_msg_point_serialize"));
        assert!(pkg.header.contains("test_msgs_msg_point_deserialize"));

        // Check source file
        assert!(pkg.source.contains("test_msgs_msg_point.h"));
        assert!(pkg.source.contains("nano_ros_cdr_write_i32"));
        assert!(pkg.source.contains("nano_ros_cdr_write_f64"));
        assert!(pkg.source.contains("nano_ros_cdr_write_bool"));

        // Check file names
        assert_eq!(pkg.header_name, "test_msgs_msg_point.h");
        assert_eq!(pkg.source_name, "test_msgs_msg_point.c");
    }

    #[test]
    fn test_c_message_with_string() {
        let msg = parse_message("string name\n").unwrap();
        let type_hash = "def456";

        let result = generate_c_message_package("std_msgs", "String", &msg, type_hash);
        assert!(result.is_ok());

        let pkg = result.unwrap();
        assert!(pkg.header.contains("char name[256]"));
        assert!(pkg.source.contains("nano_ros_cdr_write_string"));
    }

    #[test]
    fn test_c_message_with_array() {
        let msg = parse_message("int32[3] values\n").unwrap();
        let type_hash = "ghi789";

        let result = generate_c_message_package("test_msgs", "IntArray", &msg, type_hash);
        assert!(result.is_ok());

        let pkg = result.unwrap();
        assert!(pkg.header.contains("int32_t values[3]"));
        assert!(pkg.source.contains("for (size_t i = 0; i < 3; ++i)"));
    }

    #[test]
    fn test_c_simple_service_generation() {
        let srv = parse_service("int32 a\nint32 b\n---\nint32 sum\n").unwrap();
        let type_hash = "srv123";

        let result = generate_c_service_package("test_srvs", "AddTwoInts", &srv, type_hash);
        assert!(result.is_ok());

        let pkg = result.unwrap();

        // Check header file
        assert!(pkg.header.contains("#ifndef TEST_SRVS_SRV_ADD_TWO_INTS_H"));
        assert!(pkg
            .header
            .contains("typedef struct test_srvs_srv_add_two_ints_request"));
        assert!(pkg
            .header
            .contains("typedef struct test_srvs_srv_add_two_ints_response"));
        assert!(pkg.header.contains("int32_t a"));
        assert!(pkg.header.contains("int32_t b"));
        assert!(pkg.header.contains("int32_t sum"));

        // Check source file
        assert!(pkg
            .source
            .contains("test_srvs_srv_add_two_ints_request_init"));
        assert!(pkg
            .source
            .contains("test_srvs_srv_add_two_ints_response_init"));
        assert!(pkg
            .source
            .contains("test_srvs_srv_add_two_ints_request_serialize"));
        assert!(pkg
            .source
            .contains("test_srvs_srv_add_two_ints_response_serialize"));

        // Check file names
        assert_eq!(pkg.header_name, "test_srvs_srv_add_two_ints.h");
        assert_eq!(pkg.source_name, "test_srvs_srv_add_two_ints.c");
    }

    #[test]
    fn test_c_simple_action_generation() {
        let action =
            parse_action("int32 order\n---\nint32 result_code\n---\nint32 progress\n").unwrap();
        let type_hash = "act456";

        let result = generate_c_action_package("test_actions", "Fibonacci", &action, type_hash);
        assert!(result.is_ok());

        let pkg = result.unwrap();

        // Check header file
        assert!(pkg
            .header
            .contains("#ifndef TEST_ACTIONS_ACTION_FIBONACCI_H"));
        assert!(pkg
            .header
            .contains("typedef struct test_actions_action_fibonacci_goal"));
        assert!(pkg
            .header
            .contains("typedef struct test_actions_action_fibonacci_result"));
        assert!(pkg
            .header
            .contains("typedef struct test_actions_action_fibonacci_feedback"));
        assert!(pkg.header.contains("int32_t order"));
        assert!(pkg.header.contains("int32_t result_code"));
        assert!(pkg.header.contains("int32_t progress"));

        // Check source file
        assert!(pkg
            .source
            .contains("test_actions_action_fibonacci_goal_init"));
        assert!(pkg
            .source
            .contains("test_actions_action_fibonacci_result_init"));
        assert!(pkg
            .source
            .contains("test_actions_action_fibonacci_feedback_init"));

        // Check file names
        assert_eq!(pkg.header_name, "test_actions_action_fibonacci.h");
        assert_eq!(pkg.source_name, "test_actions_action_fibonacci.c");
    }
}
