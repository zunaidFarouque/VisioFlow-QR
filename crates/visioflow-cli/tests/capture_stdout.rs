use assert_cmd::Command;
use image::{GrayImage, Luma};
use predicates::prelude::*;
use visioflow_cli::commands::capture::run_capture_with_source;
use visioflow_core::traits::OpticalFilterKind;

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

struct GrayFixtureSource {
    image: GrayImage,
}

impl visioflow_core::traits::FrameSource for GrayFixtureSource {
    fn capture_frame(&self) -> visioflow_core::error::Result<image::DynamicImage> {
        Ok(image::DynamicImage::ImageLuma8(self.image.clone()))
    }
}

#[test]
fn capture_engine_decodes_fixture_image() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("qr.png");
    render_qr_fixture(&fixture, "visioflow-mvp-payload");

    let source = GrayFixtureSource {
        image: image::open(&fixture)
            .expect("open fixture")
            .to_luma8(),
    };

    let payloads =
        run_capture_with_source(source, OpticalFilterKind::Otsu).expect("capture should succeed");
    assert_eq!(payloads, vec!["visioflow-mvp-payload"]);
}

#[test]
fn cli_capture_with_input_image_prints_payload() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("qr.png");
    render_qr_fixture(&fixture, "cli-stdout-payload");

    Command::cargo_bin("visioflow")
        .expect("visioflow binary")
        .args([
            "capture",
            "--source",
            "snip",
            "--action",
            "stdout",
            "--input-image",
            &fixture.display().to_string(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("cli-stdout-payload"));
}
