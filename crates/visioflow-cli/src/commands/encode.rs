use std::path::PathBuf;

use clap::ValueEnum;
use visioflow_core::encode::{encode_qr_rgba, resolve_encode_payload, QrErrorCorrection};
use visioflow_core::error::{Result, VisioFlowError};

use crate::clipboard_files::{read_clipboard_file_paths, read_clipboard_text};
use crate::clipboard_image::copy_rgba_image_to_clipboard;
use crate::commands::capture::PreviewPosition;
use crate::notifications::{send_native_notification, truncate_for_toast, NativeNotification};
use crate::qr_preview::show_qr_preview_window;

/// Minimum encoded QR image size in pixels per side.
const MIN_QR_PIXEL_SIZE: u32 = 512;

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq, Default)]
pub enum EncodeSource {
    #[default]
    Clipboard,
    Stdin,
    Text,
    File,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq, Default)]
pub enum EncodeEcc {
    #[default]
    M,
    L,
    Q,
    H,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq, Default)]
pub enum EncodeDeliver {
    /// Show square preview window only (default).
    #[default]
    Preview,
    /// Show preview and copy QR image to clipboard.
    PreviewCopy,
    /// Copy QR image to clipboard without preview.
    Copy,
    /// Render compact Unicode QR in terminal stdout.
    Terminal,
}

impl EncodeDeliver {
    #[must_use]
    pub const fn shows_preview(self) -> bool {
        matches!(self, Self::Preview | Self::PreviewCopy)
    }

    #[must_use]
    pub const fn copies_image(self) -> bool {
        matches!(self, Self::PreviewCopy | Self::Copy)
    }

    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Terminal)
    }
}

impl From<EncodeEcc> for QrErrorCorrection {
    fn from(value: EncodeEcc) -> Self {
        match value {
            EncodeEcc::L => Self::L,
            EncodeEcc::M => Self::M,
            EncodeEcc::Q => Self::Q,
            EncodeEcc::H => Self::H,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EncodeArgs {
    pub source: EncodeSource,
    pub preview_position: PreviewPosition,
    pub ecc: EncodeEcc,
    pub deliver: EncodeDeliver,
    pub notify: bool,
    pub verbose: bool,
    pub output: Option<PathBuf>,
    pub input_text: Option<String>,
    pub text: Option<String>,
    pub file: Option<PathBuf>,
}

pub fn run_encode(args: EncodeArgs) -> Result<String> {
    let payload = match args.source {
        EncodeSource::Text => args
            .text
            .or(args.input_text)
            .ok_or_else(|| VisioFlowError::Encode("no text provided for encoding".into()))?,
        EncodeSource::File => {
            let path = args
                .file
                .ok_or_else(|| VisioFlowError::Encode("no file provided for encoding".into()))?;
            std::fs::read_to_string(&path).map_err(|e| {
                VisioFlowError::Encode(format!("failed to read {}: {e}", path.display()))
            })?
        }
        EncodeSource::Stdin => {
            use std::io::Read;
            let mut buffer = String::new();
            std::io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|e| VisioFlowError::Encode(format!("failed to read stdin: {e}")))?;
            buffer.trim_end_matches(['\r', '\n']).to_string()
        }
        EncodeSource::Clipboard => {
            let (clipboard_text, file_paths) =
                resolve_clipboard_input(args.input_text.as_deref())?;

            if args.verbose {
                if !file_paths.is_empty() {
                    eprintln!(
                        "encode: found {} file path(s) on clipboard",
                        file_paths.len()
                    );
                    for path in &file_paths {
                        eprintln!("  {}", path.display());
                    }
                }
            }

            resolve_encode_payload(clipboard_text.as_deref(), &file_paths)?
        }
    };

    if args.verbose {
        eprintln!("encode: payload ({} bytes)", payload.len());
        eprintln!("  {payload}");
    }

    let encoded = encode_qr_rgba(&payload, args.ecc.into(), MIN_QR_PIXEL_SIZE)?;

    if let Some(path) = &args.output {
        save_qr_png(path, &encoded.image)?;
        if args.verbose {
            eprintln!("encode: wrote {}", path.display());
        }
    }

    if args.deliver.copies_image() {
        copy_rgba_image_to_clipboard(&encoded.image).map_err(VisioFlowError::Encode)?;
        if args.verbose {
            eprintln!("encode: copied QR image to clipboard");
        }
    }

    if args.notify && args.deliver.copies_image() {
        notify_encode_success(&payload)?;
    }

    if args.deliver.shows_preview() {
        show_qr_preview_window(
            &encoded.image,
            encoded.module_dimension,
            args.preview_position,
        )?;
    } else if args.deliver.is_terminal() {
        let terminal_qr = visioflow_core::encode_qr_terminal(&payload, args.ecc.into())?;
        println!("{terminal_qr}");
    }

    Ok(payload)
}

fn resolve_clipboard_input(
    input_text: Option<&str>,
) -> Result<(Option<String>, Vec<PathBuf>)> {
    if let Some(text) = input_text {
        return Ok((Some(text.to_string()), Vec::new()));
    }

    let text = read_clipboard_text();
    let files = read_clipboard_file_paths();
    Ok((text, files))
}

fn save_qr_png(path: &std::path::Path, image: &image::RgbaImage) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    image
        .save(path)
        .map_err(|e| VisioFlowError::Encode(format!("failed to save QR image: {e}")))
}

fn notify_encode_success(payload: &str) -> Result<()> {
    let body = truncate_for_toast(payload, 256);
    let note = NativeNotification {
        title: "VisioFlow QR".to_string(),
        body: format!("QR copied — {body}"),
        copy_payload: None,
        already_copied: true,
    };
    send_native_notification(&note).map_err(VisioFlowError::Encode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_ecc_maps_to_core_levels() {
        assert_eq!(QrErrorCorrection::from(EncodeEcc::L), QrErrorCorrection::L);
        assert_eq!(QrErrorCorrection::from(EncodeEcc::H), QrErrorCorrection::H);
    }

    #[test]
    fn deliver_preview_only_shows_window() {
        assert!(EncodeDeliver::Preview.shows_preview());
        assert!(!EncodeDeliver::Preview.copies_image());
    }

    #[test]
    fn deliver_preview_copy_shows_and_copies() {
        assert!(EncodeDeliver::PreviewCopy.shows_preview());
        assert!(EncodeDeliver::PreviewCopy.copies_image());
    }

    #[test]
    fn deliver_copy_only_copies() {
        assert!(!EncodeDeliver::Copy.shows_preview());
        assert!(EncodeDeliver::Copy.copies_image());
    }
}
