use crate::Vector2D;
use crate::error::Error;
use crate::reader::Accessor;
use crate::{AccessorOpt, Properties, TryFromAccessor};
#[cfg(feature = "image")]
use image::*;
use std::fmt::{Display, Formatter};
use std::io::{Read, SeekFrom};
use std::ops::Deref;
use std::time::Duration;

const ZLIB_HEADER_BYTE: u16 = 0x9C78;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum CanvasFormat {
    BGRA4444,
    BGRA8888,
    Gray,
    ARGB1555,
    RGB565,
    RGB565Thumb,
    DXT1,
    DXT3,
    DXT5,
    ALPHA8,
    RGBA1010102,
    BC7,
    RGBAFloat,
    Unknown(i32),
}

impl From<i32> for CanvasFormat {
    fn from(value: i32) -> Self {
        match value {
            1 => CanvasFormat::BGRA4444,
            2 => CanvasFormat::BGRA8888,
            3 => CanvasFormat::Gray,
            257 => CanvasFormat::ARGB1555,
            513 => CanvasFormat::RGB565,
            517 => CanvasFormat::RGB565Thumb,
            1026 => CanvasFormat::DXT3,
            2050 => CanvasFormat::DXT5,
            // KMST 1186
            2304 => CanvasFormat::ALPHA8,
            2562 => CanvasFormat::RGBA1010102,
            4097 => CanvasFormat::DXT1,
            4098 => CanvasFormat::BC7,
            4100 => CanvasFormat::RGBAFloat,
            _ => CanvasFormat::Unknown(value),
        }
    }
}

impl CanvasFormat {
    fn data_size(&self, width: i32, height: i32) -> i32 {
        match self {
            CanvasFormat::BGRA4444 | CanvasFormat::ARGB1555 | CanvasFormat::RGB565 => {
                width * height * 2
            }
            CanvasFormat::BGRA8888 | CanvasFormat::RGBA1010102 => width * height * 4,
            CanvasFormat::DXT3 | CanvasFormat::DXT5 | CanvasFormat::Gray | CanvasFormat::BC7 => {
                ((width + 3) / 4) * ((height + 3) / 4) * 16
            }
            CanvasFormat::DXT1 => ((width + 3) / 4) * ((height + 3) / 4) * 8,
            CanvasFormat::ALPHA8 => width * height,
            CanvasFormat::RGBAFloat => width * height * 16,
            CanvasFormat::RGB565Thumb => (width * height) / 128,
            CanvasFormat::Unknown(_) => 0,
        }
    }
}

impl Display for CanvasFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CanvasFormat::Unknown(_) => f.write_str("Unknown"),
            CanvasFormat::BGRA4444 => f.write_str("BGRA4444"),
            CanvasFormat::BGRA8888 => f.write_str("BGRA8888"),
            CanvasFormat::Gray => f.write_str("Gray"),
            CanvasFormat::ARGB1555 => f.write_str("ARGB1555"),
            CanvasFormat::RGB565 => f.write_str("RGB565"),
            CanvasFormat::RGB565Thumb => f.write_str("RGB565Thumb"),
            CanvasFormat::DXT3 => f.write_str("DXT3"),
            CanvasFormat::DXT5 => f.write_str("DXT5"),
            CanvasFormat::DXT1 => f.write_str("DXT1"),
            CanvasFormat::ALPHA8 => f.write_str("A8"),
            CanvasFormat::RGBA1010102 => f.write_str("RGBAAAA2"),
            CanvasFormat::BC7 => f.write_str("BC7"),
            CanvasFormat::RGBAFloat => f.write_str("RGBAFloat"),
        }
    }
}

#[derive(Clone)]
pub struct CanvasAttribute {
    pub property: Option<Properties>,
    pub size: Vector2D,
    pub format: CanvasFormat,
    pub data_size: usize,
}

impl TryFromAccessor for CanvasAttribute {
    type Error = Error;

    fn try_from_accessor(
        opt: AccessorOpt,
        accessor: &mut dyn Accessor,
    ) -> Result<Self, Self::Error> {
        let property = Properties::builtin(opt, accessor)?;
        let size = Vector2D::try_from_accessor(opt, accessor)?;
        let format = CanvasFormat::from(accessor.get_var_i32_le() + accessor.get_u8() as i32);

        if let CanvasFormat::Unknown(format) = format {
            return Err(Error::UnexpectedData(format!("canvas format {format}")));
        }

        // unknown bytes
        accessor.advance(4);

        // has zero bytes at start useless
        let data_size = (accessor.get_i32_le() - 1) as usize;
        accessor.advance(1);

        Ok(CanvasAttribute {
            property,
            size,
            format,
            data_size,
        })
    }
}

#[derive(Clone)]
pub struct Canvas {
    pub attr: CanvasAttribute,
    data: Vec<u8>,
}

impl Canvas {
    #[inline]
    pub fn into_inner(self) -> Vec<u8> {
        self.data
    }

    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

impl TryFromAccessor for Canvas {
    type Error = Error;

    fn try_from_accessor(
        opt: AccessorOpt,
        accessor: &mut dyn Accessor,
    ) -> Result<Self, Self::Error> {
        let attr = CanvasAttribute::try_from_accessor(opt, accessor)?;

        let flag = accessor.get_u16_le();
        accessor.try_seek(SeekFrom::Current(-2))?;

        let data = if flag != ZLIB_HEADER_BYTE {
            // decrypt image data
            let mut de_data = Vec::with_capacity(attr.data_size);
            while accessor.has_remaining() {
                let size = accessor.get_i32_le() as usize;
                let off = de_data.len();
                de_data.resize(off + size, 0);
                accessor.decrypt_to_slice(&mut de_data[off..off + size]);
            }
            de_data
        } else {
            accessor.copy_to_vec(attr.data_size)
        };

        let raw_data_size = attr.format.data_size(attr.size.x, attr.size.y) as usize;
        let mut zlib_dec = flate2::read::ZlibDecoder::new_with_buf(
            &*data,
            vec![0; raw_data_size.max(attr.data_size).min(32 * 1024)],
        );

        let mut raw_data = vec![0; raw_data_size];

        let mut de_size = 0;
        while de_size < raw_data_size {
            let deflate_size = zlib_dec.read(&mut raw_data[de_size..])?;
            if deflate_size == 0 {
                break;
            }
            de_size += deflate_size;
        }

        if de_size != raw_data_size {
            return Err(Error::BrokenFile);
        }

        Ok(Canvas {
            attr,
            data: raw_data,
        })
    }
}

#[cfg(feature = "image")]
macro_rules! to_bit8 {
    (5,$val:expr) => {{
        let val = $val as u8;
        val << 3 | val >> 2
    }};
    (6,$val:expr) => {{
        let val = $val as u8;
        val << 2 | val >> 4
    }};
    (4,$val:expr) => {{
        let val = $val as u8;
        val | val << 4
    }};
    (1,$val:expr) => {{ if $val & 1 == 1 { u8::MAX } else { u8::MIN } }};
}

#[cfg(feature = "image")]
impl Canvas {
    const DXT3_5_DECODED_BYTES_PER_BLOCK: usize = 64;
    const DXT3_5_ENCODED_BYTES_PER_BLOCK: usize = 16;

    pub fn image(&self) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        match self.attr.format {
            CanvasFormat::RGB565 => self.rgb565_to_rgba(),
            CanvasFormat::RGB565Thumb => self.rgb565_thumb_to_rgba(),
            CanvasFormat::BGRA4444 => self.bgra4444_to_rgba(),
            CanvasFormat::BGRA8888 => self.bgra8888_to_rgba(),
            CanvasFormat::ARGB1555 => self.argb1555_to_rgba(),
            CanvasFormat::Gray | CanvasFormat::DXT3 => self.dxt3_to_rgba(),
            CanvasFormat::DXT5 => self.dxt5_to_rgba(),
            CanvasFormat::RGBA1010102 => self.rgba1010102_to_rgba(),
            CanvasFormat::BC7 => {
                // TODO: not implement
                None
            }
            _ => None,
        }
    }

    #[inline]
    fn process_chunk<F, U>(&self, size: usize, cb: F) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>>
    where
        F: Fn(&[u8]) -> U,
        U: IntoIterator<Item = u8>,
    {
        let data = self
            .data
            .chunks_exact(size)
            .flat_map(cb)
            .collect::<Vec<_>>();
        ImageBuffer::from_raw(self.attr.size.x as u32, self.attr.size.y as u32, data)
    }

    fn bgra4444_to_rgba(&self) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        self.process_chunk(2, |chunk| {
            let bits = u16::from_le_bytes([chunk[0], chunk[1]]);
            [
                to_bit8!(4, (bits >> 8) & 0xF),
                to_bit8!(4, (bits >> 4) & 0xF),
                to_bit8!(4, (bits & 0xF)),
                to_bit8!(4, (bits >> 12) & 0xF),
            ]
        })
    }

    fn bgra8888_to_rgba(&self) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        self.process_chunk(4, |chunk| [chunk[2], chunk[1], chunk[0], chunk[3]])
    }

    fn dxt3_to_rgba(&self) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        use crate::entry::canvas::dxt;

        let width_block = (self.attr.size.x / 4) as usize;
        let per_line_size = Self::DXT3_5_DECODED_BYTES_PER_BLOCK * width_block;

        self.process_chunk(
            Self::DXT3_5_ENCODED_BYTES_PER_BLOCK * width_block,
            |chunk| {
                let mut block = vec![0u8; per_line_size];
                dxt::decode_dxt3_row(chunk, &mut block);
                block
            },
        )
    }

    fn argb1555_to_rgba(&self) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        self.process_chunk(2, |chunk| {
            let color = u16::from_le_bytes([chunk[0], chunk[1]]);
            [
                to_bit8!(5, (color >> 10) & 0x1F),
                to_bit8!(5, (color >> 5) & 0x1F),
                to_bit8!(5, (color & 0x1F)),
                to_bit8!(1, color >> 0xF),
            ]
        })
    }

    #[inline]
    fn rgb565_to_rgba_color(chunk: &[u8]) -> [u8; 4] {
        let color = u16::from_le_bytes([chunk[0], chunk[1]]);
        [
            to_bit8!(5, (color >> 11) & 0x1F),
            to_bit8!(6, (color >> 5) & 0x3F),
            to_bit8!(5, color & 0x1F),
            u8::MAX,
        ]
    }

    fn rgb565_to_rgba(&self) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        self.process_chunk(2, Self::rgb565_to_rgba_color)
    }

    fn rgb565_thumb_to_rgba(&self) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        self.process_chunk(((self.attr.size.x / 16) * 2) as usize, |block| {
            block
                .chunks_exact(2)
                .flat_map(|chunk| Self::rgb565_to_rgba_color(chunk).repeat(16))
                .collect::<Vec<_>>()
                .repeat(16)
        })
    }

    fn dxt5_to_rgba(&self) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        use crate::entry::canvas::dxt;

        let width_block = (self.attr.size.x / 4) as usize;
        let per_line_size = Self::DXT3_5_DECODED_BYTES_PER_BLOCK * width_block;

        self.process_chunk(
            Self::DXT3_5_ENCODED_BYTES_PER_BLOCK * width_block,
            |chunk| {
                let mut block = vec![0u8; per_line_size];
                dxt::decode_dxt5_row(chunk, &mut block);
                block
            },
        )
    }

    fn rgba1010102_to_rgba(&self) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        self.process_chunk(4, |chunk| {
            let color = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            [
                ((color >> 24) & 0xff) as u8,
                ((color >> 14) & 0xff) as u8,
                ((color >> 4) & 0xff) as u8,
                ((color & 0x03) * 85) as u8,
            ]
        })
    }
}

const MCV0: u32 = 0x3056434D; // MCV0

#[derive(Default, Clone)]
pub struct Metadata {
    pub data_offset: usize,
    pub data_size: usize,
    pub alpha_data_offset: usize,
    pub alpha_data_size: usize,
    pub delay: Duration,
    pub start_time: Duration,
}

// mcv flag
pub const ALPHA_MAP: u8 = 1;
pub const PER_FRAME_DEALY: u8 = 1 << 1;
pub const PER_FRAME_TIME_LINE: u8 = 1 << 2;

#[derive(Clone)]
pub struct VideoAttribute {
    pub properties: Option<Properties>,
    pub size: usize,
    pub fourcc: u32,
    pub width: u16,
    pub height: u16,
    pub frame_cnt: u32,
    pub mcv_flag: u8,
    pub frame_dealy_unit: u64,
    pub default_delay: u32,
    pub metadata: Vec<Metadata>,
}

impl TryFromAccessor for VideoAttribute {
    type Error = Error;

    fn try_from_accessor(
        opt: AccessorOpt,
        accessor: &mut dyn Accessor,
    ) -> Result<Self, Self::Error> {
        let properties = Properties::builtin(opt, accessor)?;

        let offset = accessor.pos();
        accessor.advance(1);
        let size = accessor.get_var_i32_le() as usize;

        let flag = accessor.get_u32_le();
        if flag != MCV0 {
            return Err(Error::UnexpectedData(format!("video flag {flag}")));
        }
        accessor.advance(2);
        let header_len = accessor.get_u16_le();
        let fourcc = accessor.get_u32_le() ^ 0xa5a5a5a5;
        let width = accessor.get_u16_le();
        let height = accessor.get_u16_le();
        let frame_cnt = accessor.get_u32_le();
        let mcv_flag = accessor.get_u8();
        accessor.advance(3);
        let frame_dealy_unit = accessor.get_u64_le();
        let default_delay = accessor.get_u32_le();
        accessor.seek(SeekFrom::Start((offset + header_len as usize) as u64));

        let mut metadata = vec![Metadata::default(); frame_cnt as usize];
        metadata.iter_mut().for_each(|frame| {
            frame.data_offset = accessor.get_i32_le() as usize;
            frame.data_size = accessor.get_i32_le() as usize;
        });

        let has_alpha_map = mcv_flag & ALPHA_MAP == ALPHA_MAP;

        if has_alpha_map {
            metadata.iter_mut().for_each(|frame| {
                frame.alpha_data_offset = accessor.get_i32_le() as usize;
                frame.alpha_data_size = accessor.get_i32_le() as usize;
            });
        }

        metadata.iter_mut().for_each(|frame| {
            if mcv_flag & PER_FRAME_DEALY == PER_FRAME_DEALY {
                frame.delay = Duration::from_nanos(
                    (accessor.get_u32_le() as u64).wrapping_mul(frame_dealy_unit),
                );
            } else {
                frame.delay =
                    Duration::from_nanos((default_delay as u64).wrapping_mul(frame_dealy_unit));
            }
        });

        metadata.iter_mut().fold(0, |time, frame| {
            if mcv_flag & PER_FRAME_TIME_LINE == PER_FRAME_TIME_LINE {
                frame.start_time =
                    Duration::from_nanos(accessor.get_u64_le().wrapping_mul(frame_dealy_unit));
                time
            } else {
                frame.start_time = Duration::from_nanos(time);
                time + frame.delay.as_nanos() as u64
            }
        });

        let data_offset = accessor.pos();
        metadata.iter_mut().for_each(|frame| {
            frame.data_offset += data_offset;
            if has_alpha_map {
                frame.alpha_data_offset += data_offset;
            }
        });

        Ok(VideoAttribute {
            properties,
            size,
            fourcc,
            width,
            height,
            frame_cnt,
            mcv_flag,
            frame_dealy_unit,
            default_delay,
            metadata,
        })
    }
}

#[derive(Clone)]
pub struct Frame {
    pub data: Vec<u8>,
    pub alpha_data: Vec<u8>,
}

#[derive(Clone)]
pub struct Video {
    attribute: VideoAttribute,
    frames: Vec<Frame>,
}

impl Deref for Video {
    type Target = [Frame];

    fn deref(&self) -> &Self::Target {
        &self.frames
    }
}

impl Video {
    #[inline]
    pub fn attr(&self) -> &VideoAttribute {
        &self.attribute
    }

    #[inline]
    pub fn into_inner(self) -> Vec<Frame> {
        self.frames
    }
}

impl TryFromAccessor for Video {
    type Error = Error;

    fn try_from_accessor(
        opt: AccessorOpt,
        accessor: &mut dyn Accessor,
    ) -> Result<Self, Self::Error> {
        let attribute = VideoAttribute::try_from_accessor(opt, accessor)?;

        let frames = attribute
            .metadata
            .iter()
            .map(|metadata| {
                accessor.seek(SeekFrom::Start(metadata.data_offset as u64));
                let mut data = vec![0; attribute.size];
                accessor.copy_to_slice(&mut data);
                let alpha_data = if attribute.mcv_flag & ALPHA_MAP == ALPHA_MAP {
                    accessor.seek(SeekFrom::Start(metadata.alpha_data_offset as u64));
                    let mut buffer = vec![0; metadata.alpha_data_size];
                    accessor.copy_to_slice(&mut buffer);
                    buffer
                } else {
                    Vec::with_capacity(0)
                };
                Frame { data, alpha_data }
            })
            .collect::<Vec<_>>();

        Ok(Video { attribute, frames })
    }
}
