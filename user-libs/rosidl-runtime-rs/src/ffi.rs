//! Raw FFI bindings to rosidl_runtime_c
//!
//! # Safety
//! All functions in this module are unsafe and require careful handling of:
//! - Pointer validity
//! - Memory ownership
//! - Proper initialization/finalization
//!
//! Most users should use the safe wrappers in `string` and `sequence` modules instead.

use std::os::raw::c_char;

/// C-compatible string structure (mirrors rosidl_runtime_c__String)
#[repr(C)]
#[derive(Debug)]
pub struct rosidl_runtime_c__String {
    pub data: *mut c_char,
    pub size: usize,
    pub capacity: usize,
}

/// C-compatible sequence structure
///
/// This is a generic container matching the layout of all rosidl_runtime_c sequences.
#[repr(C)]
#[derive(Debug)]
pub struct SequenceInner<T> {
    pub data: *mut T,
    pub size: usize,
    pub capacity: usize,
}

#[link(name = "rosidl_runtime_c")]
extern "C" {
    // =========================================================================
    // String operations
    // =========================================================================

    /// Initialize a rosidl_runtime_c__String structure
    pub fn rosidl_runtime_c__String__init(s: *mut rosidl_runtime_c__String) -> bool;

    /// Deallocate the memory of the rosidl_runtime_c__String structure
    pub fn rosidl_runtime_c__String__fini(s: *mut rosidl_runtime_c__String);

    /// Assign the c string pointer to the rosidl_runtime_c__String structure
    pub fn rosidl_runtime_c__String__assign(
        s: *mut rosidl_runtime_c__String,
        value: *const c_char,
    ) -> bool;

    /// Assign the c string pointer of n characters to the rosidl_runtime_c__String structure
    pub fn rosidl_runtime_c__String__assignn(
        s: *mut rosidl_runtime_c__String,
        value: *const c_char,
        n: usize,
    ) -> bool;

    /// Copy rosidl_runtime_c__String structure content
    pub fn rosidl_runtime_c__String__copy(
        input: *const rosidl_runtime_c__String,
        output: *mut rosidl_runtime_c__String,
    ) -> bool;

    /// Check for rosidl_runtime_c__String structure equality
    pub fn rosidl_runtime_c__String__are_equal(
        lhs: *const rosidl_runtime_c__String,
        rhs: *const rosidl_runtime_c__String,
    ) -> bool;

    // =========================================================================
    // String sequence operations
    // =========================================================================

    /// Initialize a rosidl_runtime_c__String__Sequence structure
    pub fn rosidl_runtime_c__String__Sequence__init(
        seq: *mut SequenceInner<rosidl_runtime_c__String>,
        size: usize,
    ) -> bool;

    /// Deallocate the memory of the string sequence structure
    pub fn rosidl_runtime_c__String__Sequence__fini(
        seq: *mut SequenceInner<rosidl_runtime_c__String>,
    );

    /// Copy rosidl_runtime_c__String__Sequence structure content
    pub fn rosidl_runtime_c__String__Sequence__copy(
        input: *const SequenceInner<rosidl_runtime_c__String>,
        output: *mut SequenceInner<rosidl_runtime_c__String>,
    ) -> bool;

    // =========================================================================
    // Primitive sequence operations
    // =========================================================================

    // float (f32)
    pub fn rosidl_runtime_c__float__Sequence__init(
        seq: *mut SequenceInner<f32>,
        size: usize,
    ) -> bool;
    pub fn rosidl_runtime_c__float__Sequence__fini(seq: *mut SequenceInner<f32>);
    pub fn rosidl_runtime_c__float__Sequence__copy(
        input: *const SequenceInner<f32>,
        output: *mut SequenceInner<f32>,
    ) -> bool;

    // double (f64)
    pub fn rosidl_runtime_c__double__Sequence__init(
        seq: *mut SequenceInner<f64>,
        size: usize,
    ) -> bool;
    pub fn rosidl_runtime_c__double__Sequence__fini(seq: *mut SequenceInner<f64>);
    pub fn rosidl_runtime_c__double__Sequence__copy(
        input: *const SequenceInner<f64>,
        output: *mut SequenceInner<f64>,
    ) -> bool;

    // int8 (i8)
    pub fn rosidl_runtime_c__int8__Sequence__init(seq: *mut SequenceInner<i8>, size: usize)
        -> bool;
    pub fn rosidl_runtime_c__int8__Sequence__fini(seq: *mut SequenceInner<i8>);
    pub fn rosidl_runtime_c__int8__Sequence__copy(
        input: *const SequenceInner<i8>,
        output: *mut SequenceInner<i8>,
    ) -> bool;

    // uint8 (u8)
    pub fn rosidl_runtime_c__uint8__Sequence__init(
        seq: *mut SequenceInner<u8>,
        size: usize,
    ) -> bool;
    pub fn rosidl_runtime_c__uint8__Sequence__fini(seq: *mut SequenceInner<u8>);
    pub fn rosidl_runtime_c__uint8__Sequence__copy(
        input: *const SequenceInner<u8>,
        output: *mut SequenceInner<u8>,
    ) -> bool;

    // int16 (i16)
    pub fn rosidl_runtime_c__int16__Sequence__init(
        seq: *mut SequenceInner<i16>,
        size: usize,
    ) -> bool;
    pub fn rosidl_runtime_c__int16__Sequence__fini(seq: *mut SequenceInner<i16>);
    pub fn rosidl_runtime_c__int16__Sequence__copy(
        input: *const SequenceInner<i16>,
        output: *mut SequenceInner<i16>,
    ) -> bool;

    // uint16 (u16)
    pub fn rosidl_runtime_c__uint16__Sequence__init(
        seq: *mut SequenceInner<u16>,
        size: usize,
    ) -> bool;
    pub fn rosidl_runtime_c__uint16__Sequence__fini(seq: *mut SequenceInner<u16>);
    pub fn rosidl_runtime_c__uint16__Sequence__copy(
        input: *const SequenceInner<u16>,
        output: *mut SequenceInner<u16>,
    ) -> bool;

    // int32 (i32)
    pub fn rosidl_runtime_c__int32__Sequence__init(
        seq: *mut SequenceInner<i32>,
        size: usize,
    ) -> bool;
    pub fn rosidl_runtime_c__int32__Sequence__fini(seq: *mut SequenceInner<i32>);
    pub fn rosidl_runtime_c__int32__Sequence__copy(
        input: *const SequenceInner<i32>,
        output: *mut SequenceInner<i32>,
    ) -> bool;

    // uint32 (u32)
    pub fn rosidl_runtime_c__uint32__Sequence__init(
        seq: *mut SequenceInner<u32>,
        size: usize,
    ) -> bool;
    pub fn rosidl_runtime_c__uint32__Sequence__fini(seq: *mut SequenceInner<u32>);
    pub fn rosidl_runtime_c__uint32__Sequence__copy(
        input: *const SequenceInner<u32>,
        output: *mut SequenceInner<u32>,
    ) -> bool;

    // int64 (i64)
    pub fn rosidl_runtime_c__int64__Sequence__init(
        seq: *mut SequenceInner<i64>,
        size: usize,
    ) -> bool;
    pub fn rosidl_runtime_c__int64__Sequence__fini(seq: *mut SequenceInner<i64>);
    pub fn rosidl_runtime_c__int64__Sequence__copy(
        input: *const SequenceInner<i64>,
        output: *mut SequenceInner<i64>,
    ) -> bool;

    // uint64 (u64)
    pub fn rosidl_runtime_c__uint64__Sequence__init(
        seq: *mut SequenceInner<u64>,
        size: usize,
    ) -> bool;
    pub fn rosidl_runtime_c__uint64__Sequence__fini(seq: *mut SequenceInner<u64>);
    pub fn rosidl_runtime_c__uint64__Sequence__copy(
        input: *const SequenceInner<u64>,
        output: *mut SequenceInner<u64>,
    ) -> bool;

    // boolean (bool)
    pub fn rosidl_runtime_c__boolean__Sequence__init(
        seq: *mut SequenceInner<bool>,
        size: usize,
    ) -> bool;
    pub fn rosidl_runtime_c__boolean__Sequence__fini(seq: *mut SequenceInner<bool>);
    pub fn rosidl_runtime_c__boolean__Sequence__copy(
        input: *const SequenceInner<bool>,
        output: *mut SequenceInner<bool>,
    ) -> bool;
}
