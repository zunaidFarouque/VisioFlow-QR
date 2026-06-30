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

fn seed_auto_rules(store: &std::path::Path) {
    let store_s = store.display().to_string();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args(["rule", "--store", &store_s, "create", "url"])
        .assert()
        .success();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "rule",
            "--store",
            &store_s,
            "config",
            "url",
            "--regex",
            r"^https?://\S+$",
        ])
        .assert()
        .success();

    let rules_path = store;
    let mut rules: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(rules_path).expect("read rules"))
            .expect("parse rules");
    rules["url"]["auto_compatible"] = serde_json::Value::Bool(true);
    rules["url"]["priority"] = serde_json::json!(10);
    std::fs::write(rules_path, serde_json::to_string_pretty(&rules).expect("encode"))
        .expect("write rules");

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args(["rule", "--store", &store_s, "create", "wifi"])
        .assert()
        .success();

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "rule",
            "--store",
            &store_s,
            "set-action",
            "wifi",
            "--wifi-connect",
        ])
        .assert()
        .success();

    let mut rules: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(rules_path).expect("read rules"))
            .expect("parse rules");
    rules["wifi"]["auto_compatible"] = serde_json::Value::Bool(true);
    rules["wifi"]["priority"] = serde_json::json!(5);
    std::fs::write(rules_path, serde_json::to_string_pretty(&rules).expect("encode"))
        .expect("write rules");

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args(["rule", "--store", &store_s, "create", "catchall"])
        .assert()
        .success();

    let mut rules: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(rules_path).expect("read rules"))
            .expect("parse rules");
    rules["catchall"]["auto_compatible"] = serde_json::Value::Bool(true);
    rules["catchall"]["priority"] = serde_json::json!(999);
    std::fs::write(rules_path, serde_json::to_string_pretty(&rules).expect("encode"))
        .expect("write rules");
}

#[test]
fn capture_auto_routes_url_rule() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("qr.png");
    render_qr_fixture(&fixture, "https://example.com");

    let store = dir.path().join("rules.json");
    seed_auto_rules(&store);

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "capture",
            "--source",
            "snip",
            "--store",
            &store.display().to_string(),
            "--input-image",
            &fixture.display().to_string(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(r#"matched rule "url""#))
        .stdout(predicate::str::is_empty());
}

#[test]
fn capture_explicit_mismatch_falls_back_to_copy() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("qr.png");
    render_qr_fixture(&fixture, "https://example.com");

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
            "--store",
            &store.display().to_string(),
            "--trigger",
            "asset",
            "--input-image",
            &fixture.display().to_string(),
        ])
        .assert()
        .success()
        .stderr(
            predicate::str::contains(r#"rule "asset" did not match; copied payload to clipboard"#),
        )
        .stdout(predicate::str::is_empty());
}

#[test]
fn capture_trigger_copy_builtin() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("qr.png");
    render_qr_fixture(&fixture, "copy-only-payload");

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "capture",
            "--source",
            "snip",
            "--trigger",
            "copy",
            "--input-image",
            &fixture.display().to_string(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("copy-only mode"))
        .stdout(predicate::str::is_empty());
}

#[test]
fn capture_auto_except_wifi_skips_wifi_rule() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("qr.png");
    render_qr_fixture(&fixture, "WIFI:T:WPA;S:lab;P:secret;;");

    let store = dir.path().join("rules.json");
    seed_auto_rules(&store);

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "capture",
            "--source",
            "snip",
            "--store",
            &store.display().to_string(),
            "--except",
            "wifi",
            "--input-image",
            &fixture.display().to_string(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("connecting to WiFi").not())
        .stderr(predicate::str::contains(r#"matched rule "catchall""#));
}
