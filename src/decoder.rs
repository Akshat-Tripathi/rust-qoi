use std::{
    convert::TryInto,
    error,
    fmt::Display,
    io::{Bytes, Read},
    iter::Peekable,
};

use image::{error::DecodingError, ImageDecoder, ImageError, ImageResult};

use crate::{
    chunks::QoiChunk,
    codec::QoiCodecState,
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
    state: QoiCodecState,
    channels: u8,
}

impl<R: Read> QoiReader<R> {
    pub fn new(reader: R, channels: u8) -> QoiReader<R> {
        QoiReader {
            reader: reader.bytes().peekable(),
            state: QoiCodecState::new(),
            channels,
        }
    }

    fn read_chunk<'a>(&mut self, buf: &'a mut [u8]) -> std::io::Result<&'a mut [u8]> {
        let chunk = QoiChunk::decode(&mut self.reader, &self.state);
        let (pixel, repeats) = self.state.process_chunk(chunk);

        for i in 0..repeats {
            let i = i * (self.channels as usize);
            buf[i] = pixel.r();
            buf[i+1] = pixel.g();
            buf[i+2] = pixel.b();
            if self.channels == 4 {
                buf[i+3] = pixel.a();
            }
        }
        Ok(&mut buf[((self.channels as usize)*repeats)..])
    }
}

impl<R: Read> Read for QoiReader<R> {
    //This will return self.channels * number of pixels read
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = buf.len();
        let mut ptr = buf;

        while ptr.len() >= self.channels.into() {
            ptr = self.read_chunk(ptr)?;
        }

        Ok(len - ptr.len())
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
