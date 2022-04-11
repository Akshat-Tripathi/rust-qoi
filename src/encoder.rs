use std::io::Write;

use image::error::{ImageError, ImageFormatHint, UnsupportedError, UnsupportedErrorKind};
use image::ImageEncoder;

pub struct QoiEncoder<W: Write> {
    w: W,
}

impl<W: Write> QoiEncoder<W> {
    pub fn new(w: W) -> QoiEncoder<W> {
        QoiEncoder { w }
    }

    fn encode_rgb(self, buf: &[u8], width: u32, height: u32) -> image::ImageResult<()> {
        Ok(())
    }
    fn encode_rgba(self, buf: &[u8], width: u32, height: u32) -> image::ImageResult<()> {
        Ok(())
    }
}

impl<W: Write> ImageEncoder for QoiEncoder<W> {
    fn write_image(
        self,
        buf: &[u8],
        width: u32,
        height: u32,
        color_type: image::ColorType,
    ) -> image::ImageResult<()> {
        match color_type {
            image::ColorType::Rgb8 => self.encode_rgb(buf, width, height),
            image::ColorType::Rgba8 => self.encode_rgba(buf, width, height),
            _ => Err(ImageError::Unsupported(
                UnsupportedError::from_format_and_kind(
                    ImageFormatHint::Name("Qoi".to_string()),
                    UnsupportedErrorKind::Color(color_type.into()),
                ),
            )),
        }
    }
}
