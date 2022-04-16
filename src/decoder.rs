use std::{error, fmt::Display, io::Read, convert::TryInto};

use image::{error::DecodingError, ImageDecoder, ImageError, ImageResult};

const HEADER_SIZE: usize = 14;

#[derive(Debug)]
enum DecoderError {
    HeaderTooSmall,
    InvalidHeader,
}

impl error::Error for DecoderError {}

impl Display for DecoderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecoderError::HeaderTooSmall => write!(f, "Header too small"),
            DecoderError::InvalidHeader => write!(f, "Invalid header"),
        }
    }
}

impl From<DecoderError> for ImageError {
    fn from(e: DecoderError) -> ImageError {
        ImageError::Decoding(DecodingError::new(
            image::error::ImageFormatHint::Name("QOI".to_string()),
            e,
        ))
    }
}

pub struct QoiDecoder<R: Read> {
    reader: R,
    width: u32,
    height: u32,
    image_type: image::ColorType,
    //TODO: Understand what a colour space is
}

impl<R: Read> QoiDecoder<R> {
    pub fn new(reader: R) -> ImageResult<QoiDecoder<R>> {
        let mut decoder = QoiDecoder {
            reader,
            width: 0,
            height: 0,
            image_type: image::ColorType::Rgba8,
        };
        decoder.read_metadata()?;
        Ok(decoder)
    }

    fn read_metadata(&mut self) -> ImageResult<()> {
        let mut buf = [0u8; HEADER_SIZE];
        self.reader
            .read(&mut buf)
            .or(Err(DecoderError::HeaderTooSmall))?;

        if &buf[0..4] != b"qoif" {
            return Err(DecoderError::InvalidHeader)?;
        }

        self.width = u32::from_be_bytes(buf[4..8].try_into().unwrap());
        self.height = u32::from_be_bytes(buf[8..12].try_into().unwrap());

        self.image_type = match buf[12] {
            3 => image::ColorType::Rgb8,
            4 => image::ColorType::Rgba8,
            _ => return Err(DecoderError::InvalidHeader)?,
        };

        Ok(())
    }
}
