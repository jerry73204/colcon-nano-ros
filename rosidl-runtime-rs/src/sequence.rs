//! Idiomatic Rust wrapper for ROS 2 sequences
//!
//! Provides a safe, user-friendly API around C sequence types.

use crate::ffi;
use crate::traits::SequenceElement;
use std::fmt;
use std::marker::PhantomData;

/// ROS 2 sequence with automatic memory management
///
/// Generic container for arrays of elements, with automatic C memory management.
/// For primitive types (f32, f64, i8-i64, u8-u64, bool), this uses the
/// rosidl_runtime_c sequence functions. For message types, generated code
/// provides the sequence operations.
///
/// # Example
/// ```ignore
/// use rosidl_runtime_rs::Sequence;
///
/// // Primitive sequence
/// let vec = vec![1.0, 2.0, 3.0, 4.0];
/// let seq: Sequence<f64> = vec.into();
///
/// for value in seq.as_slice() {
///     println!("Value: {}", value);
/// }
///
/// let back_to_vec: Vec<f64> = seq.into();
/// ```
pub struct Sequence<T> {
    inner: ffi::SequenceInner<T>,
    _marker: PhantomData<T>,
}

// Manual Clone implementation (can't derive due to PhantomData)
impl<T: Clone> Clone for Sequence<T> {
    fn clone(&self) -> Self {
        // For now, just create empty sequence - proper cloning requires type-specific functions
        Self {
            inner: ffi::SequenceInner {
                data: std::ptr::null_mut(),
                size: 0,
                capacity: 0,
            },
            _marker: PhantomData,
        }
    }
}

// Manual PartialEq implementation
impl<T: PartialEq> PartialEq for Sequence<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T> Sequence<T> {
    /// Get the number of elements
    pub fn len(&self) -> usize {
        self.inner.size
    }

    /// Check if sequence is empty
    pub fn is_empty(&self) -> bool {
        self.inner.size == 0
    }

    /// Get capacity (allocated elements)
    pub fn capacity(&self) -> usize {
        self.inner.capacity
    }

    /// Get immutable slice view of the sequence
    pub fn as_slice(&self) -> &[T] {
        if self.inner.data.is_null() || self.inner.size == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.inner.data, self.inner.size) }
        }
    }

    /// Get mutable slice view of the sequence
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        if self.inner.data.is_null() || self.inner.size == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(self.inner.data, self.inner.size) }
        }
    }

    /// Get mutable access to the underlying FFI type
    ///
    /// # Safety
    /// Caller must ensure the FFI type remains valid and properly initialized
    pub unsafe fn as_mut_ffi(&mut self) -> &mut ffi::SequenceInner<T> {
        &mut self.inner
    }

    /// Get immutable access to the underlying FFI type
    pub fn as_ffi(&self) -> &ffi::SequenceInner<T> {
        &self.inner
    }

    /// Convert to Vec with element conversion
    ///
    /// Used for sequences of message types that need RMW → idiomatic conversion
    pub fn to_vec_converted<U>(&self) -> Vec<U>
    where
        U: SequenceElement<RmwType = T>,
        for<'a> &'a T: Into<U>,
    {
        self.as_slice().iter().map(|elem| elem.into()).collect()
    }

    /// Create from slice with element conversion
    ///
    /// Used for sequences of message types that need idiomatic → RMW conversion
    pub fn from_slice_converted<U>(_slice: &[U]) -> Self
    where
        U: SequenceElement<RmwType = T>,
        for<'a> &'a U: Into<T>,
    {
        // Stub implementation - proper conversion requires type-specific init functions
        Self {
            inner: ffi::SequenceInner {
                data: std::ptr::null_mut(),
                size: 0,
                capacity: 0,
            },
            _marker: PhantomData,
        }
    }
}

// Primitive sequence operations (uses rosidl_runtime_c)
impl<T: PrimitiveSequence> Sequence<T> {
    /// Create a new sequence with the specified capacity
    pub fn new(size: usize) -> Result<Self, SequenceError> {
        let mut inner = ffi::SequenceInner {
            data: std::ptr::null_mut(),
            size: 0,
            capacity: 0,
        };

        unsafe {
            if T::sequence_init(&mut inner, size) {
                Ok(Self {
                    inner,
                    _marker: PhantomData,
                })
            } else {
                Err(SequenceError::InitializationFailed)
            }
        }
    }

    /// Manually drop the sequence (call this before the sequence goes out of scope)
    pub fn fini(&mut self) {
        unsafe {
            T::sequence_fini(&mut self.inner);
        }
    }

    /// Clone the sequence
    pub fn clone_seq(&self) -> Result<Self, SequenceError> {
        let mut new_seq = Sequence::new(self.len())?;
        unsafe {
            if !T::sequence_copy(&self.inner, &mut new_seq.inner) {
                return Err(SequenceError::AllocationFailed);
            }
        }
        Ok(new_seq)
    }
}

// Conversion from Vec for primitive types
impl<T: PrimitiveSequence + Clone> From<Vec<T>> for Sequence<T> {
    fn from(vec: Vec<T>) -> Self {
        let mut seq = Sequence::new(vec.len()).expect("Failed to allocate sequence");
        seq.as_mut_slice().clone_from_slice(&vec);
        seq
    }
}

// Conversion to Vec for primitive types
impl<T: PrimitiveSequence + Clone> From<Sequence<T>> for Vec<T> {
    fn from(seq: Sequence<T>) -> Self {
        seq.as_slice().to_vec()
    }
}

impl<T: fmt::Debug> fmt::Debug for Sequence<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sequence")
            .field("size", &self.inner.size)
            .field("capacity", &self.inner.capacity)
            .field("data", &self.as_slice())
            .finish()
    }
}

/// Errors that can occur during sequence operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceError {
    /// Sequence initialization failed
    InitializationFailed,
    /// Memory allocation failed
    AllocationFailed,
}

impl fmt::Display for SequenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SequenceError::InitializationFailed => write!(f, "Sequence initialization failed"),
            SequenceError::AllocationFailed => write!(f, "Memory allocation failed"),
        }
    }
}

impl std::error::Error for SequenceError {}

/// Marker trait for primitive types that can use rosidl_runtime_c sequence functions
///
/// This trait is automatically implemented for all primitive types (f32, f64,
/// i8-i64, u8-u64, bool) that have corresponding rosidl_runtime_c sequence operations.
pub trait PrimitiveSequence: Sized {
    /// Initialize a sequence with the specified size
    ///
    /// # Safety
    /// Caller must ensure the sequence pointer is valid
    unsafe fn sequence_init(seq: &mut ffi::SequenceInner<Self>, size: usize) -> bool;

    /// Finalize a sequence, freeing its memory
    ///
    /// # Safety
    /// Caller must ensure the sequence pointer is valid
    unsafe fn sequence_fini(seq: &mut ffi::SequenceInner<Self>);

    /// Copy sequence content
    ///
    /// # Safety
    /// Caller must ensure both sequence pointers are valid
    unsafe fn sequence_copy(
        input: &ffi::SequenceInner<Self>,
        output: &mut ffi::SequenceInner<Self>,
    ) -> bool;
}

// Macro to implement PrimitiveSequence for all primitive types
macro_rules! impl_primitive_sequence {
    ($rust_type:ty, $c_init:ident, $c_fini:ident, $c_copy:ident) => {
        impl PrimitiveSequence for $rust_type {
            unsafe fn sequence_init(seq: &mut ffi::SequenceInner<Self>, size: usize) -> bool {
                ffi::$c_init(seq as *mut _, size)
            }

            unsafe fn sequence_fini(seq: &mut ffi::SequenceInner<Self>) {
                ffi::$c_fini(seq as *mut _)
            }

            unsafe fn sequence_copy(
                input: &ffi::SequenceInner<Self>,
                output: &mut ffi::SequenceInner<Self>,
            ) -> bool {
                ffi::$c_copy(input as *const _, output as *mut _)
            }
        }
    };
}

// Implement PrimitiveSequence for all supported types
impl_primitive_sequence!(
    f64,
    rosidl_runtime_c__double__Sequence__init,
    rosidl_runtime_c__double__Sequence__fini,
    rosidl_runtime_c__double__Sequence__copy
);

impl_primitive_sequence!(
    f32,
    rosidl_runtime_c__float__Sequence__init,
    rosidl_runtime_c__float__Sequence__fini,
    rosidl_runtime_c__float__Sequence__copy
);

impl_primitive_sequence!(
    i8,
    rosidl_runtime_c__int8__Sequence__init,
    rosidl_runtime_c__int8__Sequence__fini,
    rosidl_runtime_c__int8__Sequence__copy
);

impl_primitive_sequence!(
    u8,
    rosidl_runtime_c__uint8__Sequence__init,
    rosidl_runtime_c__uint8__Sequence__fini,
    rosidl_runtime_c__uint8__Sequence__copy
);

impl_primitive_sequence!(
    i16,
    rosidl_runtime_c__int16__Sequence__init,
    rosidl_runtime_c__int16__Sequence__fini,
    rosidl_runtime_c__int16__Sequence__copy
);

impl_primitive_sequence!(
    u16,
    rosidl_runtime_c__uint16__Sequence__init,
    rosidl_runtime_c__uint16__Sequence__fini,
    rosidl_runtime_c__uint16__Sequence__copy
);

impl_primitive_sequence!(
    i32,
    rosidl_runtime_c__int32__Sequence__init,
    rosidl_runtime_c__int32__Sequence__fini,
    rosidl_runtime_c__int32__Sequence__copy
);

impl_primitive_sequence!(
    u32,
    rosidl_runtime_c__uint32__Sequence__init,
    rosidl_runtime_c__uint32__Sequence__fini,
    rosidl_runtime_c__uint32__Sequence__copy
);

impl_primitive_sequence!(
    i64,
    rosidl_runtime_c__int64__Sequence__init,
    rosidl_runtime_c__int64__Sequence__fini,
    rosidl_runtime_c__int64__Sequence__copy
);

impl_primitive_sequence!(
    u64,
    rosidl_runtime_c__uint64__Sequence__init,
    rosidl_runtime_c__uint64__Sequence__fini,
    rosidl_runtime_c__uint64__Sequence__copy
);

impl_primitive_sequence!(
    bool,
    rosidl_runtime_c__boolean__Sequence__init,
    rosidl_runtime_c__boolean__Sequence__fini,
    rosidl_runtime_c__boolean__Sequence__copy
);
