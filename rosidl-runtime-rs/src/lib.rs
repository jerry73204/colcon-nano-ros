//! Runtime support library for ROS 2 Rust bindings
//!
//! This crate provides the core runtime infrastructure for ROS 2 Rust bindings,
//! including:
//! - Type traits for message/service/action definitions
//! - Idiomatic Rust wrappers around ROS C types (String, Sequence)
//! - FFI bindings to rosidl_runtime_c
//!
//! # Architecture
//!
//! This crate provides two layers:
//! - **Idiomatic API**: Safe, ergonomic Rust types (`String`, `Sequence<T>`)
//! - **FFI layer**: Raw C bindings for per-package code generation
//!
//! Most users will use the idiomatic API. Generated package code may use both.

pub mod ffi;
pub mod sequence;
pub mod string;
pub mod traits;

// Re-export commonly used items
pub use sequence::Sequence;
pub use string::String;
pub use traits::{Action, Message, RmwMessage, SequenceAlloc, SequenceElement, Service};
