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

#[test]
fn encode_text_flag_encodes_direct_payload() {
    let output = tempfile::NamedTempFile::with_suffix(".png").expect("temp png");
    let png_path = output.path().to_string_lossy().to_string();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "encode",
            "--text",
            "https://visioflow.local/direct-text",
            "--deliver",
            "copy",
            "--no-notify",
            "--output",
            &png_path,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("https://visioflow.local/direct-text"));

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
        .stdout(predicate::str::contains("https://visioflow.local/direct-text"));
}

#[test]
fn encode_file_flag_encodes_file_contents() {
    let mut file = NamedTempFile::with_suffix(".txt").expect("temp txt");
    writeln!(file, "https://visioflow.local/file-content").expect("write");
    file.flush().expect("flush");

    let output = tempfile::NamedTempFile::with_suffix(".png").expect("temp png");
    let png_path = output.path().to_string_lossy().to_string();
    let file_path = file.path().to_string_lossy().to_string();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "encode",
            "--file",
            &file_path,
            "--deliver",
            "copy",
            "--no-notify",
            "--output",
            &png_path,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("https://visioflow.local/file-content"));

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
        .stdout(predicate::str::contains("https://visioflow.local/file-content"));
}

#[test]
fn encode_terminal_renders_unicode_qr() {
    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "encode",
            "--terminal",
            "--text",
            "https://visioflow.local/terminal-qr",
            "--no-notify",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("▀").or(predicate::str::contains("▄")));
}

