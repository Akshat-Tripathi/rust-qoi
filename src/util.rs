use std::num::Wrapping;

use crate::consts::SEEN_PIXEL_ARRAY_SIZE;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Pixel {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Pixel {
        Pixel { r, g, b, a }
    }

    /// Get the pixel's r.
    #[must_use]
    pub(crate) fn r(&self) -> u8 {
        self.r
    }

    /// Get the pixel's g.
    #[must_use]
    pub(crate) fn g(&self) -> u8 {
        self.g
    }

    /// Get the pixel's b.
    #[must_use]
    pub(crate) fn b(&self) -> u8 {
        self.b
    }

    /// Get the pixel's a.
    #[must_use]
    pub(crate) fn a(&self) -> u8 {
        self.a
    }

    pub(crate) fn hash(&self) -> usize {
        (Wrapping(self.r()) * Wrapping(3)
            + Wrapping(self.g()) * Wrapping(5)
            + Wrapping(self.b()) * Wrapping(7)
            + Wrapping(self.a()) * Wrapping(11))
        .0 as usize
            % SEEN_PIXEL_ARRAY_SIZE
    }
}
