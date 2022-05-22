#![allow(non_camel_case_types)]

use std::fmt::Debug;
use std::io::{Bytes, Read, Write};
use std::iter::Peekable;
use std::num::Wrapping;

use crate::consts::SEEN_PIXEL_ARRAY_SIZE;
use crate::util::Pixel;

pub(crate) trait QOI_CHUNK<const N: usize>
where
    Self: Debug,
{
    fn encode<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        #[cfg(test)]
        println!("{:?}", self);

        let bytes = self.to_bytes();
        writer.write_all(&bytes)
    }

    fn try_decode<R: Read>(buf: &mut Peekable<Bytes<R>>) -> Option<Self>
    where
        Self: Sized,
    {
        let flag = *buf.peek()?.as_ref().ok()?;
        if Self::matches(flag) {
            let bytes = buf.take(N).collect::<Vec<std::io::Result<u8>>>();
            if bytes.len() != N {
                return None;
            }
            
            let bytes = bytes
            .iter()
            .map(|r| *r.as_ref().unwrap())
            .collect::<Vec<u8>>();
            
            let chunk = Self::from_bytes(&bytes);
            
            #[cfg(test)]
            println!("{:?}", chunk);
            
            Some(chunk)
        } else {
            None
        }
    }

    fn matches(byte: u8) -> bool;

    fn to_bytes(&self) -> [u8; N];
    fn from_bytes(buffer: &[u8]) -> Self;
}

#[derive(Debug)]
pub(crate) struct OP_RGB {
    r: u8,
    g: u8,
    b: u8,
}

impl OP_RGB {
    const FLAG: u8 = 0b1111_1110;
    const SIZE: usize = 4;

    pub fn new(pixel: Pixel) -> OP_RGB {
        OP_RGB {
            r: pixel.r(),
            g: pixel.g(),
            b: pixel.b(),
        }
    }
}

impl From<OP_RGB> for Pixel {
    fn from(chunk: OP_RGB) -> Self {
        Pixel::new(chunk.r, chunk.g, chunk.b, 255)
    }
}

impl QOI_CHUNK<{ Self::SIZE }> for OP_RGB {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [Self::FLAG, self.r, self.g, self.b]
    }

    fn matches(byte: u8) -> bool {
        byte == Self::FLAG
    }

    fn from_bytes(buffer: &[u8]) -> Self {
        OP_RGB {
            r: buffer[1],
            g: buffer[2],
            b: buffer[3],
        }
    }
}

#[derive(Debug)]
pub(crate) struct OP_RGBA {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl OP_RGBA {
    const FLAG: u8 = 0b1111_1111;
    const SIZE: usize = 5;

    pub fn new(pixel: Pixel) -> OP_RGBA {
        OP_RGBA {
            r: pixel.r(),
            g: pixel.g(),
            b: pixel.b(),
            a: pixel.a(),
        }
    }
}

impl From<OP_RGBA> for Pixel {
    fn from(chunk: OP_RGBA) -> Self {
        Pixel::new(chunk.r, chunk.g, chunk.b, chunk.a)
    }
}

impl QOI_CHUNK<{ Self::SIZE }> for OP_RGBA {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [Self::FLAG, self.r, self.g, self.b, self.a]
    }

    fn matches(byte: u8) -> bool {
        byte == Self::FLAG
    }

    fn from_bytes(buffer: &[u8]) -> Self {
        OP_RGBA {
            r: buffer[1],
            g: buffer[2],
            b: buffer[3],
            a: buffer[4],
        }
    }
}

#[derive(Debug)]
pub(crate) struct OP_INDEX {
    index: u8,
}

impl OP_INDEX {
    const FLAG: u8 = 0b00;
    const SIZE: usize = 1;

    pub fn new(index: u8) -> OP_INDEX {
        OP_INDEX { index }
    }
}

impl From<([Pixel; SEEN_PIXEL_ARRAY_SIZE], OP_INDEX)> for Pixel {
    fn from((arr, chunk): ([Pixel; SEEN_PIXEL_ARRAY_SIZE], OP_INDEX)) -> Self {
        arr[chunk.index as usize]
    }
}

impl QOI_CHUNK<{ Self::SIZE }> for OP_INDEX {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [Self::FLAG << 6 | self.index]
    }

    fn matches(byte: u8) -> bool {
        byte >> 6 == Self::FLAG
    }

    fn from_bytes(buffer: &[u8]) -> Self {
        OP_INDEX {
            index: buffer[0] & 0b011_1111,
        }
    }
}

#[derive(Debug)]
pub(crate) struct OP_DIFF {
    dr: u8,
    dg: u8,
    db: u8,
}

impl OP_DIFF {
    const FLAG: u8 = 0b01;
    const SIZE: usize = 1;

    pub fn try_new(prev: Pixel, curr: Pixel) -> Option<OP_DIFF> {
        if prev.a() != curr.a() {
            return None;
        }
        let dr = biased_sub(curr.r(), prev.r(), 2);
        let dg = biased_sub(curr.g(), prev.g(), 2);
        let db = biased_sub(curr.b(), prev.b(), 2);

        if dr < 4 && dg < 4 && db < 4 {
            Some(OP_DIFF { dr, dg, db })
        } else {
            None
        }
    }
}

impl From<(Pixel, OP_DIFF)> for Pixel {
    fn from((px, chunk): (Pixel, OP_DIFF)) -> Self {
        Pixel::new(
            biased_add(px.r(), chunk.dr, 2),
            biased_add(px.g(), chunk.dg, 2),
            biased_add(px.b(), chunk.db, 2),
            px.a(),
        )
    }
}

impl QOI_CHUNK<{ Self::SIZE }> for OP_DIFF {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [Self::FLAG << 6 | self.dr << 4 | self.dg << 2 | self.db]
    }

    fn matches(byte: u8) -> bool {
        byte >> 6 == Self::FLAG
    }

    fn from_bytes(buffer: &[u8]) -> Self {
        OP_DIFF {
            dr: (buffer[0] & 0b0011_0000) >> 4,
            dg: (buffer[0] & 0b0000_1100) >> 2,
            db: buffer[0] & 0b0000_0011,
        }
    }
}

#[derive(Debug)]
pub(crate) struct OP_LUMA {
    dg: u8,
    dr_dg: u8,
    db_dg: u8,
}

impl OP_LUMA {
    const FLAG: u8 = 0b10;
    const SIZE: usize = 2;

    pub fn try_new(prev: Pixel, curr: Pixel) -> Option<OP_LUMA> {
        if prev.a() != curr.a() {
            return None;
        }
        let dr = biased_sub(curr.r(), prev.r(), 32);
        let dg = biased_sub(curr.g(), prev.g(), 32);
        let db = biased_sub(curr.b(), prev.b(), 32);

        let dr_dg = biased_sub(dr, dg, 8);
        let db_dg = biased_sub(db, dg, 8);

        if dg < 64 && dr_dg < 16 && db_dg < 16 {
            Some(OP_LUMA { dg, dr_dg, db_dg })
        } else {
            None
        }
    }
}

impl From<(Pixel, OP_LUMA)> for Pixel {
    fn from((px, chunk): (Pixel, OP_LUMA)) -> Self {
        let dr = biased_add(chunk.dr_dg, chunk.dg, 8);
        let dg = chunk.dg;
        let db = biased_add(chunk.db_dg, chunk.dg, 8);

        Pixel::new(
            biased_add(px.r(), dr, 32),
            biased_add(px.g(), dg, 32),
            biased_add(px.b(), db, 32),
            px.a()
        )
    }
}

impl QOI_CHUNK<{ Self::SIZE }> for OP_LUMA {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [Self::FLAG << 6 | self.dg, (self.dr_dg << 4) | self.db_dg]
    }

    fn matches(byte: u8) -> bool {
        byte >> 6 == Self::FLAG
    }

    fn from_bytes(buffer: &[u8]) -> Self {
        OP_LUMA {
            dg: buffer[0] & 0b0011_1111,
            dr_dg: buffer[1] >> 4,
            db_dg: buffer[1] & 0b0000_1111,
        }
    }
}

#[derive(Debug)]
pub(crate) struct OP_RUN {
    run_length: u8,
}

impl OP_RUN {
    const FLAG: u8 = 0b11;
    const SIZE: usize = 1;

    pub fn new(run: u8) -> OP_RUN {
        OP_RUN { run_length: run }
    }

    #[must_use]
    pub(crate) fn run_length(&self) -> u8 {
        self.run_length
    }
}

impl QOI_CHUNK<{ Self::SIZE }> for OP_RUN {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [Self::FLAG << 6 | (self.run_length - 1)] //Store with bias -1
    }
    fn matches(byte: u8) -> bool {
        byte >> 6 == Self::FLAG
    }

    fn from_bytes(buffer: &[u8]) -> Self {
        OP_RUN {
            run_length: (buffer[0] & 0b0011_1111) + 1, //Restore with bias +1
        }
    }
}

fn biased_sub(a: u8, b: u8, bias: u8) -> u8 {
    (Wrapping(a as i16 - b as i16).0 + bias as i16) as u8
}

fn biased_add(a: u8, b: u8, bias: u8) -> u8 {
    (Wrapping(a as i16 + b as i16).0 - bias as i16) as u8
}