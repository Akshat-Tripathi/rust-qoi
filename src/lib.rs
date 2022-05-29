mod chunks;
mod consts;
pub mod decoder;
pub mod encoder;
mod util;
mod codec;

#[cfg(test)]
mod tests {
    use std::io::BufWriter;

    use image::io::Reader as ImageReader;
    use image::{GenericImageView, ImageDecoder, ImageEncoder};

    use crate::encoder::QoiEncoder;

    fn get_images(file_ext: &str) -> Vec<String> {
        let paths = std::fs::read_dir("./qoi_test_images").unwrap();
        paths
            .map(|f| f.unwrap().file_name().into_string().unwrap())
            .filter(|f| f.ends_with(file_ext))
            .map(|f| f.clone().strip_suffix(file_ext).unwrap().to_owned())
            .collect::<Vec<String>>()
    }

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
        fn test_images() {
            for fname in get_images(".png") {
                let img = ImageReader::open("qoi_test_images/".to_owned() + &fname + ".png")
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

                let reference =
                    std::fs::read("qoi_test_images/".to_owned() + &fname + ".qoi").unwrap();

                assert!(reference == buf);
            }
        }
    }

    #[cfg(test)]
    mod decoding_tests {
        use std::io::Read;

        use super::*;
        use crate::decoder::{QoiDecoder, QoiReader};

        #[test]
        fn test_rle_decoding() {
            //254 is OP_RGB and the weird | thing is OP_RUN
            let img = Vec::from([254, 100, 100, 0, 0b1100_0000 | (3 - 1)]);
            let mut decoder = QoiReader::new(&img[..], 3);

            let mut buf = vec![0u8; 12];
            decoder.read(&mut buf).unwrap();
            
            assert!(buf == &[100, 100, 0, 100, 100, 0, 100, 100, 0, 100, 100, 0])
        }

        #[test]
        fn test_decode() {
            for fname in get_images(".qoi") {
                let reader =
                    std::fs::read("qoi_test_images/".to_owned() + &fname + ".qoi").unwrap();
                let decoder = QoiDecoder::new(&reader[..]).unwrap();

                let (w, h) = decoder.dimensions();
                let channels = decoder.color_type().channel_count();
                let mut bytes: Vec<u8> = vec![0; (w * h * channels as u32) as usize];

                decoder.read_image(&mut bytes).unwrap();

                let reference = ImageReader::open("qoi_test_images/".to_owned() + &fname + ".png")
                    .unwrap()
                    .decode()
                    .unwrap();


                assert_eq!(bytes, reference.as_bytes())
            }
        }
    }

    #[cfg(test)]
    mod general_tests {

        use crate::{
            chunks::{OP_DIFF, OP_LUMA},
            util::Pixel,
        };

        #[test]
        fn test_op_diff_to_pixel_conversion() {
            let base_pixel = Pixel::new(2, 2, 2, 0);

            for i in -2..1 {
                let test_pixel = Pixel::new(
                    (base_pixel.r() as i32 + i) as u8,
                    (base_pixel.g() as i32 + i) as u8,
                    (base_pixel.b() as i32 + i) as u8,
                    base_pixel.a(),
                );

                let chunk = OP_DIFF::try_new(base_pixel, test_pixel).unwrap();

                assert_eq!(Pixel::from((base_pixel, chunk)), test_pixel);
            }
        }

        #[test]
        fn test_op_luma_to_pixel_conversion() {
            let base_pixel = Pixel::new(100, 100, 100, 100);

            for i in -32..31 {
                let test_pixel = Pixel::new(
                    (base_pixel.r() as i32 + i) as u8,
                    (base_pixel.g() as i32 + i) as u8,
                    (base_pixel.b() as i32 + i) as u8,
                    base_pixel.a(),
                );

                let chunk = OP_LUMA::try_new(base_pixel, test_pixel).unwrap();

                assert_eq!(Pixel::from((base_pixel, chunk)), test_pixel);
            }
        }
    }
}
