use std::io::Write;

use image::error::{ImageError, ImageFormatHint, UnsupportedError, UnsupportedErrorKind};
use image::ImageEncoder;

const RGB_CHANNELS: u8 = 3;
const RGBA_CHANNELS: u8 = 4;
const SEEN_PIXEL_ARRAY_SIZE: u8 = 64;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Pixel {
    fn hash(&self) -> u8 {
        (self.r * 3 + self.g * 5 + self.b * 7 + self.a * 11) % SEEN_PIXEL_ARRAY_SIZE
    }
}


pub struct QoiEncoder<W: Write> {
    w: W,
}

impl<W: Write> QoiEncoder<W> {
    pub fn new(w: W) -> QoiEncoder<W> {
        QoiEncoder { w }
    }

    fn encode_rgb(mut self, buf: &[u8], width: u32, height: u32) -> image::ImageResult<()> {
        //TODO: check if colour_space is actually 0
        self.write_header(width, height, RGB_CHANNELS, 0)
            .map_err(|e| ImageError::IoError(e))?;

        let previously_seen: [Pixel; SEEN_PIXEL_ARRAY_SIZE as usize] = [Pixel {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
            SEEN_PIXEL_ARRAY_SIZE as usize];

        let mut last_pixel = Pixel {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        };
        for pixel in buf.chunks(RGB_CHANNELS.into()) {
            let pixel = Pixel {
                r: pixel[0],
                g: pixel[1],
                b: pixel[2],
                a: 255,
            };            
        }

        self.w.write(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01])
            .map_err(|e| ImageError::IoError(e))?;
        Ok(())
    }
    fn encode_rgba(mut self, buf: &[u8], width: u32, height: u32) -> image::ImageResult<()> {
        //TODO: check if colour_space is actually 0
        self.write_header(width, height, RGBA_CHANNELS, 0)
            .map_err(|e| ImageError::IoError(e))?;
        Ok(())
    }

    fn write_header(
        &mut self,
        width: u32,
        height: u32,
        channels: u8,
        color_space: u8,
    ) -> std::io::Result<()> {
        self.w.write_all("qoif".as_bytes())?;
        self.w.write_all(&width.to_be_bytes())?;
        self.w.write_all(&height.to_be_bytes())?;
        self.w.write_all(&[channels])?;
        self.w.write_all(&[color_space])?;
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
