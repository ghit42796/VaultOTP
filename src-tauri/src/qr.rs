use crate::error::{AppError, Result};
use image::{ImageBuffer, Luma};
use std::io::Cursor;

/// Render `data` as a QR code and return PNG bytes. Pure (no I/O).
/// Each module is scaled to an 8×8 pixel block with a 4-module quiet zone.
pub fn encode_png(data: &str) -> Result<Vec<u8>> {
    let code = qrcode::QrCode::new(data.as_bytes()).map_err(|_| AppError::QrDecode)?;
    let modules = code.width(); // module count per side (usize)
    let colors = code.into_colors(); // row-major, len == modules*modules

    const SCALE: u32 = 8;
    const QUIET: u32 = 4; // modules on each side
    let side = (modules as u32 + 2 * QUIET) * SCALE;
    let mut img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_pixel(side, side, Luma([255u8]));

    for (i, c) in colors.iter().enumerate() {
        if c == &qrcode::Color::Light {
            continue;
        }
        let mx = (i % modules) as u32;
        let my = (i / modules) as u32;
        let x0 = (QUIET + mx) * SCALE;
        let y0 = (QUIET + my) * SCALE;
        for dy in 0..SCALE {
            for dx in 0..SCALE {
                img.put_pixel(x0 + dx, y0 + dy, Luma([0u8]));
            }
        }
    }

    let mut buf = Vec::new();
    image::DynamicImage::ImageLuma8(img)
        .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|_| AppError::QrDecode)?;
    Ok(buf)
}

pub fn decode_image_bytes(bytes: &[u8]) -> Result<Vec<String>> {
    let img = image::load_from_memory(bytes).map_err(|_| AppError::QrDecode)?;
    let luma = img.to_luma8();
    let mut prepared = rqrr::PreparedImage::prepare(luma);
    let grids = prepared.detect_grids();
    let mut out = Vec::new();
    for g in grids {
        if let Ok((_meta, content)) = g.decode() {
            out.push(content);
        }
    }
    if out.is_empty() {
        return Err(AppError::QrDecode);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_fixture_qr() {
        // Fixture: tests/fixtures/qr_otpauth.png encodes
        // otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example
        let bytes = include_bytes!("../tests/fixtures/qr_otpauth.png");
        let results = decode_image_bytes(bytes).unwrap();
        assert_eq!(
            results[0],
            "otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example"
        );
    }

    #[test]
    fn non_image_bytes_error() {
        assert!(matches!(decode_image_bytes(b"not an image"), Err(AppError::QrDecode)));
    }

    #[test]
    fn encode_png_round_trips_through_decode() {
        let uri = "otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example";
        let png = encode_png(uri).unwrap();
        // PNG magic bytes
        assert_eq!(&png[..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        let decoded = decode_image_bytes(&png).unwrap();
        assert!(decoded.iter().any(|s| s == uri));
    }

    #[test]
    fn encode_png_rejects_oversized_input() {
        // QR has a capacity ceiling; a huge string must error, not panic.
        let big = "a".repeat(10_000);
        assert!(encode_png(&big).is_err());
    }
}
