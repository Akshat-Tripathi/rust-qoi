use std::io::Write;

use image::error::{ImageError, ImageFormatHint, UnsupportedError, UnsupportedErrorKind};
use image::ImageEncoder;

use crate::chunks::{QoiChunk, OP_DIFF, OP_INDEX, OP_LUMA, OP_RGB, OP_RGBA, OP_RUN};
use crate::consts::*;
use crate::util::Pixel;

pub struct QoiEncoder<W: Write> {
    w: W,
}

impl<W: Write> QoiEncoder<W> {
    pub fn new(w: W) -> QoiEncoder<W> {
        QoiEncoder { w }
    }

    fn to_chunks<const CHANNELS: u8>(buf: &[u8]) -> Vec<QoiChunk> {
        let is_rgb = CHANNELS == RGB_CHANNELS;
        let mut previously_seen: [Pixel; SEEN_PIXEL_ARRAY_SIZE] =
            [Pixel::new(0, 0, 0, 0); SEEN_PIXEL_ARRAY_SIZE];

        let mut last_pixel;
        let mut run_length = 0;

        //Just for easy bookkeeping
        let mut pixel = Pixel::new(0, 0, 0, 255);
        let mut hash_idx;

        let mut chunks = Vec::new();

        for chunk in buf.chunks(CHANNELS.into()) {
            last_pixel = pixel;

            if is_rgb {
                pixel = Pixel::new(chunk[0], chunk[1], chunk[2], 255);
            } else {
                pixel = Pixel::new(chunk[0], chunk[1], chunk[2], chunk[3]);
            }
            hash_idx = pixel.hash();

            //1. Pixel == last pixel -> run length
            if pixel == last_pixel && run_length < MAX_RUN_LENGTH {
                run_length += 1;
                continue;
            } else if run_length > 0 {
                chunks.push(QoiChunk::RUN(OP_RUN::new(run_length)));
                if pixel == last_pixel {
                    run_length = 1;
                    continue;
                }
                run_length = 0;
            }

            //2. Pixel seen before -> index
            let looked_up_pixel = previously_seen[hash_idx];
            if looked_up_pixel == pixel {
                chunks.push(QoiChunk::INDEX(OP_INDEX::new(hash_idx as u8)));
                continue;
            }

            //This is the only safe place for this.
            //If we go into any of the above branches, it is guaranteed that the pixel would already
            //be in the array, so we can skip it.
            //If this was any further down, then continues would skip adding some pixels
            previously_seen[hash_idx] = pixel;

            // 3. Pixel diff > -3 but < 2 -> small diff
            if let Some(op_diff) = OP_DIFF::try_new(last_pixel, pixel) {
                chunks.push(QoiChunk::DIFF(op_diff));
                continue;
            }

            //4. Green pixel diff in -32..31 -> big diff
            if let Some(op_luma) = OP_LUMA::try_new(last_pixel, pixel) {
                chunks.push(QoiChunk::LUMA(op_luma));
                continue;
            }

            //5. Save pixel normally
            if pixel.a() == last_pixel.a() || is_rgb {
                chunks.push(QoiChunk::RGB(OP_RGB::new(pixel)))
            } else {
                chunks.push(QoiChunk::RGBA(OP_RGBA::new(pixel)))
            }
        }

        if run_length > 1 {
            chunks.push(QoiChunk::RUN(OP_RUN::new(run_length)));
        }

        chunks
    }

    fn encode<const CHANNELS: u8>(
        mut self,
        buf: &[u8],
        width: u32,
        height: u32,
    ) -> image::ImageResult<()> {
        //TODO: check if colour_space is actually 0
        self.write_header(width, height, CHANNELS, 0)
            .map_err(|e| ImageError::IoError(e))?;

        for chunk in Self::to_chunks::<CHANNELS>(buf) {
            chunk
                .encode(&mut self.w)
                .map_err(|e| ImageError::IoError(e))?;
        }

        self.w
            .write(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01])
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
            image::ColorType::Rgb8 => self.encode::<RGB_CHANNELS>(buf, width, height),
            image::ColorType::Rgba8 => self.encode::<RGBA_CHANNELS>(buf, width, height),
            _ => Err(ImageError::Unsupported(
                UnsupportedError::from_format_and_kind(
                    ImageFormatHint::Name("Qoi".to_string()),
                    UnsupportedErrorKind::Color(color_type.into()),
                ),
            )),
        }
    }
}
