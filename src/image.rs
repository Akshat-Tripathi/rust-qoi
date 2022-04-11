pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<Pixel>,
}

impl Image {
    fn new(width: u32, height: u32, pixels: Vec<Pixel>) -> Self {
        Image {
            width,
            height,
            pixels,
        }
    }

    pub fn new_rgb(width: u32, height: u32, pixels: Vec<Pixel>) -> Self {
        debug_assert!(pixels.iter().all(|p| p.is_rgb()));
        Image::new(width, height, pixels)
    }

    pub fn new_rgba(width: u32, height: u32, pixels: Vec<Pixel>) -> Self {
        debug_assert!(pixels.iter().all(|p| p.is_rgb()));
        Image::new(width, height, pixels)
    }
}

pub enum Pixel {
    RGB { r: u8, g: u8, b: u8 },
    RGBA { r: u8, g: u8, b: u8, a: u8 },
}

impl Pixel {
    /// Returns `true` if the pixel is [`RGB`].
    ///
    /// [`RGB`]: Pixel::RGB
    #[must_use]
    pub fn is_rgb(&self) -> bool {
        matches!(self, Self::RGB { .. })
    }

    /// Returns `true` if the pixel is [`RGBA`].
    ///
    /// [`RGBA`]: Pixel::RGBA
    #[must_use]
    pub fn is_rgba(&self) -> bool {
        matches!(self, Self::RGBA { .. })
    }
}
