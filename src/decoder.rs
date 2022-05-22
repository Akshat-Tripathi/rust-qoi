use std::{
    convert::TryInto,
    error,
    fmt::Display,
    io::{Bytes, Read, self},
    iter::Peekable,
};

use image::{error::DecodingError, ImageDecoder, ImageError, ImageResult};

use crate::{
    chunks::{OP_DIFF, OP_INDEX, OP_LUMA, OP_RGB, OP_RGBA, OP_RUN, QOI_CHUNK},
    consts::SEEN_PIXEL_ARRAY_SIZE,
    util::Pixel,
};

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
    colour_type: image::ColorType,
    //TODO: Understand what a colour space is
}

impl<R: Read> QoiDecoder<R> {
    pub fn new(reader: R) -> ImageResult<QoiDecoder<R>> {
        let mut decoder = QoiDecoder {
            reader,
            width: 0,
            height: 0,
            colour_type: image::ColorType::Rgba8,
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

        self.colour_type = match buf[12] {
            3 => image::ColorType::Rgb8,
            4 => image::ColorType::Rgba8,
            _ => return Err(DecoderError::InvalidHeader)?,
        };

        Ok(())
    }
}

pub struct QoiReader<R: Read> {
    reader: Peekable<Bytes<R>>,
    previously_seen: [Pixel; SEEN_PIXEL_ARRAY_SIZE],
    last_pixel: Pixel,
    run_length: u8, //Current run length
    channels: u8,
}

impl<R: Read> QoiReader<R> {
    pub fn new(reader: R, channels: u8) -> QoiReader<R> {
        QoiReader {
            reader: reader.bytes().peekable(),
            previously_seen: [Pixel::new(0, 0, 0, 0); SEEN_PIXEL_ARRAY_SIZE],
            last_pixel: Pixel::new(0, 0, 0, 255),
            run_length: 0,
            channels,
        }
    }

    fn read_pixel(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        if self.run_length > 0 {
            self.run_length -= 1;
        } else {
            let pixel = if let Some(chunk) = OP_DIFF::try_decode(&mut self.reader) {
                Pixel::from((self.last_pixel, chunk))
            } else if let Some(chunk) = OP_INDEX::try_decode(&mut self.reader) {
                Pixel::from((self.previously_seen, chunk))
            } else if let Some(chunk) = OP_LUMA::try_decode(&mut self.reader) {
                Pixel::from((self.last_pixel, chunk))
            } else if let Some(chunk) = OP_RGBA::try_decode(&mut self.reader) {
                chunk.into()
            } else if let Some(chunk) = OP_RGB::try_decode(&mut self.reader) {
                chunk.into()
            } else if let Some(chunk) = OP_RUN::try_decode(&mut self.reader) {
                self.run_length = chunk.run_length() - 1;
                self.last_pixel
            } else {
                unreachable!()
            };

            self.previously_seen[pixel.hash()] = pixel;
            self.last_pixel = pixel;
        }

        buf[0] = self.last_pixel.r();
        buf[1] = self.last_pixel.g();
        buf[2] = self.last_pixel.b();
        if self.channels == 4 {
            buf[3] = self.last_pixel.a();
        }
        Ok(())
    }
}

impl<R: Read> Read for QoiReader<R> {
    //This will return self.channels * number of pixels read
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let max_iterations = buf.len() / (self.channels as usize);
        let mut ptr = buf;

        for _ in 0..max_iterations {
            self.read_pixel(ptr)?;
            ptr = &mut ptr[(self.channels as usize)..]
        }

        Ok(max_iterations * self.channels as usize)
    }
}

impl<'a, R: 'a + Read> ImageDecoder<'a> for QoiDecoder<R> {
    type Reader = QoiReader<R>;

    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn color_type(&self) -> image::ColorType {
        self.colour_type
    }

    fn into_reader(self) -> ImageResult<Self::Reader> {
        let channels = self.color_type().channel_count();
        Ok(QoiReader::new(self.reader, channels))
    }
}
