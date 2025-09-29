use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

fn sample_path() -> String {
    format!("{}/tests/data/sample_tree.json", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn prints_full_tree_by_default() {
    let mut cmd = Command::cargo_bin("platynui-spy-cli").expect("binary");
    cmd.arg("--input").arg(sample_path());
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Calculator"))
        .stdout(predicate::str::contains("Number Pad"))
        .stdout(predicate::str::contains("AutomationId=CalcWindow"))
        .stdout(predicate::str::contains("AutomationId=num1Button"));
}

#[test]
fn filters_by_role_and_attribute() {
    let mut cmd = Command::cargo_bin("platynui-spy-cli").expect("binary");
    cmd.arg("--input")
        .arg(sample_path())
        .arg("--filter-role")
        .arg("button")
        .arg("--format")
        .arg("json")
        .arg("--filter-attr")
        .arg("AutomationId=num2Button")
        .arg("--no-include-ancestors");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"Two\""))
        .stdout(predicate::str::contains("num2Button"))
        .stdout(predicate::str::contains("\"role\": \"button\""));
}

#[test]
fn emits_full_attribute_set() {
    let mut cmd = Command::cargo_bin("platynui-spy-cli").expect("binary");
    cmd.arg("--input")
        .arg(sample_path())
        .arg("--attribute-set")
        .arg("full");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("processId=4242"));
}

#[test]
fn appends_custom_attribute_keys() {
    let mut cmd = Command::cargo_bin("platynui-spy-cli").expect("binary");
    cmd.arg("--input")
        .arg(sample_path())
        .arg("--attribute")
        .arg("processId");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("processId=4242"));
}

#[test]
fn filters_tree_with_xpath() {
    let mut cmd = Command::cargo_bin("platynui-spy-cli").expect("binary");
    cmd.arg("--input")
        .arg(sample_path())
        .arg("--xpath")
        .arg("/Desktop/Calculator/Number Pad");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Number Pad"))
        .stdout(predicate::str::contains("One [button]"))
        .stdout(predicate::str::contains("Two [button]"))
        .stdout(predicate::str::contains("AutomationId=num1Button"))
        .stdout(predicate::str::contains("AutomationId=num2Button"));
}

#[test]
fn errors_on_missing_input() {
    let mut cmd = Command::cargo_bin("platynui-spy-cli").expect("binary");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("missing --input"));
}
