mod chunks;
pub mod encoder;
mod util;

use image::ImageEncoder;

#[cfg(test)]
mod tests {

    use std::fs::File;
    use std::io::BufWriter;

    use image::io::Reader as ImageReader;
    use image::{GenericImageView, ImageEncoder};

    use crate::encoder::QoiEncoder;

    #[cfg(test)]
    mod encoding_tests {
        use image::EncodableLayout;

        use super::*;

        fn strip(buf: &[u8]) -> &[u8] {
            &buf[14..(buf.len() - 8)] //14 byte header and 8 byte footer
        }

        type Image<const N: usize> = [(u8, u8, u8); N];

        fn encode_rgb_image<const N: usize>(img: Image<N>) -> Vec<u8> {
            let bytes = img
                .iter()
                .flat_map(|&(r, g, b)| vec![r, g, b])
                .collect::<Vec<u8>>();

            let mut out = BufWriter::new(Vec::new());
            QoiEncoder::new(&mut out)
                .write_image(bytes.as_bytes(), 2, 2, image::ColorType::Rgb8)
                .unwrap();

            Vec::from(strip(out.buffer()))
        }

        #[test]
        fn test_rle_rgb() {
            //TODO Fuzz/Prop test this with any length less than 63 and any pixel in range [2..253]
            let img: Image<4> = [(100, 100, 0), (100, 100, 0), (100, 100, 0), (100, 100, 0)];
            let pixel_buf = encode_rgb_image(img);
            //254 is OP_RGB and the weird | thing is OP_RUN
            assert!(pixel_buf == [254, 100, 100, 0, 0b1100_0000 | (3 - 1)])
        }

        #[test]
        fn test_only_pixels_rgb() {
            //TODO Fuzz/Prop test this with any reasonable length and each pixel is 3 apart from the previous one
            let img: Image<4> = [(100, 100, 0), (150, 100, 0), (100, 150, 0), (150, 150, 0)];
            let pixel_buf = encode_rgb_image(img);
            //254 is OP_RGB
            assert!(
                pixel_buf
                    == [254, 100, 100, 0, 254, 150, 100, 0, 254, 100, 150, 0, 254, 150, 150, 0]
            )
        }

        #[test]
        fn test_indexing() {
            //TODO Fuzz/Prop test this with any length with repeating pixels
            let img: Image<4> = [(100, 100, 0), (150, 100, 0), (100, 100, 0), (150, 100, 0)];
            let pixel_buf = encode_rgb_image(img);
            //254 is OP_RGB, 0 and 1 are the indices
            assert!(pixel_buf == [254, 100, 100, 0, 254, 150, 100, 0, 21, 43])
        }

        #[test]
        fn scratch() {
            let mut img = ImageReader::open("qoi_test_images/kodim10.png")
                .unwrap()
                .decode()
                .unwrap();


            let mut out = BufWriter::new(Vec::new());
            QoiEncoder::new(&mut out)
                .write_image(img.as_bytes(), 2, 2, image::ColorType::Rgb8)
                .unwrap();
        }

        #[test]
        fn test_rgb_images() {
            for fname in ["kodim10", "kodim23"].iter() {
                let img = ImageReader::open("qoi_test_images/".to_owned() + fname + ".png")
                    .unwrap()
                    .decode()
                    .unwrap();
    
                let mut buf: Vec<u8> = Vec::new();
                {
                    let fout = &mut BufWriter::new(&mut buf);
                    QoiEncoder::new(fout)
                        .write_image(img.as_bytes(), img.width(), img.height(), img.color())
                        .unwrap();
                }

                let reference = std::fs::read("qoi_test_images/".to_owned() + fname + ".qoi").unwrap();

                assert!(reference == buf);
            }
        }
    }

    #[test]
    fn test_decode() {}
}
