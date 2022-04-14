use std::io::Write;
use std::num::Wrapping;

use image::error::{ImageError, ImageFormatHint, UnsupportedError, UnsupportedErrorKind};
use image::ImageEncoder;

use crate::chunks::{OP_INDEX, OP_RGB, OP_RGBA, OP_RUN, QOI_CHUNK, OP_DIFF, OP_LUMA};
use crate::util::Pixel;

const RGB_CHANNELS: u8 = 3;
const RGBA_CHANNELS: u8 = 4;
const SEEN_PIXEL_ARRAY_SIZE: usize = 64;
const MAX_RUN_LENGTH: u8 = 62;

impl Pixel {
    fn hash(&self) -> usize {
        (Wrapping(self.r()) * Wrapping(3)
            + Wrapping(self.g()) * Wrapping(5)
            + Wrapping(self.b()) * Wrapping(7)
            + Wrapping(self.a()) * Wrapping(11))
        .0 as usize
            % SEEN_PIXEL_ARRAY_SIZE
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

        let mut previously_seen: [Pixel; SEEN_PIXEL_ARRAY_SIZE] =
            [Pixel::new(0, 0, 0, 0); SEEN_PIXEL_ARRAY_SIZE];

        let mut last_pixel;
        let mut run_length = 0;

        //Just for easy bookkeeping
        let mut pixel = Pixel::new(0, 0, 0, 255);
        let mut hash_idx = 0;

        let mut n_pixels = 0;
        for chunk in buf.chunks(RGB_CHANNELS.into()) {
            n_pixels += 1;
            last_pixel = pixel;
            previously_seen[hash_idx] = pixel;

            pixel = Pixel::new(chunk[0], chunk[1], chunk[2], 255);
            hash_idx = pixel.hash();

            //1. Pixel == last pixel -> run length
            if pixel == last_pixel && run_length < MAX_RUN_LENGTH {
                run_length += 1;
                continue;
            } else if run_length > 0 {
                OP_RUN::new(run_length)
                    .encode(&mut self.w)
                    .map_err(|e| ImageError::IoError(e))?;
                if pixel == last_pixel {
                    run_length = 1;
                    continue;
                }
                run_length = 0;
            }

            //2. Pixel seen before -> index
            let looked_up_pixel = previously_seen[hash_idx];
            if looked_up_pixel == pixel {
                OP_INDEX::new(hash_idx as u8)
                    .encode(&mut self.w)
                    .map_err(|e| ImageError::IoError(e))?;
                continue;
            }

            // 3. Pixel diff > -3 but < 2 -> small diff
            if let Some(op_diff) = OP_DIFF::try_new(last_pixel, pixel) {
                op_diff.encode(&mut self.w)
                    .map_err(|e| ImageError::IoError(e))?;
                continue;
            }
            
            //4. Green pixel diff in -32..31 -> big diff
            if let Some(op_luma) = OP_LUMA::try_new(last_pixel, pixel) {
                op_luma.encode(&mut self.w)
                    .map_err(|e| ImageError::IoError(e))?;
                continue;
            }
            
            //5. Save pixel normally
            OP_RGB::new(pixel)
                .encode(&mut self.w)
                .map_err(|e| ImageError::IoError(e))?;
        }

        if run_length > 1 {
            OP_RUN::new(run_length)
                .encode(&mut self.w)
                .map_err(|e| ImageError::IoError(e))?;
        }

        self.w
            .write(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01])
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
