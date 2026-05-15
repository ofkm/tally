use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

use crate::output;
use crate::runtime::count;
use crate::types::{CountError, CountRequest};
const HELP: &str = "Usage: tally [--tree] [PATH]...

Count source code lines by language.

With no paths, tally counts the current directory.
";

/// Parses CLI arguments, runs the count, and writes the selected report.
///
/// # Errors
///
/// Returns [`CountError`] when discovery, counting, or report formatting fails.
pub fn run() -> Result<(), CountError> {
    let Action::Count { inputs, tree } = parse_args(env::args_os().skip(1))? else {
        return Ok(());
    };
    let request = CountRequest { inputs, tree };
    let report = count(&request)?;

    let text = output::format_report(&report);
    print!("{text}");
    Ok(())
}

enum Action {
    Count { inputs: Vec<PathBuf>, tree: bool },
    PrintOnly,
}

fn parse_args(args: impl Iterator<Item = OsString>) -> Result<Action, CountError> {
    let mut inputs = Vec::new();
    let mut tree = false;
    for arg in args {
        if arg == "-h" || arg == "--help" {
            print!("{HELP}");
            return Ok(Action::PrintOnly);
        }
        if arg == "-V" || arg == "--version" {
            println!("tally {}", env!("CARGO_PKG_VERSION"));
            return Ok(Action::PrintOnly);
        }
        if arg == "--tree" {
            tree = true;
            continue;
        }
        if arg.as_encoded_bytes().starts_with(b"-") {
            return Err(CountError::InvalidArgument(
                arg.to_string_lossy().into_owned(),
            ));
        }
        inputs.push(PathBuf::from(arg));
    }

    if inputs.is_empty() {
        inputs.push(PathBuf::from("."));
    }
    Ok(Action::Count { inputs, tree })
}
