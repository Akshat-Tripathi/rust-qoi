use std::io::Write;
use std::vec;

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

    fn to_chunks<const CHANNELS: u8>(
        buf: &[u8],
    ) -> (
        Vec<QoiChunk>,
        Vec<usize>,
        QoiCodecState<CHANNELS>
    ) {
        let is_rgb = CHANNELS == RGB_CHANNELS;

        let mut unresolved_chunk_indices: Vec<usize> = Vec::new();
        let mut chunks = Vec::new();

        let mut codec_state = QoiCodecState::new();

        for chunk in buf.chunks(CHANNELS.into()) {
            let pixel = if is_rgb {
                Pixel::new(chunk[0], chunk[1], chunk[2], 255)
            } else {
                Pixel::new(chunk[0], chunk[1], chunk[2], chunk[3])
            };

            for chunk in codec_state.process_pixel(pixel) {
                match chunk {
                    QoiChunk::RGB(_) |  QoiChunk::RGBA(_) => {unresolved_chunk_indices.push(chunks.len());},
                    _ => {}
                }
                chunks.push(chunk);
            }            
        }

        if let Some(chunk) = codec_state.drain() {
            chunks.push(chunk);
        }

        (
            chunks,
            unresolved_chunk_indices,
            codec_state
        )
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

        // let splits = 8; //TODO: change this

        // let mut splits = buf
        //     .chunks(buf.len() / splits)
        //     .map(Self::to_chunks::<CHANNELS>);

        //Stitch all the split up chunks back together
        // let (mut all_chunks, mut last_pixel, mut previously_seen, _) = splits.next().unwrap();

        // for (mut chunks, last_pixel1, previously_seen1, unresolved_chunks) in splits {
        //     last_pixel = last_pixel1;
        //     previously_seen = previously_seen1;
        //     all_chunks.append(&mut chunks);
        // }

        let (chunks, _, _) = Self::to_chunks::<CHANNELS>(buf);
        for chunk in chunks {
            chunk.encode(&mut self.w).map_err(|e| ImageError::IoError(e))?;
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

struct QoiCodecState<const CHANNELS: u8> {
    last_pixel: Pixel,
    previously_seen: [Pixel; SEEN_PIXEL_ARRAY_SIZE],
    run_length: u8,
}

impl<const CHANNELS: u8> QoiCodecState<CHANNELS> {
    fn new() -> Self {
        Self {
            last_pixel: Pixel::new(0, 0, 0, 255),
            previously_seen: [Pixel::new(0, 0, 0, 0); SEEN_PIXEL_ARRAY_SIZE],
            run_length: 0,
        }
    }

    fn process_pixel(&mut self, pixel: Pixel) -> Vec<QoiChunk> {
        let is_rgb = CHANNELS == RGB_CHANNELS;

        // defer!{
        //     self.last_pixel = pixel;
        // }

        let mut chunks = vec![];

        let hash_idx = pixel.hash();

        //1. Pixel == last pixel -> run length
        if pixel == self.last_pixel && self.run_length < MAX_RUN_LENGTH {
            self.run_length += 1;
            self.last_pixel = pixel;
            return vec![];
        } else if self.run_length > 0 {
            chunks.push(QoiChunk::RUN(OP_RUN::new(self.run_length)));
            
            if pixel == self.last_pixel {
                self.run_length = 1;
                self.last_pixel = pixel;
                return chunks;
            }
            
            self.run_length = 0;
        }

        //2. Pixel seen before -> index
        let looked_up_pixel = self.previously_seen[hash_idx];
        if looked_up_pixel == pixel {
            self.last_pixel = pixel;
            chunks.push(QoiChunk::INDEX(OP_INDEX::new(hash_idx as u8)));
            return chunks;
        }

        //This is the only safe place for this.
        //If we go into any of the above branches, it is guaranteed that the pixel would already
        //be in the array, so we can skip it.
        //If this was any further down, then continues would skip adding some pixels
        self.previously_seen[hash_idx] = pixel;

        // 3. Pixel diff > -3 but < 2 -> small diff
        if let Some(op_diff) = OP_DIFF::try_new(self.last_pixel, pixel) {
            self.last_pixel = pixel;
            chunks.push(QoiChunk::DIFF(op_diff));
            return chunks;
        }

        //4. Green pixel diff in -32..31 -> big diff
        if let Some(op_luma) = OP_LUMA::try_new(self.last_pixel, pixel) {
            self.last_pixel = pixel;
            chunks.push(QoiChunk::LUMA(op_luma));
            return chunks;
        }

        //5. Save pixel normally
        let chunk = if pixel.a() == self.last_pixel.a() || is_rgb {
            QoiChunk::RGB(OP_RGB::new(pixel))
        } else {
            QoiChunk::RGBA(OP_RGBA::new(pixel))
        };
        self.last_pixel = pixel;
        chunks.push(chunk);
        chunks
    }

    fn drain(&mut self) -> Option<QoiChunk> {
        if self.run_length > 1 {
            Some(QoiChunk::RUN(OP_RUN::new(self.run_length)))
        } else {
            None
        }
    }
}
