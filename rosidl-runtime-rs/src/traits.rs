//! Core traits for ROS 2 type system
//!
//! These traits establish the relationships between:
//! - Idiomatic Rust types and RMW (C FFI) types
//! - Messages, services, and actions
//! - Type support metadata

/// Establishes type relationship between idiomatic and RMW types
///
/// This trait allows sequences to convert between idiomatic Rust types
/// and their corresponding C FFI (RMW) representations.
///
/// # Example
/// ```ignore
/// impl SequenceElement for Point {
///     type RmwType = ffi::Point;
/// }
/// ```
pub trait SequenceElement: Sized {
    /// The RMW (C FFI) type corresponding to this idiomatic type
    type RmwType;
}

/// Sequence allocation trait for RMW types
///
/// This trait provides sequence operations for RMW message types.
/// Unlike primitives which use generic rosidl_runtime_c functions,
/// each message type has its own sequence init/fini/copy functions.
pub trait SequenceAlloc {
    /// Initialize a sequence with the specified size
    fn sequence_init(seq: &mut crate::sequence::Sequence<Self>, size: usize) -> bool
    where
        Self: Sized;

    /// Finalize a sequence, freeing its memory
    fn sequence_fini(seq: &mut crate::sequence::Sequence<Self>)
    where
        Self: Sized;

    /// Copy sequence content
    fn sequence_copy(
        in_seq: &crate::sequence::Sequence<Self>,
        out_seq: &mut crate::sequence::Sequence<Self>,
    ) -> bool
    where
        Self: Sized;
}

/// Conversion between idiomatic and RMW message representations
///
/// Messages implement this trait to enable conversion between the user-friendly
/// idiomatic Rust API and the C FFI layer required for ROS communication.
pub trait Message {
    /// The RMW message type (C FFI representation)
    type RmwMsg;

    /// Convert from idiomatic to RMW format
    fn into_rmw_message(msg_cow: std::borrow::Cow<'_, Self>) -> std::borrow::Cow<'_, Self::RmwMsg>
    where
        Self: Sized + Clone,
        Self::RmwMsg: Clone;

    /// Convert from RMW to idiomatic format
    fn from_rmw_message(msg: Self::RmwMsg) -> Self
    where
        Self: Sized;
}

/// RMW message with type support information
///
/// This trait provides access to the message's type support handle,
/// which is required for ROS communication.
pub trait RmwMessage: Sized {
    /// The fully-qualified message type name (e.g., "geometry_msgs/msg/Point")
    const TYPE_NAME: &'static str;

    /// Get the type support handle for this message
    fn get_type_support() -> *const std::ffi::c_void;
}

/// Service definition with request/response types
///
/// Services consist of a request message and a response message.
pub trait Service {
    /// The request message type
    type Request;
    /// The response message type
    type Response;

    /// Get the type support handle for this service
    fn get_type_support() -> *const std::ffi::c_void;
}

/// Action definition with goal/result/feedback types
///
/// Actions consist of three message types for asynchronous operations.
pub trait Action {
    /// The goal message type
    type Goal;
    /// The result message type
    type Result;
    /// The feedback message type
    type Feedback;
}

// Implement SequenceElement for std::string::String (maps to rosidl_runtime_rs::String)
impl SequenceElement for std::string::String {
    type RmwType = crate::string::String;
}
