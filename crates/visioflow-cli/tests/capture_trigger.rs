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
fn capture_trigger_resolves_rule_vars_from_input_image() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("qr.png");
    render_qr_fixture(&fixture, "ASSET:99");

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
        ])
        .assert()
        .success();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "capture",
            "--source",
            "snip",
            "--action",
            "stdout",
            "--store",
            &store.display().to_string(),
            "--trigger",
            "asset",
            "--input-image",
            &fixture.display().to_string(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("QR_RAW=ASSET:99"))
        .stdout(predicate::str::contains("QR_VAR_ASSET=99"));
}

#[test]
fn capture_trigger_export_bash_includes_resolved_vars() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("qr.png");
    render_qr_fixture(&fixture, "hello-trigger");

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "--export",
            "bash",
            "capture",
            "--source",
            "snip",
            "--trigger",
            "plain",
            "--input-image",
            &fixture.display().to_string(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("export QR_RAW='hello-trigger'"));
}
