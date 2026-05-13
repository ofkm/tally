use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Request passed to the counting runtime.
#[derive(Debug, Clone)]
pub struct CountRequest {
    /// Input files or directories to count.
    pub inputs: Vec<PathBuf>,
    /// Runtime options that control discovery, counting, and output.
    pub options: CountOptions,
}

/// Options that control source discovery, counting, and report content.
#[derive(Debug, Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "count options mirror independent CLI/report toggles"
)]
pub struct CountOptions {
    /// Include per-file totals in the final report.
    pub by_file: bool,
    /// Only include files with these normalized extensions.
    pub include_ext: Vec<String>,
    /// Exclude files with these normalized extensions.
    pub exclude_ext: Vec<String>,
    /// Additional directory names to exclude while walking.
    pub exclude_dir: Vec<String>,
    /// Include only files whose path matches this regular expression.
    pub match_f: Option<String>,
    /// Exclude files whose path matches any of these regular expressions.
    pub not_match_f: Vec<String>,
    /// Descend only into directories whose path matches this regular expression.
    pub match_d: Option<String>,
    /// Exclude directories whose path matches any of these regular expressions.
    pub not_match_d: Vec<String>,
    /// Maximum file size in megabytes; `0` disables the limit.
    pub max_file_size_mb: f64,
    /// Follow symlinked directories while walking.
    pub follow_links: bool,
    /// Skip duplicate-content filtering.
    pub skip_uniqueness: bool,
    /// Suppress non-count diagnostics in reports.
    pub quiet: bool,
    /// Output format requested by the caller.
    pub output_format: OutputFormat,
}

impl Default for CountOptions {
    fn default() -> Self {
        Self {
            by_file: false,
            include_ext: Vec::new(),
            exclude_ext: Vec::new(),
            exclude_dir: Vec::new(),
            match_f: None,
            not_match_f: Vec::new(),
            match_d: None,
            not_match_d: Vec::new(),
            max_file_size_mb: 100.0,
            follow_links: false,
            skip_uniqueness: false,
            quiet: false,
            output_format: OutputFormat::Table,
        }
    }
}

/// Supported report rendering formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable table output.
    Table,
    /// JSON output for machine consumers.
    Json,
    /// YAML output compatible with cloc-style fixture comparisons.
    Yaml,
}

/// Aggregated line counts for one language or for the full report.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LanguageTotals {
    /// Number of files represented by this total.
    #[serde(rename = "nFiles")]
    pub files: u64,
    /// Number of blank lines.
    pub blank: u64,
    /// Number of comment lines.
    pub comment: u64,
    /// Number of code lines.
    pub code: u64,
}

impl LanguageTotals {
    /// Adds another total into this one.
    pub const fn add(&mut self, other: &Self) {
        self.files += other.files;
        self.blank += other.blank;
        self.comment += other.comment;
        self.code += other.code;
    }

    /// Returns the total number of physical lines.
    #[must_use]
    pub const fn lines(&self) -> u64 {
        self.blank + self.comment + self.code
    }
}

/// Line totals for a single counted file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileTotals {
    /// File path that was counted.
    pub path: PathBuf,
    /// Detected language name.
    pub language: String,
    /// Number of blank lines.
    pub blank: u64,
    /// Number of comment lines.
    pub comment: u64,
    /// Number of code lines.
    pub code: u64,
}

impl FileTotals {
    /// Converts file totals into an aggregate language total contribution.
    #[must_use]
    pub const fn as_language_totals(&self) -> LanguageTotals {
        LanguageTotals {
            files: 1,
            blank: self.blank,
            comment: self.comment,
            code: self.code,
        }
    }
}

/// Metadata captured at the start and end of a count run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportHeader {
    /// Count runtime in seconds.
    pub elapsed_seconds: f64,
    /// Number of counted files.
    pub n_files: u64,
    /// Number of counted lines.
    pub n_lines: u64,
}

/// Full count report produced by the runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// Runtime metadata.
    pub header: ReportHeader,
    /// Per-language totals sorted by descending code/comment/blank counts.
    pub languages: Vec<(String, LanguageTotals)>,
    /// Per-file totals, present when requested by options.
    pub files: Vec<FileTotals>,
    /// Aggregate totals across all counted files.
    pub sum: LanguageTotals,
    /// Discovery diagnostics for ignored files or directories.
    pub ignored: Vec<String>,
    /// Non-fatal file-level count errors.
    pub errors: Vec<String>,
}

/// Error type used by tally library and CLI operations.
#[derive(Debug, Error)]
pub enum CountError {
    /// An input path did not exist.
    #[error("input path does not exist: {0}")]
    MissingInput(PathBuf),
    /// A user-provided regular expression failed to compile.
    #[error("invalid regular expression `{pattern}`: {source}")]
    InvalidRegex {
        /// Original pattern.
        pattern: String,
        /// Regex compiler error.
        source: regex::Error,
    },
    /// Recursive directory walking failed.
    #[error("failed to walk input: {0}")]
    Walk(String),
    /// A file could not be read.
    #[error("failed to read {path}: {source}")]
    Read {
        /// Path that failed to read.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// Report rendering failed.
    #[error("failed to write output: {0}")]
    Output(String),
}
