pub mod encoder;

use image::ImageEncoder;

#[cfg(test)]
mod tests {

    use std::fs::File;
    use std::io::{BufWriter, Write};

    use image::io::Reader as ImageReader;
    use image::{GenericImageView, ImageEncoder};

    use crate::encoder::QoiEncoder;

    #[test]
    fn test_encode() {
        let img = ImageReader::open("qoi_test_images/kodim10.png")
            .unwrap()
            .decode()
            .unwrap();

        let fout = &mut BufWriter::new(File::create("out/test.qoi").unwrap());
        QoiEncoder::new(fout)
            .write_image(img.as_bytes(), img.width(), img.height(), img.color())
            .unwrap();
    }

    #[test]
    fn test_decode() {}
}
