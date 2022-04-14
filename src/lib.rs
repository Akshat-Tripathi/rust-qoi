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

        #[test]
        fn test_rle() {
            //TODO Fuzz/Prop test this with any length less than 63 and any pixel in range [2..253]
            let img: [(u8, u8, u8); 4] =
                [(100, 100, 0), (100, 100, 0), (100, 100, 0), (100, 100, 0)];
            let bytes = img
                .iter()
                .flat_map(|&(r, g, b)| vec![r, g, b])
                .collect::<Vec<u8>>();

            let mut out = BufWriter::new(Vec::new());
            QoiEncoder::new(&mut out)
                .write_image(bytes.as_bytes(), 2, 2, image::ColorType::Rgb8)
                .unwrap();

            let stripped = strip(out.buffer());
            //254 is OP_RGB and the weird | thing is OP_RUN
            assert!(stripped == [254, 100, 100, 0, 0b1100_0000 | (3 - 1)])
        }

        #[test]
        fn test_images() {
            let img = ImageReader::open("qoi_test_images/kodim10.png")
                .unwrap()
                .decode()
                .unwrap();

            let fout = &mut BufWriter::new(File::create("out/test.qoi").unwrap());
            QoiEncoder::new(fout)
                .write_image(img.as_bytes(), img.width(), img.height(), img.color())
                .unwrap();
        }
    }

    #[test]
    fn test_decode() {}
}
