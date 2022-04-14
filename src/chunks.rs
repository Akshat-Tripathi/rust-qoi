#![allow(non_camel_case_types)]

use std::fmt::Debug;
use std::io::Write;
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

    fn to_bytes(&self) -> [u8; N];
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

    fn matches(byte: u8) -> bool {
        byte == Self::FLAG
    }
}

impl QOI_CHUNK<{ Self::SIZE }> for OP_RGB {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [Self::FLAG, self.r, self.g, self.b]
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

    fn matches(byte: u8) -> bool {
        byte == Self::FLAG
    }
}

impl QOI_CHUNK<{ Self::SIZE }> for OP_RGBA {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [Self::FLAG, self.r, self.g, self.b, self.a]
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

    fn matches(byte: u8) -> bool {
        byte >> 6 == Self::FLAG
    }
}

impl QOI_CHUNK<{ Self::SIZE }> for OP_INDEX {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [Self::FLAG << 6 | self.index]
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
        let dr = biased_sub(prev.r(), curr.r(), 2);
        let dg = biased_sub(prev.g(), curr.g(), 2);
        let db = biased_sub(prev.b(), curr.b(), 2);

        if dr < 4 && dg < 4 && db < 4 {
            Some(OP_DIFF { dr, dg, db })
        } else {
            None
        }
    }

    fn matches(byte: u8) -> bool {
        byte >> 6 == Self::FLAG
    }
}

impl QOI_CHUNK<{ Self::SIZE }> for OP_DIFF {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [Self::FLAG << 6 | self.dr << 4 | self.dg << 2 | self.db]
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
        let dr = biased_sub(prev.r(), curr.r(), 32);
        let dg = biased_sub(prev.g(), curr.g(), 32);
        let db = biased_sub(prev.b(), curr.b(), 32);

        let dr_dg = biased_sub(dr, dg, 8);
        let db_dg = biased_sub(db, dg, 8);

        if dg < 64 && dr_dg < 16 && db_dg < 16 {
            Some(OP_LUMA { dg, dr_dg, db_dg })
        } else {
            None
        }
    }

    fn matches(byte: u8) -> bool {
        byte >> 6 == Self::FLAG
    }
}

impl QOI_CHUNK<{ Self::SIZE }> for OP_LUMA {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [Self::FLAG << 6 | self.dg, (self.dr_dg << 4) | self.db_dg]
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

    fn matches(byte: u8) -> bool {
        byte >> 6 == Self::FLAG
    }
}

impl QOI_CHUNK<{ Self::SIZE }> for OP_RUN {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [Self::FLAG << 6 | (self.run - 1)] //Store with bias -1
    }
}

fn biased_sub(a: u8, b: u8, bias: u8) -> u8 {
    (Wrapping(a as i16 - b as i16).0 + bias as i16) as u8
}
