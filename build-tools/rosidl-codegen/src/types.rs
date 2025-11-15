use rosidl_parser::ast::ConstantValue;
use rosidl_parser::idl::ast::ConstantValue as IdlConstantValue;
use rosidl_parser::idl::types::IdlType;
use rosidl_parser::FieldType;

/// Check if a field type is a sequence (unbounded or bounded)
pub fn is_sequence_type(field_type: &FieldType) -> bool {
    matches!(
        field_type,
        FieldType::Sequence { .. } | FieldType::BoundedSequence { .. }
    )
}

/// Check if a field type is a primitive (no conversion needed)
pub fn is_primitive_type(field_type: &FieldType) -> bool {
    matches!(field_type, FieldType::Primitive(_))
}

/// Check if a field type is a string type (String, BoundedString, WString, BoundedWString)
pub fn is_string_type(field_type: &FieldType) -> bool {
    matches!(
        field_type,
        FieldType::String
            | FieldType::BoundedString(_)
            | FieldType::WString
            | FieldType::BoundedWString(_)
    )
}

/// Check if a field type is specifically an unbounded string
pub fn is_unbounded_string_type(field_type: &FieldType) -> bool {
    matches!(field_type, FieldType::String)
}

/// Check if a field type is specifically a bounded string
pub fn is_bounded_string_type(field_type: &FieldType) -> bool {
    matches!(field_type, FieldType::BoundedString(_))
}

/// Check if a field type is a WString type (unbounded or bounded)
pub fn is_wstring_type(field_type: &FieldType) -> bool {
    matches!(
        field_type,
        FieldType::WString | FieldType::BoundedWString(_)
    )
}

/// Check if a field type is specifically an unbounded WString
pub fn is_unbounded_wstring_type(field_type: &FieldType) -> bool {
    matches!(field_type, FieldType::WString)
}

/// Check if a field type is specifically a bounded WString
pub fn is_bounded_wstring_type(field_type: &FieldType) -> bool {
    matches!(field_type, FieldType::BoundedWString(_))
}

/// Check if a field type is a sequence of primitives (can be copied directly)
pub fn is_primitive_sequence(field_type: &FieldType) -> bool {
    match field_type {
        FieldType::Sequence { element_type } | FieldType::BoundedSequence { element_type, .. } => {
            matches!(**element_type, FieldType::Primitive(_))
        }
        _ => false,
    }
}

/// Check if a field type is a sequence of strings (any string type)
pub fn is_string_sequence(field_type: &FieldType) -> bool {
    match field_type {
        FieldType::Sequence { element_type } | FieldType::BoundedSequence { element_type, .. } => {
            is_string_type(element_type)
        }
        _ => false,
    }
}

/// Check if a field type is a sequence of unbounded strings
pub fn is_unbounded_string_sequence(field_type: &FieldType) -> bool {
    match field_type {
        FieldType::Sequence { element_type } | FieldType::BoundedSequence { element_type, .. } => {
            is_unbounded_string_type(element_type)
        }
        _ => false,
    }
}

/// Check if a field type is a sequence of bounded strings
pub fn is_bounded_string_sequence(field_type: &FieldType) -> bool {
    match field_type {
        FieldType::Sequence { element_type } | FieldType::BoundedSequence { element_type, .. } => {
            is_bounded_string_type(element_type)
        }
        _ => false,
    }
}

/// Check if a field type is an array (needs clone, not conversion)
pub fn is_array_type(field_type: &FieldType) -> bool {
    matches!(field_type, FieldType::Array { .. })
}

/// Check if a field type is a large array (> 32 elements, needs big_array for serde)
pub fn is_large_array(field_type: &FieldType) -> bool {
    matches!(field_type, FieldType::Array { size, .. } if *size > 32)
}

/// Check if a field type is an array of primitives (can be cloned directly)
pub fn is_primitive_array(field_type: &FieldType) -> bool {
    match field_type {
        FieldType::Array { element_type, .. } => matches!(**element_type, FieldType::Primitive(_)),
        _ => false,
    }
}

/// Check if a field type is an array of strings (any string type)
pub fn is_string_array(field_type: &FieldType) -> bool {
    match field_type {
        FieldType::Array { element_type, .. } => is_string_type(element_type),
        _ => false,
    }
}

/// Check if a field type is an array of unbounded strings
pub fn is_unbounded_string_array(field_type: &FieldType) -> bool {
    match field_type {
        FieldType::Array { element_type, .. } => is_unbounded_string_type(element_type),
        _ => false,
    }
}

/// Check if a field type is an array of bounded strings
pub fn is_bounded_string_array(field_type: &FieldType) -> bool {
    match field_type {
        FieldType::Array { element_type, .. } => is_bounded_string_type(element_type),
        _ => false,
    }
}

/// Check if a field type is an array of nested messages (needs element conversion)
pub fn is_nested_array(field_type: &FieldType) -> bool {
    match field_type {
        FieldType::Array { element_type, .. } => {
            matches!(**element_type, FieldType::NamespacedType { .. })
        }
        _ => false,
    }
}

/// Check if a field type is a bounded sequence (vs unbounded)
pub fn is_bounded_sequence(field_type: &FieldType) -> bool {
    matches!(field_type, FieldType::BoundedSequence { .. })
}

/// Convert a ConstantValue to a Rust code string
pub fn constant_value_to_rust(value: &ConstantValue) -> String {
    match value {
        ConstantValue::Integer(i) => i.to_string(),
        ConstantValue::UInteger(u) => u.to_string(),
        ConstantValue::Float(f) => {
            // Ensure float literals always have decimal point
            let s = f.to_string();
            if s.contains('.') || s.contains('e') || s.contains('E') {
                s
            } else {
                format!("{}.0", s)
            }
        }
        ConstantValue::Bool(b) => b.to_string(),
        ConstantValue::String(s) => format!("\"{}\"", s.escape_default()),
        ConstantValue::Array(values) => {
            let inner = values
                .iter()
                .map(constant_value_to_rust)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{}]", inner)
        }
    }
}

/// Rust keywords that need to be escaped
const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
];

/// Escape Rust keywords by appending underscore
pub fn escape_keyword(name: &str) -> String {
    if RUST_KEYWORDS.contains(&name) {
        format!("{}_", name)
    } else {
        name.to_string()
    }
}

/// Get the Rust type string for a field type
/// If `rmw_layer` is true, returns RMW types (rosidl_runtime_rs::*), else idiomatic types
/// `current_package` is used to detect self-references and use `crate::` instead of `pkg::`
pub fn rust_type_for_field(
    field_type: &FieldType,
    rmw_layer: bool,
    current_package: Option<&str>,
) -> String {
    match field_type {
        FieldType::Primitive(prim) => prim.rust_type().to_string(),

        FieldType::String => {
            if rmw_layer {
                "crate::rosidl_runtime_rs::String".to_string()
            } else {
                "std::string::String".to_string()
            }
        }

        FieldType::BoundedString(size) => {
            if rmw_layer {
                format!("crate::rosidl_runtime_rs::BoundedString<{}>", size)
            } else {
                // Idiomatic layer uses String even for bounded
                "std::string::String".to_string()
            }
        }

        FieldType::WString => {
            if rmw_layer {
                "crate::rosidl_runtime_rs::WString".to_string()
            } else {
                "std::string::String".to_string()
            }
        }

        FieldType::BoundedWString(size) => {
            if rmw_layer {
                format!("crate::rosidl_runtime_rs::BoundedWString<{}>", size)
            } else {
                "std::string::String".to_string()
            }
        }

        FieldType::Array { element_type, size } => {
            let elem = rust_type_for_field(element_type, rmw_layer, current_package);
            format!("[{}; {}]", elem, size)
        }

        FieldType::Sequence { element_type } => {
            let elem = rust_type_for_field(element_type, rmw_layer, current_package);
            if rmw_layer {
                format!("crate::rosidl_runtime_rs::Sequence<{}>", elem)
            } else {
                format!("std::vec::Vec<{}>", elem)
            }
        }

        FieldType::BoundedSequence {
            element_type,
            max_size,
        } => {
            let elem = rust_type_for_field(element_type, rmw_layer, current_package);
            if rmw_layer {
                format!(
                    "crate::rosidl_runtime_rs::BoundedSequence<{}, {}>",
                    elem, max_size
                )
            } else {
                // Idiomatic layer uses Vec even for bounded
                format!("std::vec::Vec<{}>", elem)
            }
        }

        FieldType::NamespacedType { package, name } => {
            // Check if this is a self-reference (same package referencing itself)
            let is_self_ref = package.as_deref() == current_package;

            if let Some(pkg) = package {
                let module_name = to_snake_case(name);
                if is_self_ref {
                    // Self-reference: use crate:: instead of pkg::
                    if rmw_layer {
                        // RMW layer uses FFI hierarchy: crate::ffi::msg::module::Type
                        format!("crate::ffi::msg::{}::{}", module_name, name)
                    } else {
                        // Idiomatic layer uses module hierarchy: crate::msg::module::Type
                        format!("crate::msg::{}::{}", module_name, name)
                    }
                } else {
                    // Cross-package reference
                    if rmw_layer {
                        // RMW layer uses FFI hierarchy: pkg::ffi::msg::module::Type
                        format!("{}::ffi::msg::{}::{}", pkg, module_name, name)
                    } else {
                        // Idiomatic layer uses module hierarchy: pkg::msg::module::Type
                        format!("{}::msg::{}::{}", pkg, module_name, name)
                    }
                }
            } else {
                // Local same-package type reference (no package specified)
                let module_name = to_snake_case(name);
                if rmw_layer {
                    // RMW layer uses FFI hierarchy
                    format!("crate::ffi::msg::{}::{}", module_name, name)
                } else {
                    // Idiomatic layer uses module hierarchy
                    format!("crate::msg::{}::{}", module_name, name)
                }
            }
        }
    }
}

/// Get the Rust type string for a constant
/// Similar to `rust_type_for_field` but uses `&'static str` for string types
/// since constants must be const-compatible
pub fn rust_type_for_constant(field_type: &FieldType) -> String {
    match field_type {
        FieldType::Primitive(prim) => prim.rust_type().to_string(),

        // All string types become &'static str for constants
        FieldType::String
        | FieldType::BoundedString(_)
        | FieldType::WString
        | FieldType::BoundedWString(_) => "&'static str".to_string(),

        // Arrays, sequences, and namespaced types are not typically used as constants
        // but we handle them for completeness
        FieldType::Array { element_type, size } => {
            let elem = rust_type_for_constant(element_type);
            format!("[{}; {}]", elem, size)
        }

        FieldType::Sequence { element_type } => {
            let elem = rust_type_for_constant(element_type);
            format!("&'static [{}]", elem)
        }

        FieldType::BoundedSequence {
            element_type,
            max_size: _,
        } => {
            let elem = rust_type_for_constant(element_type);
            format!("&'static [{}]", elem)
        }

        FieldType::NamespacedType { package, name } => {
            if let Some(pkg) = package {
                format!("{}::msg::{}", pkg, name)
            } else {
                format!("crate::msg::{}", name)
            }
        }
    }
}

/// Convert snake_case to UpperCamelCase
pub fn to_upper_camel_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect()
}

// Re-export to_snake_case from utils to ensure consistent behavior
pub use crate::utils::to_snake_case;

// ============================================================================
// IDL Type Support
// ============================================================================

/// Get the Rust type string for an IDL type
/// If `rmw_layer` is true, returns RMW types (rosidl_runtime_rs::*), else idiomatic types
/// `current_package` is used to detect self-references and use `crate::` instead of `pkg::`
pub fn rust_type_for_idl(
    idl_type: &IdlType,
    rmw_layer: bool,
    current_package: Option<&str>,
) -> String {
    match idl_type {
        IdlType::Primitive(prim) => prim.to_rust_type().to_string(),

        IdlType::String(None) => {
            if rmw_layer {
                "crate::rosidl_runtime_rs::String".to_string()
            } else {
                "std::string::String".to_string()
            }
        }

        IdlType::String(Some(bound)) => {
            if rmw_layer {
                format!("crate::rosidl_runtime_rs::BoundedString<{}>", bound)
            } else {
                "std::string::String".to_string()
            }
        }

        IdlType::WString(None) => {
            if rmw_layer {
                "crate::rosidl_runtime_rs::WString".to_string()
            } else {
                "std::string::String".to_string()
            }
        }

        IdlType::WString(Some(bound)) => {
            if rmw_layer {
                format!("crate::rosidl_runtime_rs::BoundedWString<{}>", bound)
            } else {
                "std::string::String".to_string()
            }
        }

        IdlType::Sequence(element_type, None) => {
            let elem = rust_type_for_idl(element_type, rmw_layer, current_package);
            if rmw_layer {
                format!("crate::rosidl_runtime_rs::Sequence<{}>", elem)
            } else {
                format!("std::vec::Vec<{}>", elem)
            }
        }

        IdlType::Sequence(element_type, Some(bound)) => {
            let elem = rust_type_for_idl(element_type, rmw_layer, current_package);
            if rmw_layer {
                format!(
                    "crate::rosidl_runtime_rs::BoundedSequence<{}, {}>",
                    elem, bound
                )
            } else {
                format!("std::vec::Vec<{}>", elem)
            }
        }

        IdlType::Array(element_type, dimensions) => {
            let elem = rust_type_for_idl(element_type, rmw_layer, current_package);
            let mut result = elem;
            for dim in dimensions.iter().rev() {
                result = format!("[{}; {}]", result, dim);
            }
            result
        }

        IdlType::UserDefined(name) => {
            // Local type reference (same package)
            let module_name = to_snake_case(name);
            if rmw_layer {
                format!("crate::ffi::msg::{}::{}", module_name, name)
            } else {
                format!("crate::msg::{}::{}", module_name, name)
            }
        }

        IdlType::Scoped(path) => {
            // Scoped name like package::msg::Type
            // Assume format: [package, interface_type, typename]
            if path.len() >= 3 {
                let package = &path[0];
                let typename = &path[path.len() - 1];
                let module_name = to_snake_case(typename);

                // Check if this is a self-reference
                let is_self_ref = package.as_str() == current_package.unwrap_or("");

                if is_self_ref {
                    if rmw_layer {
                        format!("crate::ffi::msg::{}::{}", module_name, typename)
                    } else {
                        format!("crate::msg::{}::{}", module_name, typename)
                    }
                } else if rmw_layer {
                    format!("{}::ffi::msg::{}::{}", package, module_name, typename)
                } else {
                    format!("{}::msg::{}::{}", package, module_name, typename)
                }
            } else if path.len() == 1 {
                // Simple name - treat as local type
                let module_name = to_snake_case(&path[0]);
                if rmw_layer {
                    format!("crate::ffi::msg::{}::{}", module_name, path[0])
                } else {
                    format!("crate::msg::{}::{}", module_name, path[0])
                }
            } else {
                // Fallback
                path.join("::")
            }
        }
    }
}

/// Get the Rust type string for an IDL constant
/// Similar to `rust_type_for_idl` but uses `&'static str` for string types
/// since constants must be const-compatible
pub fn rust_type_for_idl_constant(idl_type: &IdlType) -> String {
    match idl_type {
        IdlType::Primitive(prim) => prim.to_rust_type().to_string(),

        // All string types become &'static str for constants
        IdlType::String(_) | IdlType::WString(_) => "&'static str".to_string(),

        // Arrays and sequences
        IdlType::Array(element_type, dimensions) => {
            let elem = rust_type_for_idl_constant(element_type);
            let mut result = elem;
            for dim in dimensions.iter().rev() {
                result = format!("[{}; {}]", result, dim);
            }
            result
        }

        IdlType::Sequence(element_type, _) => {
            let elem = rust_type_for_idl_constant(element_type);
            format!("&'static [{}]", elem)
        }

        // User-defined types (enums, etc.)
        IdlType::UserDefined(name) => format!("crate::msg::{}", name),

        IdlType::Scoped(path) => {
            if path.len() >= 3 {
                let package = &path[0];
                let typename = &path[path.len() - 1];
                format!("{}::msg::{}", package, typename)
            } else if path.len() == 1 {
                format!("crate::msg::{}", path[0])
            } else {
                path.join("::")
            }
        }
    }
}

/// Convert an IDL constant value to Rust code string
pub fn idl_constant_value_to_rust(value: &IdlConstantValue) -> String {
    match value {
        IdlConstantValue::Integer(i) => i.to_string(),
        IdlConstantValue::Float(f) => {
            // Ensure float literals always have decimal point
            if f.is_finite() {
                if f.fract() == 0.0 {
                    format!("{:.1}", f) // Ensure .0 suffix
                } else {
                    f.to_string()
                }
            } else {
                f.to_string()
            }
        }
        IdlConstantValue::Boolean(b) => b.to_string(),
        IdlConstantValue::String(s) | IdlConstantValue::WString(s) => {
            format!("\"{}\"", s.escape_default())
        }
    }
}

/// Check if an IDL type is a wide string
pub fn is_idl_wide_string(idl_type: &IdlType) -> bool {
    matches!(idl_type, IdlType::WString(_))
}

/// Check if an IDL type is a sequence
pub fn is_idl_sequence(idl_type: &IdlType) -> bool {
    matches!(idl_type, IdlType::Sequence(_, _))
}

/// Check if an IDL type is an array
pub fn is_idl_array(idl_type: &IdlType) -> bool {
    matches!(idl_type, IdlType::Array(_, _))
}

/// Check if an IDL type is a primitive
pub fn is_idl_primitive(idl_type: &IdlType) -> bool {
    matches!(idl_type, IdlType::Primitive(_))
}

/// Check if an IDL type is a string type
pub fn is_idl_string(idl_type: &IdlType) -> bool {
    matches!(idl_type, IdlType::String(_) | IdlType::WString(_))
}
