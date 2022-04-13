#![allow(non_camel_case_types)]

use std::io::Write;

pub(crate) trait QOI_CHUNK<const N: usize> {
    fn encode<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let bytes = self.to_bytes();
        writer.write_all(&bytes)
    }

    fn to_bytes(&self) -> [u8; N];
}

pub(crate) struct OP_RGB {
    r: u8,
    g: u8,
    b: u8,
}

impl OP_RGB {
    const FLAG: u8 = 0b1111_1110;
    const SIZE: usize = 4;

    pub fn new(r: u8, g: u8, b: u8) -> OP_RGB {
        OP_RGB { r, g, b }
    }

    fn matches(byte: u8) -> bool {
        byte == Self::FLAG
    }
}

impl QOI_CHUNK<{Self::SIZE}> for OP_RGB {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [Self::FLAG, self.r, self.g, self.b]
    }
}

pub(crate) struct OP_RGBA {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl OP_RGBA {
    const FLAG: u8 = 0b1111_1111;
    const SIZE: usize = 5;

    fn matches(byte: u8) -> bool {
        byte == Self::FLAG
    }
}

impl QOI_CHUNK<{Self::SIZE}> for OP_RGBA {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [Self::FLAG, self.r, self.g, self.b, self.a]
    }
}

pub(crate) struct OP_INDEX {
    index: u8,
}

impl OP_INDEX {
    const FLAG: u8 = 0b00;
    const SIZE: usize = 1;

    fn matches(byte: u8) -> bool {
        byte >> 6 == Self::FLAG
    }
}

impl QOI_CHUNK<{Self::SIZE}> for OP_INDEX {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [Self::FLAG << 6 | self.index]
    }
}

pub(crate) struct OP_DIFF {
    dr: u8,
    dg: u8,
    db: u8,
}

impl OP_DIFF {
    const FLAG: u8 = 0b01;
    const SIZE: usize = 1;

    fn matches(byte: u8) -> bool {
        byte >> 6 == Self::FLAG
    }
}

impl QOI_CHUNK<{Self::SIZE}> for OP_DIFF {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [Self::FLAG << 6 | self.dr << 4 | self.dg << 2 | self.db]
    }
}

pub(crate) struct OP_LUMA {
    diff_green: u8,
    dr_dg: u8,
    db_dg: u8,
}

impl OP_LUMA {
    const FLAG: u8 = 0b10;
    const SIZE: usize = 2;

    fn matches(byte: u8) -> bool {
        byte >> 6 == Self::FLAG
    }
}

impl QOI_CHUNK<{Self::SIZE}> for OP_LUMA {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [
            Self::FLAG << 6 | self.diff_green,
            (self.dr_dg << 4) | self.db_dg,
        ]
    }
}

pub(crate) struct OP_RUN {
    run: u8,
}

impl OP_RUN {
    const FLAG: u8 = 0b11;
    const SIZE: usize = 1;

    fn matches(byte: u8) -> bool {
        byte >> 6 == Self::FLAG
    }
}

impl QOI_CHUNK<{Self::SIZE}> for OP_RUN {
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        [Self::FLAG << 6 | self.run]
    }
}
