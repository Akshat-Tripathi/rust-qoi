use crate::chunks::{QoiChunk, OP_DIFF, OP_INDEX, OP_LUMA, OP_RGB, OP_RGBA, OP_RUN};
use crate::consts::*;
use crate::util::Pixel;

pub(crate) struct QoiCodecState<const CHANNELS: u8> {
    last_pixel: Pixel,
    previously_seen: [Pixel; SEEN_PIXEL_ARRAY_SIZE],
    run_length: u8,
}

impl<const CHANNELS: u8> QoiCodecState<CHANNELS> {
    pub(crate) fn new() -> Self {
        Self {
            last_pixel: Pixel::new(0, 0, 0, 255),
            previously_seen: [Pixel::new(0, 0, 0, 0); SEEN_PIXEL_ARRAY_SIZE],
            run_length: 0,
        }
    }

    pub(crate) fn process_pixel(&mut self, pixel: Pixel) -> Vec<(QoiChunk, bool)> {
        let is_rgb = CHANNELS == RGB_CHANNELS;

        let mut chunks = vec![];
        let hash_idx = pixel.hash();

        //1. Pixel == last pixel -> run length
        if pixel == self.last_pixel && self.run_length < MAX_RUN_LENGTH {
            self.run_length += 1;
            return self.cleanup(chunks, None, pixel);
        } else if self.run_length > 0 {
            chunks.push((QoiChunk::RUN(OP_RUN::new(self.run_length)), true));

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

        //This is the only safe place for this.
        //If we go into any of the above branches, it is guaranteed that the pixel would already
        //be in the array, so we can skip it.
        //If this was any further down, then continues would skip adding some pixels
        self.previously_seen[hash_idx] = pixel;

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
            QoiChunk::RGB(OP_RGB::new(pixel))
        } else {
            QoiChunk::RGBA(OP_RGBA::new(pixel))
        };
        self.cleanup(chunks, Some(chunk), pixel)
    }

    //This only exists because every time we need to return something from process_pixel, there's some cleanup code that needs to be run
    #[inline]
    fn cleanup(
        &mut self,
        mut chunks: Vec<(QoiChunk, bool)>,
        chunk: Option<QoiChunk>,
        pixel: Pixel,
    ) -> Vec<(QoiChunk, bool)> {
        if let Some(chunk) = chunk {
            let resolved = self.is_resolved(&chunk);
            chunks.push((chunk, resolved));
        }
        self.last_pixel = pixel;
        chunks
    }

    fn is_resolved(&self, chunk: &QoiChunk) -> bool {
        match chunk {
            QoiChunk::RGB(_) | QoiChunk::RGBA(_) => true,
            _ => false,
        }
    }

    pub(crate) fn drain(&mut self) -> Option<QoiChunk> {
        if self.run_length > 1 {
            Some(QoiChunk::RUN(OP_RUN::new(self.run_length)))
        } else {
            None
        }
    }
}
