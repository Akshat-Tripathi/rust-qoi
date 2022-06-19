use std::collections::VecDeque;
use std::io::Write;

use image::error::{ImageError, ImageFormatHint, UnsupportedError, UnsupportedErrorKind};
use image::ImageEncoder;

use crate::chunks::{QoiChunk, OP_RUN};
use crate::codec::{ChunkState, QoiCodecState};
use crate::consts::*;
use crate::util::Pixel;

pub struct QoiEncoder<W: Write> {
    w: W,
}

impl<W: Write> QoiEncoder<W> {
    pub fn new(w: W) -> QoiEncoder<W> {
        QoiEncoder { w }
    }

    fn to_chunks<const CHANNELS: u8>(buf: &[u8]) -> (VecDeque<ChunkState>, QoiCodecState) {
        let is_rgb = CHANNELS == RGB_CHANNELS;

        let mut chunks = VecDeque::new();

        let mut codec_state = QoiCodecState::new();

        for chunk in buf.chunks(CHANNELS.into()) {
            let pixel = if is_rgb {
                Pixel::new(chunk[0], chunk[1], chunk[2], 255)
            } else {
                Pixel::new(chunk[0], chunk[1], chunk[2], chunk[3])
            };

            for chunk in codec_state.process_pixel::<CHANNELS>(pixel) {
                chunks.push_back(chunk);
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

        let splits = 2; //TODO: change this

        let mut splits = buf
            .chunks(buf.len() / splits)
            .map(Self::to_chunks::<CHANNELS>);

        // Stitch all the split up chunks back together
        let (chunks, mut global_state) = splits.next().unwrap();

        for chunk_state in chunks {
            let chunk = chunk_state.get_chunk();
            chunk
                .encode(&mut self.w)
                .map_err(|e| ImageError::IoError(e))?;
        }

        for (mut chunks, state) in splits {
            //Fix first few chunks
            let initial_state = QoiCodecState::new();

            let dummy = ChunkState::Resolved(QoiChunk::RUN(OP_RUN::new(0)));
            let first_chunk = std::mem::replace(&mut chunks[0], dummy).get_chunk();
            let pixel = initial_state.lookup_chunk(first_chunk);

            //Will return upto 2 chunks
            // 0 => continue the previous run - so we need to fix all following run length encoded chunks
            // 1 => global_state.run_length = 0 - so we can keep going normally
            // 2 => previous run ended - so we can keep going normally
            let mut temp_state = global_state.clone();
            let mut chunks1 = temp_state.process_pixel::<CHANNELS>(pixel);

            //TODO: Bad hack fix this
            if temp_state.run_length() > 0 && chunks1.len() == 1 {
                chunks1
                    .pop()
                    .unwrap()
                    .get_chunk()
                    .encode(&mut self.w)
                    .map_err(|e| ImageError::IoError(e))?;
            }

            match chunks1.len() {
                0 => {
                    //The split was in the middle of a run
                    chunks.pop_front(); //Get rid of the dummy chunk
                    let mut actual_run_length = temp_state.run_length() as u32;
                    if chunks.len() > 0 {
                        while let ChunkState::Resolved(QoiChunk::RUN(run)) = &chunks[0] {
                            actual_run_length += run.run_length() as u32;
                            chunks.pop_front();
                        }
                    }

                    while actual_run_length > MAX_RUN_LENGTH as u32 {
                        QoiChunk::RUN(OP_RUN::new(MAX_RUN_LENGTH))
                            .encode(&mut self.w)
                            .map_err(|e| ImageError::IoError(e))?;
                        actual_run_length -= MAX_RUN_LENGTH as u32;
                    }
                    if actual_run_length > 0 {
                        QoiChunk::RUN(OP_RUN::new(actual_run_length as u8))
                            .encode(&mut self.w)
                            .map_err(|e| ImageError::IoError(e))?;
                    }
                }
                1 => {
                    //Normal split
                    chunks[0] = chunks1.pop().unwrap();
                }
                2 => {
                    //The split ended a run
                    chunks[0] = chunks1.pop().unwrap();

                    let run = chunks1.pop().unwrap().get_chunk();
                    run.encode(&mut self.w)
                        .map_err(|e| ImageError::IoError(e))?;
                }
                _ => unreachable!(),
            };
            for chunk_state in chunks {
                let chunk = match chunk_state {
                    ChunkState::Resolved(chunk) => chunk,
                    ChunkState::Unresolved(chunk, pixel) => {
                        global_state.lookup_pixel(&pixel).unwrap_or(chunk)
                    }
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
