use std::path::PathBuf;

use clap::{ArgAction, Parser, ValueHint};

use crate::output;
use crate::runtime::count;
use crate::types::{CountError, CountOptions, CountRequest, OutputFormat};

#[derive(Debug, Parser)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "CLI flag structs naturally mirror independent boolean flags"
)]
#[command(
    version,
    about = "Count source lines by language",
    long_about = "tally is a modern Rust line counter inspired by cloc. It counts physical source lines by language, walks directories recursively, respects ignore files, and skips heavy generated directories such as .git, node_modules, and target by default.",
    after_help = "Examples:
  tally
  tally src tests --json
  tally --yaml --by-file --include-ext rs,toml .
  tally --exclude-dir vendor --not-match-f '(^|/)fixtures/' ."
)]
struct Args {
    #[arg(
        value_name = "PATH",
        default_value = ".",
        value_hint = ValueHint::AnyPath,
        help = "Files or directories to count"
    )]
    inputs: Vec<PathBuf>,

    #[arg(long, help = "Write machine-readable JSON output")]
    json: bool,

    #[arg(long, help = "Write machine-readable YAML output")]
    yaml: bool,

    #[arg(
        long = "by-file",
        alias = "by_file",
        help = "Include per-file totals in machine-readable output and table output"
    )]
    by_file: bool,

    #[arg(
        long = "include-ext",
        alias = "include_ext",
        value_delimiter = ',',
        value_name = "EXT[,EXT...]",
        help = "Only count files with these extensions"
    )]
    include_ext: Vec<String>,

    #[arg(
        long = "exclude-ext",
        alias = "exclude_ext",
        value_delimiter = ',',
        value_name = "EXT[,EXT...]",
        help = "Skip files with these extensions"
    )]
    exclude_ext: Vec<String>,

    #[arg(
        long = "exclude-dir",
        alias = "exclude_dir",
        value_delimiter = ',',
        value_name = "DIR[,DIR...]",
        help = "Skip directories by name in addition to the default ignored directories"
    )]
    exclude_dir: Vec<String>,

    #[arg(
        long = "match-f",
        alias = "match_f",
        value_name = "REGEX",
        help = "Only count files whose path matches this regex"
    )]
    match_f: Option<String>,

    #[arg(
        long = "not-match-f",
        alias = "not_match_f",
        action = ArgAction::Append,
        value_name = "REGEX",
        help = "Skip files whose path matches this regex; can be repeated"
    )]
    not_match_f: Vec<String>,

    #[arg(
        long = "match-d",
        alias = "match_d",
        value_name = "REGEX",
        help = "Only descend into directories whose path matches this regex"
    )]
    match_d: Option<String>,

    #[arg(
        long = "not-match-d",
        alias = "not_match_d",
        action = ArgAction::Append,
        value_name = "REGEX",
        help = "Skip directories whose path matches this regex; can be repeated"
    )]
    not_match_d: Vec<String>,

    #[arg(
        long = "max-file-size",
        alias = "max_file_size",
        default_value_t = 100.0,
        value_name = "MB",
        help = "Skip files larger than this many megabytes; use 0 to disable the size limit"
    )]
    max_file_size: f64,

    #[arg(
        long = "follow-links",
        alias = "follow_links",
        help = "Follow symlinked directories while walking"
    )]
    follow_links: bool,

    #[arg(
        long = "skip-uniqueness",
        alias = "skip_uniqueness",
        help = "Do not hash files to remove duplicate content"
    )]
    skip_uniqueness: bool,

    #[arg(long, help = "Suppress non-count diagnostics in reports")]
    quiet: bool,
}

/// Parses CLI arguments, runs the count, and writes the selected report.
///
/// # Errors
///
/// Returns [`CountError`] when discovery, counting, or report formatting fails.
pub fn run() -> Result<(), CountError> {
    let args = Args::parse();
    let output_format = report_format(&args);
    let request = CountRequest {
        inputs: args.inputs,
        options: CountOptions {
            by_file: args.by_file,
            include_ext: normalize_exts(args.include_ext),
            exclude_ext: normalize_exts(args.exclude_ext),
            exclude_dir: args.exclude_dir,
            match_f: args.match_f,
            not_match_f: args.not_match_f,
            match_d: args.match_d,
            not_match_d: args.not_match_d,
            max_file_size_mb: args.max_file_size,
            follow_links: args.follow_links,
            skip_uniqueness: args.skip_uniqueness,
            quiet: args.quiet,
            output_format,
        },
    };
    let report = count(&request)?;

    let text = output::format_report(&report, output_format)?;
    print!("{text}");
    Ok(())
}

const fn report_format(args: &Args) -> OutputFormat {
    if args.json {
        OutputFormat::Json
    } else if args.yaml {
        OutputFormat::Yaml
    } else {
        OutputFormat::Table
    }
}

fn normalize_exts(exts: Vec<String>) -> Vec<String> {
    exts.into_iter()
        .map(|ext| ext.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|ext| !ext.is_empty())
        .collect()
}
