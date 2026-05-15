//! Library interface for `tally`, a small source line counter.
//!
//! The crate exposes reusable modules for discovering source files, classifying
//! languages, counting blank/comment/code lines, and rendering a compact report.

/// Command-line parsing and binary entrypoint helpers.
pub mod cli;
/// Source line counting for individual files and text buffers.
pub mod counter;
/// Recursive file discovery and filtering.
pub mod discovery;
/// Language classification and syntax definitions.
pub mod language;
/// Compact text report rendering.
pub mod output;
/// End-to-end counting orchestration.
pub mod runtime;
/// Public request, report, and error types.
pub mod types;
