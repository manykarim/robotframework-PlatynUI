use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn prints_tree_in_text_format() {
    let mut cmd = Command::cargo_bin("spy-cli").unwrap();
    cmd.arg("--input")
        .arg(fixture_path("sample_tree.json"))
        .arg("--include-properties");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("[window] Calculator"))
        .stdout(predicate::str::contains("AutomationId: \"rootWindow\""));
}

#[test]
fn filters_with_role_and_max_depth() {
    let mut cmd = Command::cargo_bin("spy-cli").unwrap();
    cmd.arg("--input")
        .arg(fixture_path("sample_tree.json"))
        .arg("--role")
        .arg("button")
        .arg("--max-depth")
        .arg("2")
        .arg("--format")
        .arg("json");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"role\": \"button\""))
        .stdout(predicate::str::contains("\"name\": \"+\""));
}

#[test]
fn rejects_invalid_property_argument() {
    let mut cmd = Command::cargo_bin("spy-cli").unwrap();
    cmd.arg("--input")
        .arg(fixture_path("sample_tree.json"))
        .arg("--property")
        .arg("missing-value");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("key=value"));
}
