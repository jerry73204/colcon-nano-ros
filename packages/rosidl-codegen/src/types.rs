use rosidl_parser::ast::ConstantValue;
use rosidl_parser::idl::ast::ConstantValue as IdlConstantValue;
use rosidl_parser::idl::types::IdlType;
use rosidl_parser::FieldType;

/// Code generation backend selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CodegenBackend {
    /// rclrs backend - generates two-layer types (RMW + idiomatic) with C FFI
    /// Requires rosidl_runtime_rs and links against ROS 2 C libraries
    #[default]
    Rclrs,

    /// nano-ros backend - generates single-layer pure Rust types
    /// Uses heapless collections for no_std compatibility
    /// No C dependencies, suitable for embedded RTOS platforms
    NanoRos,
}

/// Extension trait for FieldType providing type checking methods
pub trait FieldTypeExt {
    /// Check if this is a sequence (unbounded or bounded)
    fn is_sequence(&self) -> bool;

    /// Check if this is a primitive (no conversion needed)
    fn is_primitive(&self) -> bool;

    /// Check if this is a string type (String, BoundedString, WString, BoundedWString)
    fn is_string(&self) -> bool;

    /// Check if this is specifically an unbounded string
    fn is_unbounded_string(&self) -> bool;

    /// Check if this is specifically a bounded string
    fn is_bounded_string(&self) -> bool;

    /// Check if this is a WString type (unbounded or bounded)
    fn is_wstring(&self) -> bool;

    /// Check if this is specifically an unbounded WString
    fn is_unbounded_wstring(&self) -> bool;

    /// Check if this is specifically a bounded WString
    fn is_bounded_wstring(&self) -> bool;

    /// Check if this is a sequence of primitives (can be copied directly)
    fn is_primitive_sequence(&self) -> bool;

    /// Check if this is a sequence of strings (any string type)
    fn is_string_sequence(&self) -> bool;

    /// Check if this is a sequence of unbounded strings
    fn is_unbounded_string_sequence(&self) -> bool;

    /// Check if this is a sequence of bounded strings
    fn is_bounded_string_sequence(&self) -> bool;

    /// Check if this is an array (needs clone, not conversion)
    fn is_array(&self) -> bool;

    /// Check if this is a large array (> 32 elements, needs big_array for serde)
    fn is_large_array(&self) -> bool;

    /// Check if this is an array of primitives (can be cloned directly)
    fn is_primitive_array(&self) -> bool;

    /// Check if this is an array of strings (any string type)
    fn is_string_array(&self) -> bool;

    /// Check if this is an array of unbounded strings
    fn is_unbounded_string_array(&self) -> bool;

    /// Check if this is an array of bounded strings
    fn is_bounded_string_array(&self) -> bool;

    /// Check if this is an array of unbounded wstrings
    fn is_unbounded_wstring_array(&self) -> bool;

    /// Check if this is an array of bounded wstrings
    fn is_bounded_wstring_array(&self) -> bool;

    /// Check if this is a sequence of unbounded wstrings
    fn is_unbounded_wstring_sequence(&self) -> bool;

    /// Check if this is a sequence of bounded wstrings
    fn is_bounded_wstring_sequence(&self) -> bool;

    /// Check if this is an array of nested messages (needs element conversion)
    fn is_nested_array(&self) -> bool;

    /// Check if this is a bounded sequence (vs unbounded)
    fn is_bounded_sequence(&self) -> bool;
}

impl FieldTypeExt for FieldType {
    fn is_sequence(&self) -> bool {
        matches!(
            self,
            FieldType::Sequence { .. } | FieldType::BoundedSequence { .. }
        )
    }

    fn is_primitive(&self) -> bool {
        matches!(self, FieldType::Primitive(_))
    }

    fn is_string(&self) -> bool {
        matches!(
            self,
            FieldType::String
                | FieldType::BoundedString(_)
                | FieldType::WString
                | FieldType::BoundedWString(_)
        )
    }

    fn is_unbounded_string(&self) -> bool {
        matches!(self, FieldType::String)
    }

    fn is_bounded_string(&self) -> bool {
        matches!(self, FieldType::BoundedString(_))
    }

    fn is_wstring(&self) -> bool {
        matches!(self, FieldType::WString | FieldType::BoundedWString(_))
    }

    fn is_unbounded_wstring(&self) -> bool {
        matches!(self, FieldType::WString)
    }

    fn is_bounded_wstring(&self) -> bool {
        matches!(self, FieldType::BoundedWString(_))
    }

    fn is_primitive_sequence(&self) -> bool {
        match self {
            FieldType::Sequence { element_type }
            | FieldType::BoundedSequence { element_type, .. } => {
                matches!(**element_type, FieldType::Primitive(_))
            }
            _ => false,
        }
    }

    fn is_string_sequence(&self) -> bool {
        match self {
            FieldType::Sequence { element_type }
            | FieldType::BoundedSequence { element_type, .. } => element_type.is_string(),
            _ => false,
        }
    }

    fn is_unbounded_string_sequence(&self) -> bool {
        match self {
            FieldType::Sequence { element_type }
            | FieldType::BoundedSequence { element_type, .. } => element_type.is_unbounded_string(),
            _ => false,
        }
    }

    fn is_bounded_string_sequence(&self) -> bool {
        match self {
            FieldType::Sequence { element_type }
            | FieldType::BoundedSequence { element_type, .. } => element_type.is_bounded_string(),
            _ => false,
        }
    }

    fn is_array(&self) -> bool {
        matches!(self, FieldType::Array { .. })
    }

    fn is_large_array(&self) -> bool {
        matches!(self, FieldType::Array { size, .. } if *size > 32)
    }

    fn is_primitive_array(&self) -> bool {
        match self {
            FieldType::Array { element_type, .. } => {
                matches!(**element_type, FieldType::Primitive(_))
            }
            _ => false,
        }
    }

    fn is_string_array(&self) -> bool {
        match self {
            FieldType::Array { element_type, .. } => element_type.is_string(),
            _ => false,
        }
    }

    fn is_unbounded_string_array(&self) -> bool {
        match self {
            FieldType::Array { element_type, .. } => element_type.is_unbounded_string(),
            _ => false,
        }
    }

    fn is_bounded_string_array(&self) -> bool {
        match self {
            FieldType::Array { element_type, .. } => element_type.is_bounded_string(),
            _ => false,
        }
    }

    fn is_unbounded_wstring_array(&self) -> bool {
        match self {
            FieldType::Array { element_type, .. } => matches!(**element_type, FieldType::WString),
            _ => false,
        }
    }

    fn is_bounded_wstring_array(&self) -> bool {
        match self {
            FieldType::Array { element_type, .. } => {
                matches!(**element_type, FieldType::BoundedWString(_))
            }
            _ => false,
        }
    }

    fn is_unbounded_wstring_sequence(&self) -> bool {
        match self {
            FieldType::Sequence { element_type }
            | FieldType::BoundedSequence { element_type, .. } => {
                matches!(**element_type, FieldType::WString)
            }
            _ => false,
        }
    }

    fn is_bounded_wstring_sequence(&self) -> bool {
        match self {
            FieldType::Sequence { element_type }
            | FieldType::BoundedSequence { element_type, .. } => {
                matches!(**element_type, FieldType::BoundedWString(_))
            }
            _ => false,
        }
    }

    fn is_nested_array(&self) -> bool {
        match self {
            FieldType::Array { element_type, .. } => {
                matches!(**element_type, FieldType::NamespacedType { .. })
            }
            _ => false,
        }
    }

    fn is_bounded_sequence(&self) -> bool {
        matches!(self, FieldType::BoundedSequence { .. })
    }
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
                if is_self_ref {
                    // Self-reference: use crate:: instead of pkg::
                    if rmw_layer {
                        // RMW layer uses rmw submodule: crate::msg::rmw::Type
                        format!("crate::msg::rmw::{}", name)
                    } else {
                        // Idiomatic layer: types are re-exported from msg module
                        format!("crate::msg::{}", name)
                    }
                } else {
                    // Cross-package reference
                    if rmw_layer {
                        // RMW layer uses rmw submodule: pkg::msg::rmw::Type
                        format!("{}::msg::rmw::{}", pkg, name)
                    } else {
                        // Idiomatic layer: types are re-exported from msg module
                        format!("{}::msg::{}", pkg, name)
                    }
                }
            } else {
                // Local same-package type reference (no package specified)
                if rmw_layer {
                    // RMW layer uses rmw submodule
                    format!("crate::msg::rmw::{}", name)
                } else {
                    // Idiomatic layer: types are re-exported from msg module
                    format!("crate::msg::{}", name)
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

// ============================================================================
// nano-ros Type Mapping
// ============================================================================

/// Default string capacity for nano-ros heapless strings
pub const NANO_ROS_DEFAULT_STRING_CAPACITY: usize = 256;

/// Default sequence capacity for nano-ros heapless vectors
pub const NANO_ROS_DEFAULT_SEQUENCE_CAPACITY: usize = 64;

/// Get the Rust type string for a field type using nano-ros backend
/// Returns heapless types for no_std compatibility
/// `current_package` is used to detect self-references and use `crate::` instead of `pkg::`
pub fn nano_ros_type_for_field(field_type: &FieldType, current_package: Option<&str>) -> String {
    match field_type {
        FieldType::Primitive(prim) => prim.rust_type().to_string(),

        FieldType::String => {
            format!("heapless::String<{}>", NANO_ROS_DEFAULT_STRING_CAPACITY)
        }

        FieldType::BoundedString(size) => {
            format!("heapless::String<{}>", size)
        }

        FieldType::WString => {
            // WString maps to regular heapless::String (UTF-8)
            format!("heapless::String<{}>", NANO_ROS_DEFAULT_STRING_CAPACITY)
        }

        FieldType::BoundedWString(size) => {
            format!("heapless::String<{}>", size)
        }

        FieldType::Array { element_type, size } => {
            let elem = nano_ros_type_for_field(element_type, current_package);
            format!("[{}; {}]", elem, size)
        }

        FieldType::Sequence { element_type } => {
            let elem = nano_ros_type_for_field(element_type, current_package);
            format!(
                "heapless::Vec<{}, {}>",
                elem, NANO_ROS_DEFAULT_SEQUENCE_CAPACITY
            )
        }

        FieldType::BoundedSequence {
            element_type,
            max_size,
        } => {
            let elem = nano_ros_type_for_field(element_type, current_package);
            format!("heapless::Vec<{}, {}>", elem, max_size)
        }

        FieldType::NamespacedType { package, name } => {
            // Check if this is a self-reference (same package referencing itself)
            let is_self_ref = package.as_deref() == current_package;

            if let Some(pkg) = package {
                if is_self_ref {
                    // Self-reference: use crate::
                    format!("crate::msg::{}", name)
                } else {
                    // Cross-package reference
                    format!("{}::msg::{}", pkg, name)
                }
            } else {
                // Local same-package type reference (no package specified)
                format!("crate::msg::{}", name)
            }
        }
    }
}

/// Get the Rust type string for a constant using nano-ros backend
/// Similar to `nano_ros_type_for_field` but uses `&'static str` for string types
pub fn nano_ros_type_for_constant(field_type: &FieldType) -> String {
    match field_type {
        FieldType::Primitive(prim) => prim.rust_type().to_string(),

        // All string types become &'static str for constants
        FieldType::String
        | FieldType::BoundedString(_)
        | FieldType::WString
        | FieldType::BoundedWString(_) => "&'static str".to_string(),

        FieldType::Array { element_type, size } => {
            let elem = nano_ros_type_for_constant(element_type);
            format!("[{}; {}]", elem, size)
        }

        FieldType::Sequence { element_type } | FieldType::BoundedSequence { element_type, .. } => {
            let elem = nano_ros_type_for_constant(element_type);
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
            if rmw_layer {
                format!("crate::msg::rmw::{}", name)
            } else {
                // Idiomatic layer: types are re-exported from msg module
                format!("crate::msg::{}", name)
            }
        }

        IdlType::Scoped(path) => {
            // Scoped name like package::msg::Type
            // Assume format: [package, interface_type, typename]
            if path.len() >= 3 {
                let package = &path[0];
                let typename = &path[path.len() - 1];

                // Check if this is a self-reference
                let is_self_ref = package.as_str() == current_package.unwrap_or("");

                if is_self_ref {
                    if rmw_layer {
                        format!("crate::msg::rmw::{}", typename)
                    } else {
                        // Idiomatic layer: types are re-exported from msg module
                        format!("crate::msg::{}", typename)
                    }
                } else if rmw_layer {
                    format!("{}::msg::rmw::{}", package, typename)
                } else {
                    // Idiomatic layer: types are re-exported from msg module
                    format!("{}::msg::{}", package, typename)
                }
            } else if path.len() == 1 {
                // Simple name - treat as local type
                if rmw_layer {
                    format!("crate::msg::rmw::{}", path[0])
                } else {
                    // Idiomatic layer: types are re-exported from msg module
                    format!("crate::msg::{}", path[0])
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

/// Extension trait for IdlType providing type checking methods
pub trait IdlTypeExt {
    /// Check if this is a wide string
    fn is_wide_string(&self) -> bool;

    /// Check if this is a sequence
    fn is_sequence(&self) -> bool;

    /// Check if this is an array
    fn is_array(&self) -> bool;

    /// Check if this is a primitive
    fn is_primitive(&self) -> bool;

    /// Check if this is a string type
    fn is_string(&self) -> bool;
}

impl IdlTypeExt for IdlType {
    fn is_wide_string(&self) -> bool {
        matches!(self, IdlType::WString(_))
    }

    fn is_sequence(&self) -> bool {
        matches!(self, IdlType::Sequence(_, _))
    }

    fn is_array(&self) -> bool {
        matches!(self, IdlType::Array(_, _))
    }

    fn is_primitive(&self) -> bool {
        matches!(self, IdlType::Primitive(_))
    }

    fn is_string(&self) -> bool {
        matches!(self, IdlType::String(_) | IdlType::WString(_))
    }
}

/// Convert IDL primitive to .msg primitive type
pub fn idl_primitive_to_primitive(
    idl_prim: &rosidl_parser::idl::types::IdlPrimitiveType,
) -> rosidl_parser::PrimitiveType {
    use rosidl_parser::idl::types::IdlPrimitiveType;
    use rosidl_parser::PrimitiveType;

    match idl_prim {
        IdlPrimitiveType::Short => PrimitiveType::Int16,
        IdlPrimitiveType::UnsignedShort => PrimitiveType::UInt16,
        IdlPrimitiveType::Long => PrimitiveType::Int32,
        IdlPrimitiveType::UnsignedLong => PrimitiveType::UInt32,
        IdlPrimitiveType::LongLong => PrimitiveType::Int64,
        IdlPrimitiveType::UnsignedLongLong => PrimitiveType::UInt64,
        IdlPrimitiveType::Float => PrimitiveType::Float32,
        IdlPrimitiveType::Double => PrimitiveType::Float64,
        IdlPrimitiveType::LongDouble => PrimitiveType::Float64, // Map to f64
        IdlPrimitiveType::Char => PrimitiveType::Char,
        IdlPrimitiveType::Wchar => PrimitiveType::UInt16, // Wchar is 16-bit
        IdlPrimitiveType::Boolean => PrimitiveType::Bool,
        IdlPrimitiveType::Octet => PrimitiveType::Byte,
        IdlPrimitiveType::Int8 => PrimitiveType::Int8,
        IdlPrimitiveType::Uint8 => PrimitiveType::UInt8,
        IdlPrimitiveType::Int16 => PrimitiveType::Int16,
        IdlPrimitiveType::Uint16 => PrimitiveType::UInt16,
        IdlPrimitiveType::Int32 => PrimitiveType::Int32,
        IdlPrimitiveType::Uint32 => PrimitiveType::UInt32,
        IdlPrimitiveType::Int64 => PrimitiveType::Int64,
        IdlPrimitiveType::Uint64 => PrimitiveType::UInt64,
    }
}

/// Convert IDL annotation value to .msg constant value
pub fn annotation_value_to_constant_value(
    ann_val: &rosidl_parser::idl::ast::AnnotationValue,
) -> rosidl_parser::ast::ConstantValue {
    use rosidl_parser::ast::ConstantValue;
    use rosidl_parser::idl::ast::AnnotationValue;

    match ann_val {
        AnnotationValue::Integer(i) => ConstantValue::Integer(*i),
        AnnotationValue::Float(f) => ConstantValue::Float(*f),
        AnnotationValue::String(s) => ConstantValue::String(s.clone()),
        AnnotationValue::Boolean(b) => ConstantValue::Bool(*b),
        AnnotationValue::Identifier(id) => {
            // For identifiers, we need to check if they're boolean keywords
            match id.as_str() {
                "TRUE" | "True" | "true" => ConstantValue::Bool(true),
                "FALSE" | "False" | "false" => ConstantValue::Bool(false),
                // For other identifiers, treat as string
                _ => ConstantValue::String(id.clone()),
            }
        }
    }
}

// ============================================================================
// C Type Mapping (for nano-ros-c)
// ============================================================================

/// Default string capacity for C strings (matches nano-ros-c)
pub const C_DEFAULT_STRING_CAPACITY: usize = 256;

/// Default sequence capacity for C arrays
pub const C_DEFAULT_SEQUENCE_CAPACITY: usize = 64;

/// Get the C base type string for a field type (without array suffix)
/// For array declarations in C, the array suffix comes after the variable name.
/// Use `c_array_suffix_for_field` to get the suffix.
pub fn c_type_for_field(field_type: &FieldType, _current_package: Option<&str>) -> String {
    match field_type {
        FieldType::Primitive(prim) => c_primitive_type(prim),

        // Strings use char as base type, with array suffix for the size
        FieldType::String | FieldType::BoundedString(_) => "char".to_string(),

        FieldType::WString | FieldType::BoundedWString(_) => "char".to_string(),

        // Arrays use the element type as base type, with array suffix for the size
        FieldType::Array { element_type, .. } => c_type_for_field(element_type, None),

        // Sequences use anonymous struct
        FieldType::Sequence { element_type } => {
            let elem = c_type_for_field(element_type, None);
            let elem_suffix = c_array_suffix_for_field(element_type);
            format!(
                "struct {{ uint32_t size; {} data{}[{}]; }}",
                elem, elem_suffix, C_DEFAULT_SEQUENCE_CAPACITY
            )
        }

        FieldType::BoundedSequence {
            element_type,
            max_size,
        } => {
            let elem = c_type_for_field(element_type, None);
            let elem_suffix = c_array_suffix_for_field(element_type);
            format!(
                "struct {{ uint32_t size; {} data{}[{}]; }}",
                elem, elem_suffix, max_size
            )
        }

        FieldType::NamespacedType { package, name } => {
            if let Some(pkg) = package {
                format!(
                    "struct {}_msg_{}",
                    to_c_package_name(pkg),
                    to_snake_case(name)
                )
            } else {
                format!("struct msg_{}", to_snake_case(name))
            }
        }
    }
}

/// Get the C array suffix for a field type (e.g., "[256]" for strings, "[3]" for arrays)
/// This comes after the field name in C declarations: `char name[256];`
pub fn c_array_suffix_for_field(field_type: &FieldType) -> String {
    match field_type {
        FieldType::String => format!("[{}]", C_DEFAULT_STRING_CAPACITY),
        FieldType::BoundedString(size) => format!("[{}]", size),
        FieldType::WString => format!("[{}]", C_DEFAULT_STRING_CAPACITY),
        FieldType::BoundedWString(size) => format!("[{}]", size),
        FieldType::Array { element_type, size } => {
            // For nested arrays (rare), we need to combine suffixes
            let inner_suffix = c_array_suffix_for_field(element_type);
            format!("[{}]{}", size, inner_suffix)
        }
        // Sequences and other types don't have array suffixes (they're inline structs or scalars)
        _ => String::new(),
    }
}

/// Convert a primitive type to its C equivalent
fn c_primitive_type(prim: &rosidl_parser::PrimitiveType) -> String {
    use rosidl_parser::PrimitiveType;
    match prim {
        PrimitiveType::Bool => "bool".to_string(),
        PrimitiveType::Byte => "uint8_t".to_string(),
        PrimitiveType::Char => "char".to_string(),
        PrimitiveType::Int8 => "int8_t".to_string(),
        PrimitiveType::Int16 => "int16_t".to_string(),
        PrimitiveType::Int32 => "int32_t".to_string(),
        PrimitiveType::Int64 => "int64_t".to_string(),
        PrimitiveType::UInt8 => "uint8_t".to_string(),
        PrimitiveType::UInt16 => "uint16_t".to_string(),
        PrimitiveType::UInt32 => "uint32_t".to_string(),
        PrimitiveType::UInt64 => "uint64_t".to_string(),
        PrimitiveType::Float32 => "float".to_string(),
        PrimitiveType::Float64 => "double".to_string(),
    }
}

/// Convert package name to C-compatible identifier (replace - with _)
pub fn to_c_package_name(name: &str) -> String {
    name.replace('-', "_")
}

/// Get the C type string for a constant
pub fn c_type_for_constant(field_type: &FieldType) -> String {
    match field_type {
        FieldType::Primitive(prim) => c_primitive_type(prim),
        // String constants are const char*
        FieldType::String
        | FieldType::BoundedString(_)
        | FieldType::WString
        | FieldType::BoundedWString(_) => "const char*".to_string(),
        // Arrays and sequences shouldn't normally be constants
        _ => "void*".to_string(),
    }
}

/// Get the CDR write method name for a C primitive type
pub fn c_cdr_write_method(prim: &rosidl_parser::PrimitiveType) -> &'static str {
    use rosidl_parser::PrimitiveType;
    match prim {
        PrimitiveType::Bool => "write_bool",
        PrimitiveType::Byte => "write_u8",
        PrimitiveType::Char => "write_u8",
        PrimitiveType::Int8 => "write_i8",
        PrimitiveType::Int16 => "write_i16",
        PrimitiveType::Int32 => "write_i32",
        PrimitiveType::Int64 => "write_i64",
        PrimitiveType::UInt8 => "write_u8",
        PrimitiveType::UInt16 => "write_u16",
        PrimitiveType::UInt32 => "write_u32",
        PrimitiveType::UInt64 => "write_u64",
        PrimitiveType::Float32 => "write_f32",
        PrimitiveType::Float64 => "write_f64",
    }
}

/// Get the CDR read method name for a C primitive type
pub fn c_cdr_read_method(prim: &rosidl_parser::PrimitiveType) -> &'static str {
    use rosidl_parser::PrimitiveType;
    match prim {
        PrimitiveType::Bool => "read_bool",
        PrimitiveType::Byte => "read_u8",
        PrimitiveType::Char => "read_u8",
        PrimitiveType::Int8 => "read_i8",
        PrimitiveType::Int16 => "read_i16",
        PrimitiveType::Int32 => "read_i32",
        PrimitiveType::Int64 => "read_i64",
        PrimitiveType::UInt8 => "read_u8",
        PrimitiveType::UInt16 => "read_u16",
        PrimitiveType::UInt32 => "read_u32",
        PrimitiveType::UInt64 => "read_u64",
        PrimitiveType::Float32 => "read_f32",
        PrimitiveType::Float64 => "read_f64",
    }
}
