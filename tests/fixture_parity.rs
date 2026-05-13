use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use serde::Deserialize;
use tempfile::tempdir;

#[derive(Debug, Deserialize)]
struct Totals {
    #[serde(rename = "nFiles")]
    files: u64,
    blank: u64,
    comment: u64,
    code: u64,
}

#[derive(Debug, Deserialize)]
struct RustReport {
    #[serde(flatten)]
    sections: BTreeMap<String, serde_yaml_ng::Value>,
}

#[test]
fn c_fixture_matches_upstream_counts() {
    assert_fixture("cloc/tests/inputs/C-Ansi.c", "C", 1, 2, 2, 7);
}

#[test]
fn blade_fixture_matches_upstream_counts() {
    assert_fixture("cloc/tests/inputs/master.blade.php", "Blade", 1, 10, 5, 22);
}

#[test]
fn svelte_fixture_matches_upstream_counts() {
    assert_fixture("cloc/tests/inputs/reactive.svelte", "Svelte", 1, 2, 2, 9);
}

#[test]
fn svelte_script_comment_fixture_matches_upstream_counts() {
    assert_fixture(
        "cloc/tests/inputs/test_w_cpp_comments.svelte",
        "Svelte",
        1,
        3,
        7,
        4,
    );
}

#[test]
fn tree_flag_reports_directory_shares_by_language() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("src/bin")).unwrap();
    fs::write(dir.path().join("src/main.rs"), "fn main() {}\n\n").unwrap();
    fs::write(
        dir.path().join("src/bin/tool.rs"),
        "// comment\nfn tool() {}\n",
    )
    .unwrap();

    let output = Command::cargo_bin("tally")
        .unwrap()
        .args(["--tree", "--skip-uniqueness"])
        .arg(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();

    assert!(text.contains("Tree:"));
    assert!(text.contains("Rust:"));
    assert!(text.contains("+-- src"));
    assert!(text.contains("+-- bin"));
    assert!(text.contains("100.0%"));
    assert!(text.contains("50.0%"));
}

fn assert_fixture(input: &str, language: &str, files: u64, blank: u64, comment: u64, code: u64) {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = Command::cargo_bin("tally")
        .unwrap()
        .args(["--yaml", "--skip-uniqueness"])
        .arg(repo.join(input))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let report: RustReport = serde_yaml_ng::from_slice(&output).unwrap();
    let totals: Totals = serde_yaml_ng::from_value(report.sections[language].clone()).unwrap();
    assert_eq!(totals.files, files);
    assert_eq!(totals.blank, blank);
    assert_eq!(totals.comment, comment);
    assert_eq!(totals.code, code);
}
