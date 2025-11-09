use rosidl_parser::ast::ConstantValue;
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

/// Check if a field type is a sequence of primitives (can be copied directly)
pub fn is_primitive_sequence(field_type: &FieldType) -> bool {
    match field_type {
        FieldType::Sequence { element_type } | FieldType::BoundedSequence { element_type, .. } => {
            matches!(**element_type, FieldType::Primitive(_))
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

/// Convert a ConstantValue to a Rust code string
pub fn constant_value_to_rust(value: &ConstantValue) -> String {
    match value {
        ConstantValue::Integer(i) => i.to_string(),
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
                "rosidl_runtime_rs::String".to_string()
            } else {
                "std::string::String".to_string()
            }
        }

        FieldType::BoundedString(size) => {
            if rmw_layer {
                format!("rosidl_runtime_rs::BoundedString<{}>", size)
            } else {
                // Idiomatic layer uses String even for bounded
                "std::string::String".to_string()
            }
        }

        FieldType::WString => {
            if rmw_layer {
                "rosidl_runtime_rs::WString".to_string()
            } else {
                "std::string::String".to_string()
            }
        }

        FieldType::BoundedWString(size) => {
            if rmw_layer {
                format!("rosidl_runtime_rs::BoundedWString<{}>", size)
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
                format!("rosidl_runtime_rs::Sequence<{}>", elem)
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
                format!("rosidl_runtime_rs::BoundedSequence<{}, {}>", elem, max_size)
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
                        format!("crate::ffi::msg::{}::{}", to_snake_case(name), name)
                    } else {
                        format!("crate::msg::{}::{}", to_snake_case(name), name)
                    }
                } else {
                    // Cross-package reference
                    if rmw_layer {
                        format!("{}::ffi::msg::{}::{}", pkg, to_snake_case(name), name)
                    } else {
                        format!("{}::msg::{}::{}", pkg, to_snake_case(name), name)
                    }
                }
            } else {
                // Local same-package type reference (no package specified)
                if rmw_layer {
                    format!("crate::ffi::msg::{}::{}", to_snake_case(name), name)
                } else {
                    format!("crate::msg::{}::{}", to_snake_case(name), name)
                }
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

/// Convert UpperCamelCase to snake_case
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_is_uppercase = false;

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && !prev_is_uppercase {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
            prev_is_uppercase = true;
        } else {
            result.push(ch);
            prev_is_uppercase = false;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rosidl_parser::PrimitiveType;

    #[test]
    fn test_escape_keywords() {
        assert_eq!(escape_keyword("type"), "type_");
        assert_eq!(escape_keyword("match"), "match_");
        assert_eq!(escape_keyword("async"), "async_");
        assert_eq!(escape_keyword("normal_field"), "normal_field");
    }

    #[test]
    fn test_primitive_types() {
        let int32 = FieldType::Primitive(PrimitiveType::Int32);
        assert_eq!(rust_type_for_field(&int32, false, None), "i32");
        assert_eq!(rust_type_for_field(&int32, true, None), "i32");

        let float64 = FieldType::Primitive(PrimitiveType::Float64);
        assert_eq!(rust_type_for_field(&float64, false, None), "f64");
    }

    #[test]
    fn test_string_types() {
        let unbounded = FieldType::String;
        assert_eq!(
            rust_type_for_field(&unbounded, false, None),
            "std::string::String"
        );
        assert_eq!(
            rust_type_for_field(&unbounded, true, None),
            "rosidl_runtime_rs::String"
        );

        let bounded = FieldType::BoundedString(256);
        assert_eq!(
            rust_type_for_field(&bounded, false, None),
            "std::string::String"
        );
        assert_eq!(
            rust_type_for_field(&bounded, true, None),
            "rosidl_runtime_rs::BoundedString<256>"
        );
    }

    #[test]
    fn test_array_types() {
        let array = FieldType::Array {
            element_type: Box::new(FieldType::Primitive(PrimitiveType::Int32)),
            size: 5,
        };
        assert_eq!(rust_type_for_field(&array, false, None), "[i32; 5]");
        assert_eq!(rust_type_for_field(&array, true, None), "[i32; 5]");
    }

    #[test]
    fn test_sequence_types() {
        let seq = FieldType::Sequence {
            element_type: Box::new(FieldType::Primitive(PrimitiveType::Float64)),
        };
        assert_eq!(rust_type_for_field(&seq, false, None), "std::vec::Vec<f64>");
        assert_eq!(
            rust_type_for_field(&seq, true, None),
            "rosidl_runtime_rs::Sequence<f64>"
        );
    }

    #[test]
    fn test_case_conversion() {
        assert_eq!(to_upper_camel_case("test_message"), "TestMessage");
        assert_eq!(to_upper_camel_case("foo_bar_baz"), "FooBarBaz");

        assert_eq!(to_snake_case("TestMessage"), "test_message");
        assert_eq!(to_snake_case("FooBarBaz"), "foo_bar_baz");
    }
}
