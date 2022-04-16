#![allow(non_camel_case_types)]

use std::fmt::Debug;
use std::io::Write;
use std::iter::Peekable;
use std::num::Wrapping;

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

    fn try_decode(buf: &mut Peekable<impl Iterator<Item = u8>>) -> Option<Self>
    where
        Self: Sized,
    {
        let flag = *buf.peek()?;
        if Self::matches(flag) {
            let bytes = buf.take(N).collect::<Vec<u8>>();
            if bytes.len() != N {
                return None
            }
            Some(Self::from_bytes(&bytes))
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
    run: u8,
}

impl OP_RUN {
    const FLAG: u8 = 0b11;
    const SIZE: usize = 1;

    pub fn new(run: u8) -> OP_RUN {
        OP_RUN { run }
    }
}

impl QOI_CHUNK<{ Self::SIZE }> for OP_RUN {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [Self::FLAG << 6 | (self.run - 1)] //Store with bias -1
    }

    fn matches(byte: u8) -> bool {
        byte >> 6 == Self::FLAG
    }

    fn from_bytes(buffer: &[u8]) -> Self {
        OP_RUN {
            run: (buffer[0] & 0b0011_1111) + 1, //Restore with bias +1
        }
    }
}

fn biased_sub(a: u8, b: u8, bias: u8) -> u8 {
    (Wrapping(a as i16 - b as i16).0 + bias as i16) as u8
}
