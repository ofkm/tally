//! Library interface for `tally`, a source line counter inspired by cloc.
//!
//! The crate exposes reusable modules for discovering source files, classifying
//! languages, counting blank/comment/code lines, and rendering reports.

/// Command-line parsing and binary entrypoint helpers.
pub mod cli;
/// Source line counting for individual files and text buffers.
pub mod counter;
/// Recursive file discovery and filtering.
pub mod discovery;
/// Language classification and syntax definitions.
pub mod language;
/// Report rendering for table, JSON, and YAML formats.
pub mod output;
/// End-to-end counting orchestration.
pub mod runtime;
/// Public request, report, and error types.
pub mod types;
