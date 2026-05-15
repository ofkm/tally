use std::path::PathBuf;

use thiserror::Error;

/// Request passed to the counting runtime.
#[derive(Debug, Clone)]
pub struct CountRequest {
    /// Input files or directories to count.
    pub inputs: Vec<PathBuf>,
    /// Include per-language directory totals.
    pub tree: bool,
}

/// Aggregated line counts for one language or for the full report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LanguageTotals {
    /// Number of files represented by this total.
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
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// Aggregated line counts for one directory within a language tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryTotals {
    /// Directory path represented by this total.
    pub path: PathBuf,
    /// Aggregated counts for all matching files under this directory.
    pub totals: LanguageTotals,
}

/// Directory totals for one language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageTree {
    /// Language name for this directory tree.
    pub language: String,
    /// Directories sorted with the root first, then by path name.
    pub directories: Vec<DirectoryTotals>,
}

/// Full count report produced by the runtime.
#[derive(Debug, Clone)]
pub struct Report {
    /// Per-language totals sorted by descending code/comment/blank counts.
    pub languages: Vec<(String, LanguageTotals)>,
    /// Per-language directory totals, present when requested.
    pub tree: Vec<LanguageTree>,
    /// Aggregate totals across all counted files.
    pub sum: LanguageTotals,
    /// Non-fatal file-level count errors.
    pub errors: Vec<String>,
}

/// Error type used by tally library and CLI operations.
#[derive(Debug, Error)]
pub enum CountError {
    /// A command-line argument was not recognized.
    #[error("unknown argument: {0}")]
    InvalidArgument(String),
    /// An input path did not exist.
    #[error("input path does not exist: {0}")]
    MissingInput(PathBuf),
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
}
