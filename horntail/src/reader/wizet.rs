use crate::AccessorOpt;
use crate::crypto::{MapleTableNone, MapleVersion};
use crate::error::{Error, Result};
use crate::reader::Source;
use crate::reader::{Accessor, BinaryAccessor};
use std::io::SeekFrom;
use std::path::Path;

const WIZET_SIGNATURE: u32 = 0x31474B50;

pub struct WizetFile {
    source: Source,
    ver: MapleVersion,
    copyright: String,
    data_pos: usize,
    no_version: bool,
}

pub fn get_encrypt_version<P: AsRef<Path>>(p: P) -> Result<u16> {
    let accessor = Source::new(p).open()?;
    let mut accessor = BinaryAccessor::new(MapleTableNone, accessor);
    if accessor.get_u32_le() != WIZET_SIGNATURE {
        return Err(Error::BrokenFile);
    }
    accessor.advance(8);
    let header_size = accessor.get_u32_le();
    accessor.try_seek(SeekFrom::Start(header_size as u64))?;
    Ok(accessor.get_u16_le())
}

impl WizetFile {
    pub const EXTENSION: &'static str = "wz";

    pub fn new<P: AsRef<Path>>(path: P, ver: MapleVersion, no_version: bool) -> Result<WizetFile> {
        let source = Source::new(path);
        let mmap = source.open()?;
        let mut accessor = BinaryAccessor::new(MapleTableNone, mmap);
        if accessor.get_u32_le() != WIZET_SIGNATURE {
            return Err(Error::BrokenFile);
        }

        let data_size = accessor.get_u64_le() as usize;
        let header_size = accessor.get_u32_le() as usize;

        if accessor.len() - data_size != header_size {
            return Err(Error::BrokenFile);
        }

        let mut copyright_raw = vec![0; header_size - accessor.pos() - 1];
        accessor.copy_to_slice(&mut copyright_raw);
        let copyright = String::from_utf8(copyright_raw).unwrap();

        if !no_version {
            accessor.try_seek(SeekFrom::Start(header_size as u64))?;
            if ver.hash_enc() != accessor.get_u16_le() {
                return Err(Error::InvalidVersion);
            }
        }

        Ok(WizetFile {
            source,
            ver,
            copyright,
            data_pos: header_size,
            no_version,
        })
    }

    #[inline]
    pub fn copyright(&self) -> &str {
        &self.copyright
    }

    #[inline]
    pub fn data_pos(&self) -> usize {
        self.data_pos
    }

    #[inline]
    pub fn offset(&self) -> usize {
        if self.no_version {
            self.data_pos
        } else {
            self.data_pos.saturating_add(2)
        }
    }

    #[inline]
    pub fn source(&self) -> &Source {
        &self.source
    }

    #[inline]
    pub fn version(&self) -> MapleVersion {
        self.ver
    }

    #[inline]
    pub fn accessor_opt(&self) -> AccessorOpt {
        AccessorOpt {
            ver_hash: self.ver.hash(),
            parent_offset: self.data_pos,
            offset: self.offset(),
        }
    }
}
