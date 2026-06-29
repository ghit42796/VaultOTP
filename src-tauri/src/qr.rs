use crate::error::{AppError, Result};

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
}
