use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn rule_create_config_execute_prints_resolved_vars() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = dir.path().join("rules.json");

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "rule",
            "--store",
            &store.display().to_string(),
            "create",
            "asset",
        ])
        .assert()
        .success();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "rule",
            "--store",
            &store.display().to_string(),
            "config",
            "asset",
            "--regex",
            r"ASSET:(?P<asset>\d+)",
            "--map",
            "asset:ASSET",
        ])
        .assert()
        .success();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "rule",
            "--store",
            &store.display().to_string(),
            "set-action",
            "asset",
            "--exec",
            "/bin/echo",
        ])
        .assert()
        .success();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "--output",
            "plain",
            "rule",
            "--store",
            &store.display().to_string(),
            "execute",
            "asset",
            "--payload",
            "ASSET:42",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("QR_RAW=ASSET:42"))
        .stdout(predicate::str::contains("QR_VAR_ASSET=42"));
}

#[test]
fn rule_execute_json_output() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = dir.path().join("rules.json");

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "rule",
            "--store",
            &store.display().to_string(),
            "create",
            "plain",
        ])
        .assert()
        .success();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "--output",
            "json",
            "rule",
            "--store",
            &store.display().to_string(),
            "execute",
            "plain",
            "--payload",
            "hello",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""QR_RAW":"hello""#));
}
