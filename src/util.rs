#[derive(Clone, Copy, PartialEq, Eq)]
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
}

