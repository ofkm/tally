use std::process::ExitCode;

fn main() -> ExitCode {
    match tally::cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("tally: {err}");
            ExitCode::FAILURE
        }
    }
}
