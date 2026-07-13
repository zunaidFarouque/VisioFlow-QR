use image::RgbaImage;

/// Copy an RGBA image to the system clipboard.
pub fn copy_rgba_image_to_clipboard(image: &RgbaImage) -> Result<(), String> {
    let (width, height) = image.dimensions();
    let width = width as usize;
    let height = height as usize;
    if width == 0 || height == 0 {
        return Err("cannot copy empty image to clipboard".to_string());
    }

    let bytes = rgba_to_clipboard_bytes(image);
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    clipboard
        .set_image(arboard::ImageData {
            width,
            height,
            bytes: bytes.into(),
        })
        .map_err(|e| format!("clipboard image write failed: {e}"))
}

fn rgba_to_clipboard_bytes(image: &RgbaImage) -> Vec<u8> {
    let (width, height) = image.dimensions();
    let pixel_count = (width as usize) * (height as usize);
    let mut bytes = Vec::with_capacity(pixel_count * 4);

    for pixel in image.pixels() {
        let [r, g, b, a] = pixel.0;
        #[cfg(windows)]
        {
            bytes.extend_from_slice(&[b, g, r, a]);
        }
        #[cfg(not(windows))]
        {
            bytes.extend_from_slice(&[r, g, b, a]);
        }
    }

    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn rgba_to_clipboard_bytes_uses_platform_order() {
        let mut image = RgbaImage::new(1, 1);
        image.put_pixel(0, 0, Rgba([10, 20, 30, 255]));
        let bytes = rgba_to_clipboard_bytes(&image);
        #[cfg(windows)]
        assert_eq!(bytes, vec![30, 20, 10, 255]);
        #[cfg(not(windows))]
        assert_eq!(bytes, vec![10, 20, 30, 255]);
    }
}
