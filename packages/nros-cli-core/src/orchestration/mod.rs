//! Build-time orchestration schemas.
//!
//! These types are data contracts only. They intentionally avoid launch
//! parsing, generated runtime code, and target runtime crates.

pub mod config;
pub mod plan;
pub mod schema;
pub mod source_metadata;

pub use config::{ComponentConfig, SystemConfig};
pub use plan::NrosPlan;
pub use source_metadata::SourceMetadata;
