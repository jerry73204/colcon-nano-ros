//! C-callable static library for nros C code generation.
//!
//! Wraps `cargo_nano_ros::generate_c_from_args_file()` as a single C function
//! for use by the CMake build system.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;

/// Generate C bindings from a JSON arguments file.
///
/// # Arguments
/// * `args_file` - Null-terminated C string: path to the JSON arguments file
/// * `verbose` - Non-zero for verbose output
///
/// # Returns
/// 0 on success, 1 on error (details printed to stderr).
///
/// # Safety
/// `args_file` must be a valid, null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nros_codegen_generate_c(args_file: *const c_char, verbose: i32) -> i32 {
    let c_str = unsafe { CStr::from_ptr(args_file) };
    let path = match c_str.to_str() {
        Ok(s) => PathBuf::from(s),
        Err(e) => {
            eprintln!("nros-codegen: invalid UTF-8 in args_file path: {e}");
            return 1;
        }
    };

    let config = cargo_nano_ros::GenerateCConfig {
        args_file: path,
        verbose: verbose != 0,
    };

    match cargo_nano_ros::generate_c_from_args_file(config) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("nros-codegen: {e:#}");
            1
        }
    }
}
