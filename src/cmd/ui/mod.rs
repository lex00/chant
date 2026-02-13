//! CLI-specific UI output and formatting modules

pub mod formatters;
pub mod icons;
pub mod output;
pub mod render;

// Re-export commonly used items for convenience
pub use icons::is_quiet;
pub use output::{Output, OutputMode};
