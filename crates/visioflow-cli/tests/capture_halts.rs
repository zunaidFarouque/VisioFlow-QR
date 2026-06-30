use assert_cmd::Command;
use image::{GrayImage, Luma};
use predicates::prelude::*;

fn render_qr_fixture(path: &std::path::Path, payload: &str) {
    use qrcode::QrCode;

    let code = QrCode::new(payload.as_bytes()).expect("valid qr");
    let modules = code.to_colors();
    let dimension = code.width();
    let scale = 8u32;
    let size = dimension as u32 * scale;
    let mut image = GrayImage::from_pixel(size, size, Luma([255u8]));

    for y in 0..dimension {
        for x in 0..dimension {
            let idx = y * dimension + x;
            if modules[idx] == qrcode::Color::Dark {
                for dy in 0..scale {
                    for dx in 0..scale {
                        image.put_pixel(
                            x as u32 * scale + dx,
                            y as u32 * scale + dy,
                            Luma([0u8]),
                        );
                    }
                }
            }
        }
    }

    image.save(path).expect("write fixture");
}

#[test]
fn cli_capture_interactive_proceeds_on_yes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("qr.png");
    render_qr_fixture(&fixture, "halt-yes-payload");

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "capture",
            "--source",
            "snip",
            "--action",
            "stdout",
            "--interactive",
            "--input-image",
            &fixture.display().to_string(),
        ])
        .write_stdin("y\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("halt-yes-payload"));
}

#[test]
fn cli_capture_interactive_cancels_on_empty_input() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("qr.png");
    render_qr_fixture(&fixture, "halt-no-payload");

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "capture",
            "--source",
            "snip",
            "--action",
            "stdout",
            "--interactive",
            "--input-image",
            &fixture.display().to_string(),
        ])
        .write_stdin("\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cancelled"));
}

#[test]
fn cli_capture_single_payload_skips_select_menu() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("qr.png");
    render_qr_fixture(&fixture, "select-skip-payload");

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "capture",
            "--source",
            "snip",
            "--action",
            "stdout",
            "--select",
            "--input-image",
            &fixture.display().to_string(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("select-skip-payload"))
        .stderr(predicate::str::contains("Multiple payloads detected").not());
}
