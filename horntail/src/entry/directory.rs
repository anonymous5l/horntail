use crate::reader::{Accessor, seek_back};
use crate::{AccessorOpt, EntryKind, Error, Image, TryFromAccessor};
use std::io::SeekFrom;
use std::ops::Deref;

const UNKNOWN: u8 = 1;
const UOL: u8 = 2;
const FOLDER: u8 = 3;
const IMAGE: u8 = 4;

#[derive(Clone)]
pub struct Directory {
    pub name: String,
    pub kind: EntryKind,
    pub offset: usize,
    pub parent_offset: usize,
}

impl TryFromAccessor for Option<Directory> {
    type Error = Error;

    fn try_from_accessor(
        opt: AccessorOpt,
        accessor: &mut dyn Accessor,
    ) -> Result<Self, Self::Error> {
        let kind = accessor.get_u8();
        if kind == UNKNOWN {
            accessor.advance(10);
            return Ok(None);
        }

        let (kind, name) = if kind == UOL {
            let off = accessor.get_u32_le() as usize + opt.parent_offset;
            seek_back(accessor, SeekFrom::Start(off as u64), |accessor| {
                (accessor.get_u8(), accessor.get_decrypt_string())
            })
        } else if kind == IMAGE || kind == FOLDER {
            (kind, accessor.get_decrypt_string())
        } else {
            panic!("invalid element kind {kind}");
        };

        let size = accessor.get_var_i32_le();
        // all the data byte sum together
        let _checksum = accessor.get_var_i32_le();
        let data_offset = compute_offset(&opt, accessor);

        match kind {
            IMAGE => {
                let mut image =
                    seek_back(accessor, SeekFrom::Start(data_offset as u64), |accessor| {
                        Image::try_from_accessor(opt.clone_with(data_offset), accessor)
                    })?;
                image.size = size as usize;
                Ok(Some(Directory {
                    name,
                    kind: image.kind,
                    offset: image.offset,
                    parent_offset: data_offset,
                }))
            }
            FOLDER => Ok(Some(Directory {
                name,
                kind: EntryKind::Folder,
                offset: data_offset,
                parent_offset: opt.parent_offset,
            })),
            _ => Err(Error::UnexpectedData(format!(
                "unexpected element kind {kind}"
            ))),
        }
    }
}

#[derive(Clone)]
pub struct Directories {
    directories: Vec<Directory>,
}

impl Deref for Directories {
    type Target = [Directory];
    fn deref(&self) -> &Self::Target {
        &self.directories
    }
}

impl Directories {
    #[inline]
    pub fn into_inner(self) -> Vec<Directory> {
        self.directories
    }
}

impl TryFromAccessor for Directories {
    type Error = Error;

    fn try_from_accessor(
        opt: AccessorOpt,
        accessor: &mut dyn Accessor,
    ) -> Result<Self, Self::Error> {
        let entry_count = accessor.get_var_i32_le() as usize;
        let mut directories = Vec::with_capacity(entry_count);
        for _ in 0..entry_count {
            let Some(directory) =
                Option::<Directory>::try_from_accessor(opt.clone_with(accessor.pos()), accessor)?
            else {
                continue;
            };
            directories.push(directory);
        }
        Ok(Directories { directories })
    }
}

#[inline]
fn compute_offset(opt: &AccessorOpt, accessor: &mut dyn Accessor) -> usize {
    let hash = opt.ver_hash as u32;
    let data_pos = opt.parent_offset;
    let offset = (accessor.pos().wrapping_sub(data_pos) as u32 ^ u32::MAX)
        .wrapping_mul(hash)
        .wrapping_sub(0x581c3f6d);
    let factor = offset & 0x1f;
    let enc_offset = accessor.get_u32_le();
    ((offset.wrapping_shl(factor) | offset.wrapping_shr(32 - factor)) ^ enc_offset)
        .wrapping_add((data_pos as u32).wrapping_shl(1)) as usize
}
