//! Idiomatic Rust wrapper for ROS 2 strings
//!
//! Provides a safe, user-friendly API around the C `rosidl_runtime_c__String` type.

use crate::ffi;
use std::ffi::CString;
use std::fmt;

/// ROS 2 string with automatic memory management
///
/// This is a safe, idiomatic wrapper around the C `rosidl_runtime_c__String`.
/// Memory is automatically allocated/deallocated via Drop.
///
/// # Example
/// ```ignore
/// use rosidl_runtime_rs::String;
///
/// let mut s = String::from("Hello, ROS!");
/// println!("String: {}", s);
/// println!("Length: {}", s.len());
///
/// s.assign("Updated!").unwrap();
/// ```
pub struct String {
    inner: ffi::rosidl_runtime_c__String,
}

impl String {
    /// Create a new empty string
    pub fn new() -> Self {
        let mut inner = ffi::rosidl_runtime_c__String {
            data: std::ptr::null_mut(),
            size: 0,
            capacity: 0,
        };
        unsafe {
            ffi::rosidl_runtime_c__String__init(&mut inner);
        }
        Self { inner }
    }

    /// Get string contents as a Rust str slice
    pub fn as_str(&self) -> &str {
        if self.inner.data.is_null() || self.inner.size == 0 {
            return "";
        }
        unsafe {
            let slice = std::slice::from_raw_parts(self.inner.data as *const u8, self.inner.size);
            std::str::from_utf8_unchecked(slice)
        }
    }

    /// Get the length in bytes (excluding null terminator)
    pub fn len(&self) -> usize {
        self.inner.size
    }

    /// Check if string is empty
    pub fn is_empty(&self) -> bool {
        self.inner.size == 0
    }

    /// Assign a new value to the string
    pub fn assign(&mut self, value: &str) -> Result<(), StringError> {
        let c_str = CString::new(value).map_err(|_| StringError::NulByteInString)?;

        unsafe {
            if ffi::rosidl_runtime_c__String__assign(&mut self.inner, c_str.as_ptr()) {
                Ok(())
            } else {
                Err(StringError::AllocationFailed)
            }
        }
    }

    /// Get mutable access to the underlying FFI type
    ///
    /// # Safety
    /// Caller must ensure the FFI type remains valid and properly initialized
    pub unsafe fn as_mut_ffi(&mut self) -> &mut ffi::rosidl_runtime_c__String {
        &mut self.inner
    }

    /// Get immutable access to the underlying FFI type
    pub fn as_ffi(&self) -> &ffi::rosidl_runtime_c__String {
        &self.inner
    }
}

impl Default for String {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for String {
    fn drop(&mut self) {
        unsafe {
            ffi::rosidl_runtime_c__String__fini(&mut self.inner);
        }
    }
}

impl Clone for String {
    fn clone(&self) -> Self {
        let mut new_string = String::new();
        unsafe {
            ffi::rosidl_runtime_c__String__copy(&self.inner, &mut new_string.inner);
        }
        new_string
    }
}

impl PartialEq for String {
    fn eq(&self, other: &Self) -> bool {
        unsafe { ffi::rosidl_runtime_c__String__are_equal(&self.inner, &other.inner) }
    }
}

impl Eq for String {}

impl From<std::string::String> for String {
    fn from(s: std::string::String) -> Self {
        let mut ros_str = String::new();
        ros_str
            .assign(&s)
            .expect("Failed to allocate ROS string from Rust string");
        ros_str
    }
}

impl From<&str> for String {
    fn from(s: &str) -> Self {
        String::from(s.to_string())
    }
}

impl From<&std::string::String> for String {
    fn from(s: &std::string::String) -> Self {
        String::from(s.as_str())
    }
}

impl From<String> for std::string::String {
    fn from(s: String) -> Self {
        s.as_str().to_string()
    }
}

impl From<&String> for std::string::String {
    fn from(s: &String) -> Self {
        s.as_str().to_string()
    }
}

impl fmt::Display for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl fmt::Debug for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "String({:?})", self.as_str())
    }
}

/// Errors that can occur during string operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringError {
    /// String contains an interior null byte
    NulByteInString,
    /// Memory allocation failed
    AllocationFailed,
}

impl fmt::Display for StringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StringError::NulByteInString => write!(f, "String contains null byte"),
            StringError::AllocationFailed => write!(f, "Memory allocation failed"),
        }
    }
}

impl std::error::Error for StringError {}
