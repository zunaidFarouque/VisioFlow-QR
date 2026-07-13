use std::io::Write;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

#[test]
fn encode_clipboard_text_writes_png_and_stdout_payload() {
    let output = tempfile::NamedTempFile::with_suffix(".png").expect("temp png");
    let png_path = output.path().to_string_lossy().to_string();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "encode",
            "--source",
            "clipboard",
            "--input-text",
            "https://example.com/encode-test",
            "--deliver",
            "copy",
            "--no-notify",
            "--output",
            &png_path,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("https://example.com/encode-test"));

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "capture",
            "--source",
            "snip",
            "--input-image",
            &png_path,
            "--action",
            "stdout",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("https://example.com/encode-test"));
}

#[test]
fn encode_extracts_url_from_shortcut_file_path() {
    let mut shortcut = NamedTempFile::with_suffix(".url").expect("shortcut");
    writeln!(
        shortcut,
        "[InternetShortcut]\nURL=https://shortcut.example/page\n"
    )
    .expect("write");
    shortcut.flush().expect("flush");

    let output = tempfile::NamedTempFile::with_suffix(".png").expect("temp png");
    let png_path = output.path().to_string_lossy().to_string();
    let shortcut_path = shortcut.path().to_string_lossy().to_string();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "encode",
            "--source",
            "clipboard",
            "--input-text",
            &shortcut_path,
            "--deliver",
            "copy",
            "--no-notify",
            "--output",
            &png_path,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("https://shortcut.example/page"));

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "capture",
            "--source",
            "snip",
            "--input-image",
            &png_path,
            "--action",
            "stdout",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("https://shortcut.example/page"));
}
