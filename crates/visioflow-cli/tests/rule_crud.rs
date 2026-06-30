use assert_cmd::Command;
use predicates::prelude::*;

fn store_path(dir: &tempfile::TempDir) -> String {
    dir.path().join("rules.json").display().to_string()
}

#[test]
fn rule_list_plain_prints_names_one_per_line() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store_path(&dir);

    for name in ["alpha", "beta"] {
        Command::cargo_bin("visioflow")
            .expect("visioflow binary")
            .args(["rule", "--store", &store, "create", name])
            .assert()
            .success();
    }

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args(["--output", "plain", "rule", "--store", &store, "list"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"(?m)^alpha\nbeta\n$").unwrap());
}

#[test]
fn rule_list_json_outputs_full_rule_objects() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store_path(&dir);

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args(["rule", "--store", &store, "create", "asset"])
        .assert()
        .success();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "rule",
            "--store",
            &store,
            "config",
            "asset",
            "--regex",
            r"ASSET:(?P<asset>\d+)",
        ])
        .assert()
        .success();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args(["--output", "json", "rule", "--store", &store, "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""name":"asset""#))
        .stdout(predicate::str::contains(r"ASSET:(?P<asset>\\d+)"));
}

#[test]
fn rule_delete_removes_rule_from_store() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store_path(&dir);

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args(["rule", "--store", &store, "create", "gone"])
        .assert()
        .success();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args(["rule", "--store", &store, "delete", "gone"])
        .assert()
        .success();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args(["--output", "plain", "rule", "--store", &store, "list"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn rule_delete_errors_when_not_found() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store_path(&dir);

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args(["rule", "--store", &store, "delete", "missing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing"));
}
