use rosidl_codegen::{generate_message_package, GeneratorError};
use rosidl_parser::parse_message;
use std::collections::HashSet;

#[test]
fn test_lib_rs_has_clone_trait_bounds() -> Result<(), GeneratorError> {
    let msg_def = "int32 x\n";
    let msg = parse_message(msg_def).unwrap();
    let deps = HashSet::new();
    let result = generate_message_package("test_msgs", "Simple", &msg, &deps)?;

    // Verify Clone trait bounds are present in Message trait
    assert!(
        result.lib_rs.contains("Self: Sized + Clone"),
        "lib.rs should contain 'Self: Sized + Clone' trait bound"
    );
    assert!(
        result.lib_rs.contains("Self::RmwMsg: Clone"),
        "lib.rs should contain 'Self::RmwMsg: Clone' trait bound"
    );

    Ok(())
}

#[test]
fn test_idiomatic_uses_snake_case_modules() -> Result<(), GeneratorError> {
    let msg_def = "int32 x\n";
    let msg = parse_message(msg_def).unwrap();
    let deps = HashSet::new();
    let result = generate_message_package("test_msgs", "Duration", &msg, &deps)?;

    // Verify snake_case module paths are used (duration:: not Duration::)
    assert!(
        result
            .message_idiomatic
            .contains("crate::ffi::msg::duration::Duration"),
        "Idiomatic layer should use snake_case module paths"
    );
    assert!(
        !result
            .message_idiomatic
            .contains("crate::ffi::msg::Duration::"),
        "Idiomatic layer should not use PascalCase in module paths"
    );

    Ok(())
}
