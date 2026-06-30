use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn notify_test_command_parses_and_exits_zero() {
    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args(["notify", "test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("toast"));
}

#[test]
fn notify_test_accepts_custom_title_and_body() {
    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "notify",
            "test",
            "--title",
            "Smoke Title",
            "--body",
            "Smoke Body",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Smoke Title"));
}
