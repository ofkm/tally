use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[test]
fn run_prints_code_totals_for_paths() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    println!(\"hi\");\n}\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_tally"))
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();

    assert!(text.contains("Code: 3\n"));
    assert!(text.contains("Rust"));
    assert!(text.contains("TOTAL"));
}

#[test]
fn help_prints_usage_without_counting() {
    let output = Command::new(env!("CARGO_BIN_EXE_tally"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();

    assert!(text.contains("Usage: tally [PATH]..."));
    assert!(!text.contains("Language"));
}
