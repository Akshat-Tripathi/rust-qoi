pub mod image;

#[cfg(test)]
mod tests {

    use image::io::Reader as ImageReader;

    #[test]
    fn test_encode() {
        let img = ImageReader::open("qoi_test_images/dice.png")?.decode()?;
        img.
    }

    fn test_decode() {
        
    }
}
