#![allow(non_camel_case_types)]

//Leaving this in for now, in case it's useful for decoding
pub(crate) enum QOI_CHUNK {
    OP_RGB(OP_RGB),
    OP_RGBA(OP_RGBA),
    OP_INDEX(OP_INDEX),
    OP_DIFF(OP_DIFF),
    OP_LUMA(OP_LUMA),
    OP_RUN(OP_RUN)
}

pub(crate) struct OP_RGB {
    r: u8,
    g: u8,
    b: u8,
}

impl OP_RGB {
    const FLAG: u8 = 0b1111_1110;
    const SIZE: u8 = 4;

    fn matches(byte: u8) -> bool {
        byte == Self::FLAG
    }

    fn encode(self) -> [u8; Self::SIZE as usize] {
        [
            Self::FLAG,
            self.r,
            self.g,
            self.b,
        ]
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
    const SIZE: u8 = 5;

    fn matches(byte: u8) -> bool {
        byte == Self::FLAG
    }

    fn encode(self) -> [u8; Self::SIZE as usize] {
        [
            Self::FLAG,
            self.r,
            self.g,
            self.b,
            self.a
        ]
    }
}

pub(crate) struct OP_INDEX {
    index: u8,   
}

impl OP_INDEX {
    const FLAG: u8 = 0b00;
    const SIZE: u8 = 1;

    fn matches(byte: u8) -> bool {
        byte >> 6 == Self::FLAG
    }

    fn encode(self) -> [u8; Self::SIZE as usize] {
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
    const SIZE: u8 = 1;

    fn matches(byte: u8) -> bool {
        byte >> 6 == Self::FLAG
    }

    fn encode(self) -> [u8; Self::SIZE as usize] {
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
    const SIZE: u8 = 2;

    fn matches(byte: u8) -> bool {
        byte >> 6 == Self::FLAG
    }

    fn encode(self) -> [u8; Self::SIZE as usize] {
        [Self::FLAG << 6 | self.diff_green, (self.dr_dg << 4) | self.db_dg]
    }
}

pub(crate) struct OP_RUN {
    run: u8
}

impl OP_RUN {
    const FLAG: u8 = 0b11;
    const SIZE: u8 = 1;
    
    fn matches(byte: u8) -> bool {
        byte >> 6 == Self::FLAG
    }

    fn encode(self) -> [u8; Self::SIZE as usize] {
        [Self::FLAG << 6 | self.run]
    }
}


