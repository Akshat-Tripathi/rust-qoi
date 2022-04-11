pub mod image;

#[cfg(test)]
mod tests {

    use crate::image::*;
    use image::{io::Reader as ImageReader, GenericImageView};

    fn read_png(filename: &str) -> Image {
        let img = ImageReader::open(filename)
            .unwrap()
            .decode()
            .unwrap();

        let (w, h) = img.dimensions();
        match img {
            image::DynamicImage::ImageRgb8(img) => {
                let pixels = img
                    .pixels()
                    .map(|p| Pixel::RGB {
                        r: p[0],
                        g: p[1],
                        b: p[2],
                    })
                    .collect();
                Image::new_rgb(w, h, pixels)
            }

            image::DynamicImage::ImageRgba8(img) => {
                let pixels = img
                    .pixels()
                    .map(|p| Pixel::RGBA {
                        r: p[0],
                        g: p[1],
                        b: p[2],
                        a: p[3],
                    })
                    .collect();
                Image::new_rgba(w, h, pixels)
            }
            _ => panic!("Incorrect image format"),
        }
    }

    #[test]
    fn test_encode() {
        
    }

    #[test]
    fn test_decode() {}
}
