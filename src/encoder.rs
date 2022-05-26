use std::io::Write;

use image::error::{ImageError, ImageFormatHint, UnsupportedError, UnsupportedErrorKind};
use image::ImageEncoder;


use crate::chunks::QoiChunk;
use crate::codec::QoiCodecState;
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
