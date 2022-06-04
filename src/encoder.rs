use std::io::Write;

use image::error::{ImageError, ImageFormatHint, UnsupportedError, UnsupportedErrorKind};
use image::ImageEncoder;

use crate::chunks::{QoiChunk, OP_RUN};
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

    fn to_chunks<const CHANNELS: u8>(buf: &[u8]) -> (Vec<(QoiChunk, bool)>, QoiCodecState) {
        let is_rgb = CHANNELS == RGB_CHANNELS;

        let mut chunks = Vec::new();

        let mut codec_state = QoiCodecState::new();

        for chunk in buf.chunks(CHANNELS.into()) {
            let pixel = if is_rgb {
                Pixel::new(chunk[0], chunk[1], chunk[2], 255)
            } else {
                Pixel::new(chunk[0], chunk[1], chunk[2], chunk[3])
            };

            for chunk in codec_state.process_pixel::<CHANNELS>(pixel) {
                chunks.push(chunk);
            }
        }

        (chunks, codec_state)
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

        let splits = 1; //TODO: change this

        let mut splits = buf
            .chunks(buf.len() / splits)
            .map(Self::to_chunks::<CHANNELS>);

        // Stitch all the split up chunks back together
        let (chunks, mut global_state) = splits.next().unwrap();

        for (chunk, _) in chunks {
            chunk
                .encode(&mut self.w)
                .map_err(|e| ImageError::IoError(e))?;
        }

        for (mut chunks, state) in splits {
            //Fix first few chunks
            let initial_state = QoiCodecState::new();

            let dummy = (QoiChunk::RUN(OP_RUN::new(0)), false);
            let (first_chunk, _) = std::mem::replace(&mut chunks[0], dummy);
            let pixel = initial_state.lookup_chunk(first_chunk);

            //Will return upto 2 chunks
            // 0 => continue the previous run - so we need to fix all following run length encoded chunks
            // 1 => global_state.run_length = 0 - so we can keep going normally
            // 2 => previous run ended - so we can keep going normally
            let mut chunks1 = global_state.process_pixel::<CHANNELS>(pixel);

            match chunks1.len() {
                0 => {
                    chunks.pop(); //Get rid of the dummy chunk
                    let mut actual_run_length = global_state.run_length();
                    while let (QoiChunk::RUN(run), true) = &chunks[0] {
                        actual_run_length += run.run_length();
                        chunks.pop();
                    }

                    while actual_run_length > MAX_RUN_LENGTH {
                        QoiChunk::RUN(OP_RUN::new(MAX_RUN_LENGTH))
                            .encode(&mut self.w)
                            .map_err(|e| ImageError::IoError(e))?;
                        actual_run_length -= MAX_RUN_LENGTH;
                    }
                    if actual_run_length > 0 {
                        QoiChunk::RUN(OP_RUN::new(actual_run_length))
                            .encode(&mut self.w)
                            .map_err(|e| ImageError::IoError(e))?;
                    }
                }
                1 => {
                    chunks[0] = chunks1.pop().unwrap();
                }
                2 => {
                    let (run, _) = chunks1.pop().unwrap();
                    run.encode(&mut self.w)
                        .map_err(|e| ImageError::IoError(e))?;

                    chunks[0] = chunks1.pop().unwrap();
                }
                _ => unreachable!(),
            };

            for (chunk, resolved) in chunks {
                let chunk = if !resolved {
                    todo!();
                    chunk
                } else {
                    chunk
                };
                chunk
                    .encode(&mut self.w)
                    .map_err(|e| ImageError::IoError(e))?;
            }

            global_state.merge(state);
        }

        if let Some((chunk, _)) = global_state.drain() {
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
