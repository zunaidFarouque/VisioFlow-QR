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
            "--no-exec",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("QR_RAW=ASSET:42"))
        .stdout(predicate::str::contains("QR_VAR_ASSET=42"));
}

#[test]
fn rule_execute_wifi_yields_native_ssid() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = dir.path().join("rules.json");

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "rule",
            "--store",
            &store.display().to_string(),
            "create",
            "wifi",
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
            "wifi",
            "--payload",
            "WIFI:T:WPA;S:lab;P:secret;;",
            "--no-exec",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "QR_RAW=WIFI:T:WPA;S:lab;P:secret;;",
        ))
        .stdout(predicate::str::contains("QR_NATIVE_WIFI_SSID=lab"));
}

#[test]
fn rule_execute_spawns_exec_with_resolved_env() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = dir.path().join("rules.json");
    let out_path = dir.path().join("child-out.txt");
    let script_path = write_env_echo_script(&dir, &out_path);

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "rule",
            "--store",
            &store.display().to_string(),
            "create",
            "run",
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
            "run",
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
            "run",
            "--exec",
            &script_path.display().to_string(),
        ])
        .assert()
        .success();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "--silent",
            "rule",
            "--store",
            &store.display().to_string(),
            "execute",
            "run",
            "--payload",
            "ASSET:99",
        ])
        .assert()
        .success();

    let contents = std::fs::read_to_string(&out_path).expect("read child output");
    assert!(contents.contains("99"));
}

#[cfg(windows)]
fn write_env_echo_script(
    dir: &tempfile::TempDir,
    out_path: &std::path::Path,
) -> std::path::PathBuf {
    let script_path = dir.path().join("echo-asset.cmd");
    let body = format!(
        "@echo off\r\necho %QR_VAR_ASSET% > \"{}\"\r\n",
        out_path.display()
    );
    std::fs::write(&script_path, body).expect("write cmd");
    script_path
}

#[cfg(not(windows))]
fn write_env_echo_script(
    dir: &tempfile::TempDir,
    out_path: &std::path::Path,
) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let script_path = dir.path().join("echo-asset.sh");
    let body = format!(
        "#!/bin/sh\necho \"$QR_VAR_ASSET\" > \"{}\"\n",
        out_path.display()
    );
    std::fs::write(&script_path, &body).expect("write sh");
    let mut perms = std::fs::metadata(&script_path).expect("meta").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script_path, perms).expect("chmod");
    script_path
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
            "demo",
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
            "demo",
            "--payload",
            "hello",
            "--no-exec",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""QR_RAW":"hello""#));
}
