use std::convert::TryInto;

use crate::chunks::{QoiChunk, OP_DIFF, OP_INDEX, OP_LUMA, OP_RGB, OP_RGBA, OP_RUN};
use crate::consts::*;
use crate::util::Pixel;

#[derive(Clone, Copy, Debug)]
pub(crate) struct QoiCodecState {
    last_pixel: Pixel,
    previously_seen: [Pixel; SEEN_PIXEL_ARRAY_SIZE],
    run_length: u8,
    modified: u64, //This must be the same as SEEN_PIXEL_ARRAY_SIZE
}

impl QoiCodecState {
    pub(crate) fn new() -> Self {
        Self {
            last_pixel: Pixel::new(0, 0, 0, 255),
            previously_seen: [Pixel::new(0, 0, 0, 0); SEEN_PIXEL_ARRAY_SIZE],
            run_length: 0,
            modified: 0,
        }
    }

    pub(crate) fn process_pixel<const CHANNELS: u8>(
        &mut self,
        pixel: Pixel,
    ) -> Vec<ChunkState> {
        let is_rgb = CHANNELS == RGB_CHANNELS;

        let mut chunks = vec![];
        let hash_idx = pixel.hash();

        //1. Pixel == last pixel -> run length
        if pixel == self.last_pixel && self.run_length < MAX_RUN_LENGTH {
            self.run_length += 1;
            return self.cleanup(chunks, None, pixel);
        } else if self.run_length > 0 {
            chunks.push(ChunkState::Resolved(QoiChunk::RUN(OP_RUN::new(self.run_length))));

            if pixel == self.last_pixel {
                self.run_length = 1;
                return self.cleanup(chunks, None, pixel);
            }

            self.run_length = 0;
        }

        //2. Pixel seen before -> index
        let looked_up_pixel = self.previously_seen[hash_idx];
        if looked_up_pixel == pixel {
            return self.cleanup(
                chunks,
                Some(QoiChunk::INDEX(OP_INDEX::new(hash_idx as u8))),
                pixel,
            );
        }

        // 3. Pixel diff > -3 but < 2 -> small diff
        if let Some(op_diff) = OP_DIFF::try_new(self.last_pixel, pixel) {
            return self.cleanup(chunks, Some(QoiChunk::DIFF(op_diff)), pixel);
        }

        //4. Green pixel diff in -32..31 -> big diff
        if let Some(op_luma) = OP_LUMA::try_new(self.last_pixel, pixel) {
            return self.cleanup(chunks, Some(QoiChunk::LUMA(op_luma)), pixel);
        }

        //5. Save pixel normally
        let chunk = if pixel.a() == self.last_pixel.a() || is_rgb {
            QoiChunk::RGB(OP_RGB::new(pixel, pixel.a()))
        } else {
            QoiChunk::RGBA(OP_RGBA::new(pixel))
        };
        self.cleanup(chunks, Some(chunk), pixel)
    }

    //This only exists because every time we need to return something from process_pixel, there's some cleanup code that needs to be run
    #[inline]
    fn cleanup(
        &mut self,
        mut chunks: Vec<ChunkState>,
        chunk: Option<QoiChunk>,
        pixel: Pixel,
    ) -> Vec<ChunkState> {
        if let Some(chunk) = chunk {
            chunks.push(if self.is_resolved(&chunk, &pixel) {
                ChunkState::Resolved(chunk)
            } else {
                ChunkState::Unresolved(chunk, pixel.clone())
            });
        }
        let hash_idx = pixel.hash();
        self.previously_seen[hash_idx] = pixel;
        self.modified |= 1 << hash_idx;
        self.last_pixel = pixel;
        chunks
    }

    fn is_resolved(&self, chunk: &QoiChunk, pixel: &Pixel) -> bool {
        match chunk {
            QoiChunk::INDEX(_) => true,
            QoiChunk::RUN(_) => true,
            _ => self.modified(pixel.hash()),
        }
    }

    //Will optionally return a run length encoded chunk
    pub(crate) fn drain(&mut self) -> Option<(QoiChunk, bool)> {
        if self.run_length > 1 {
            Some((QoiChunk::RUN(OP_RUN::new(self.run_length)), true))
        } else {
            None
        }
    }

    pub(crate) fn modified(&self, hash_idx: usize) -> bool {
        (self.modified & (1 << hash_idx)) > 0
    }

    //Assumes other is further down in parsing than self
    pub(crate) fn merge(&mut self, other: QoiCodecState) {
        self.last_pixel = other.last_pixel;
        self.run_length = other.run_length;

        for (i, px) in other.previously_seen.iter().enumerate() {
            if other.modified(i) {
                self.previously_seen[i] = px.to_owned();
            }
        }
        self.modified |= other.modified;
    }

    pub(crate) fn last_pixel(&self) -> Pixel {
        self.last_pixel
    }

    pub(crate) fn run_length(&self) -> u8 {
        self.run_length
    }

    pub(crate) fn lookup_pixel(&self, pixel: &Pixel) -> Option<QoiChunk> {
        let hash_idx = pixel.hash();
        if self.modified(hash_idx) && self.previously_seen[hash_idx] == *pixel {
            Some(QoiChunk::INDEX(OP_INDEX::new(hash_idx.try_into().unwrap())))
        } else {
            None
        }
    }
}

//This covers all methods related to decoding
impl QoiCodecState {
    pub(crate) fn lookup_chunk(&self, chunk: QoiChunk) -> Pixel {
        match chunk {
            QoiChunk::RGB(chunk) => chunk.into(),
            QoiChunk::RGBA(chunk) => chunk.into(),
            QoiChunk::RUN(chunk) => (self.last_pixel, chunk).into(),
            QoiChunk::LUMA(chunk) => (self.last_pixel, chunk).into(),
            QoiChunk::DIFF(chunk) => (self.last_pixel, chunk).into(),
            QoiChunk::INDEX(chunk) => (self.previously_seen, chunk).into(),
        }
    }

    pub(crate) fn process_chunk(&mut self, chunk: QoiChunk) -> (Pixel, usize) {
        if let QoiChunk::RUN(chunk) = chunk {
            (self.last_pixel, chunk.run_length() as usize)
        } else {
            self.last_pixel = self.lookup_chunk(chunk);
            self.previously_seen[self.last_pixel.hash()] = self.last_pixel;
            (self.last_pixel, 1)
        }
    }
}

//TODO: Better name
pub(crate) enum ChunkState {
    Resolved(QoiChunk),
    Unresolved(QoiChunk, Pixel)
}

impl ChunkState {
    pub(crate) fn get_chunk(self) -> QoiChunk {
        match self {
            ChunkState::Resolved(chunk) => chunk,
            ChunkState::Unresolved(chunk, _) => chunk,
        }
    }
}