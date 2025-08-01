use crate::reader::Accessor;
use crate::{AccessorOpt, Error, Properties, TryFromAccessor};
use std::time::Duration;

#[derive(Clone)]
pub struct WaveFormatEx {
    pub format_tag: u16,
    pub channels: u16,
    pub samples_per_sec: u32,
    pub avg_bytes_per_sec: u32,
    pub block_align: u16,
    pub bits_per_sample: u16,
    pub cb_size: u16,
}

const WAVE_FORMAT_EX_SIZE: usize = 18;

#[derive(Clone)]
pub struct MPEGLayer3WaveFormat {
    pub wfx: WaveFormatEx,
    pub wid: u16,
    pub fdw_flags: u32,
    pub block_size: u16,
    pub frames_per_block: u16,
    pub codec_delay: u16,
}

const MPEG_LAYER3_WAVE_FORMAT_SIZE: usize = WAVE_FORMAT_EX_SIZE + 12;

const MPEG_LAYER3_SIZE: usize = MPEG_LAYER3_WAVE_FORMAT_SIZE - WAVE_FORMAT_EX_SIZE;

#[derive(Clone)]
pub enum WaveFormat {
    PCM(WaveFormatEx),
    MP3(MPEGLayer3WaveFormat),
}

const WAVE_FORMAT_PCM: u16 = 0x0001;
const WAVE_FORMAT_MPEG_LAYER3: u16 = 0x0055;

#[derive(Clone)]
pub struct AMMediaType {
    pub major_type: uuid::Uuid,
    pub sub_type: uuid::Uuid,
    pub fixed_size_samples: bool,
    pub temporal_compression: bool,
    pub format_type: uuid::Uuid,
    pub pb_format: Option<WaveFormat>,
}

#[derive(Clone)]
pub struct SoundAttribute {
    pub properties: Option<Properties>,
    pub duration: Duration,
    pub media_type: AMMediaType,
    pub data_size: usize,
}

impl TryFromAccessor for SoundAttribute {
    type Error = Error;

    fn try_from_accessor(
        opt: AccessorOpt,
        accessor: &mut dyn Accessor,
    ) -> Result<Self, Self::Error> {
        let properties = Properties::optional(opt, accessor)?;
        let sound_data_size = accessor.get_var_i32_le() as usize;
        let sound_duration = Duration::from_millis(accessor.get_var_i32_le() as u64);
        let sound_type = accessor.get_u8();
        let mut uuid_buffer = [0; 16];
        accessor.copy_to_slice(&mut uuid_buffer);
        let major_type = uuid::Uuid::from_slice_le(&uuid_buffer)
            .map_err(|e| Error::UnexpectedData(format!("major type parse failed: {e}")))?;
        accessor.copy_to_slice(&mut uuid_buffer);
        let sub_type = uuid::Uuid::from_slice_le(&uuid_buffer)
            .map_err(|e| Error::UnexpectedData(format!("sub_type parse failed: {e}")))?;
        let fixed_size_samples = accessor.get_u8() != 0;
        let temporal_compression = accessor.get_u8() != 0;
        accessor.copy_to_slice(&mut uuid_buffer);
        let sound_format_type = uuid::Uuid::from_slice_le(&uuid_buffer)
            .map_err(|e| Error::UnexpectedData(format!("sound_format_type parse failed: {e}")))?;

        let sound_format = match sound_type {
            1 => None,
            2 => Some(parse_wave_format(accessor)?),
            _ => {
                return Err(Error::UnexpectedData(
                    format!("sound type {sound_type:#x}",),
                ));
            }
        };

        Ok(SoundAttribute {
            properties,
            duration: sound_duration,
            data_size: sound_data_size,
            media_type: AMMediaType {
                major_type,
                sub_type,
                fixed_size_samples,
                temporal_compression,
                format_type: sound_format_type,
                pb_format: sound_format,
            },
        })
    }
}

#[derive(Clone)]
pub struct Sound {
    attr: SoundAttribute,
    raw_data: Vec<u8>,
}

impl Sound {
    #[inline]
    pub fn attr(&self) -> &SoundAttribute {
        &self.attr
    }

    #[inline]
    pub fn raw_data(&self) -> &[u8] {
        &self.raw_data
    }
}

impl TryFromAccessor for Sound {
    type Error = Error;

    fn try_from_accessor(
        opt: AccessorOpt,
        accessor: &mut dyn Accessor,
    ) -> Result<Self, Self::Error> {
        let attr = SoundAttribute::try_from_accessor(opt, accessor)?;
        let mut raw_data = vec![0; attr.data_size];
        accessor.copy_to_slice(&mut raw_data);
        Ok(Sound { attr, raw_data })
    }
}

#[inline]
fn parse_wave_format(accessor: &mut dyn Accessor) -> crate::error::Result<WaveFormat> {
    let fmt_len = accessor.get_var_i32_le() as usize;

    let fmt_ex = WaveFormatEx {
        format_tag: accessor.get_u16_le(),
        channels: accessor.get_u16_le(),
        samples_per_sec: accessor.get_u32_le(),
        avg_bytes_per_sec: accessor.get_u32_le(),
        block_align: accessor.get_u16_le(),
        bits_per_sample: accessor.get_u16_le(),
        cb_size: accessor.get_u16_le(),
    };

    match fmt_ex.format_tag {
        WAVE_FORMAT_PCM => {
            if fmt_len != WAVE_FORMAT_EX_SIZE {
                return Err(Error::BrokenFile);
            }
            Ok(WaveFormat::PCM(fmt_ex))
        }
        WAVE_FORMAT_MPEG_LAYER3 => {
            if fmt_ex.cb_size as usize != MPEG_LAYER3_SIZE {
                return Err(Error::BrokenFile);
            }

            if fmt_len != MPEG_LAYER3_WAVE_FORMAT_SIZE {
                return Err(Error::BrokenFile);
            }

            Ok(WaveFormat::MP3(MPEGLayer3WaveFormat {
                wfx: fmt_ex,
                wid: accessor.get_u16_le(),
                fdw_flags: accessor.get_u32_le(),
                block_size: accessor.get_u16_le(),
                frames_per_block: accessor.get_u16_le(),
                codec_delay: accessor.get_u16_le(),
            }))
        }
        _ => Err(Error::UnexpectedData(format!(
            "wave format tag {:#x}",
            fmt_ex.format_tag
        ))),
    }
}
