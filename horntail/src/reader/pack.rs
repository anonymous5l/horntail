use crate::crypto::MapleTableNone;
use crate::error::{Error, Result};
use crate::reader::snow2::{Snow2, align_size};
use crate::reader::{Accessor, BinaryAccessor, Source};
use std::io::{Read, SeekFrom};
use std::ops::Deref;
use std::path::Path;

const SUPPORTED_VERSION: u8 = 2;
const BLOCK_SIZE: usize = 0x400;

pub struct PackFile {
    source: Source,
    entry_key: [u8; 16],
    entry_pos: usize,
    entry_count: usize,
    image_key_salt: Vec<u8>,
}

impl PackFile {
    pub const EXTENSION: &'static str = "ms";

    pub fn new<P: AsRef<Path>>(path: P) -> Result<PackFile> {
        let source = Source::new(path.as_ref());

        let data = source.open()?;
        let mut accessor = BinaryAccessor::new(MapleTableNone, data);
        let filename = path
            .as_ref()
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let rand_byte_size = filename
            .chars()
            .fold(0i32, |acc, c| acc.wrapping_add(c as i32))
            % 312
            + 30;
        let rand_bytes = accessor.copy_to_vec(rand_byte_size as usize);
        let hash_salt_len = accessor.get_i32_le();
        let salt_bytes = accessor.copy_to_vec((hash_salt_len as u8 ^ rand_bytes[0]) as usize * 2);

        let salt_str = String::from_iter(
            salt_bytes
                .iter()
                .step_by(2)
                .zip(rand_bytes)
                .map(|(salt, r)| (r ^ *salt) as char),
        );

        let filename_sum = filename.chars().fold(0, |acc, x| acc + (x as usize * 3));
        let filename_with_salt = (filename + salt_str.as_str()).into_bytes();
        let file_name_with_salt_len = filename_with_salt.len() as u8;

        let snow_cipher_key = (0..16)
            .map(|i| filename_with_salt[(i % file_name_with_salt_len) as usize] + i)
            .collect::<Vec<_>>();

        let pos = accessor.pos();
        let mut snow2 = Snow2::new(accessor, snow_cipher_key.try_into().unwrap());

        let mut temp = [0; 12];
        snow2.read_exact(&mut temp)?;
        let mut snow2 = BinaryAccessor::new(MapleTableNone, temp);

        let hash = snow2.get_i32_le();
        let version = snow2.get_u8();
        let entry_count = snow2.get_i32_le();

        if version != SUPPORTED_VERSION {
            return Err(Error::InvalidVersion);
        }

        if salt_bytes.chunks(2).fold(
            hash_salt_len
                .wrapping_add(version as i32)
                .wrapping_add(entry_count),
            |acc, salt| acc.wrapping_add(u16::from_le_bytes([salt[0], salt[1]]) as i32),
        ) != hash
        {
            return Err(Error::BrokenFile);
        }

        let entry_pos = pos + 9 + filename_sum % 212 + 33;
        let salt_size = file_name_with_salt_len as usize;
        let mut entry_key = [0; 16];
        entry_key.iter_mut().enumerate().for_each(|(index, b)| {
            *b = index.wrapping_add(
                (index % 3 + 2)
                    .wrapping_mul(filename_with_salt[salt_size - 1 - index % salt_size] as usize),
            ) as u8
        });

        const KEY_HASH: u32 = 0x811C9DC5;
        let kh = salt_str
            .chars()
            .fold(KEY_HASH, |kh, x| (kh ^ x as u32).wrapping_mul(0x1000193));
        let kh_digits = kh
            .to_string()
            .chars()
            .map(|x| (x as u8).wrapping_sub(b'0'))
            .collect::<Vec<_>>();

        Ok(PackFile {
            source,
            entry_key,
            entry_pos,
            entry_count: entry_count as usize,
            image_key_salt: kh_digits,
        })
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    #[inline]
    pub fn entries(&self) -> Result<PackEntries> {
        PackEntries::try_from(self)
    }
}

#[derive(Debug, Clone)]
pub struct PackEntry {
    pub name: String,
    pub flags: i32,
    pub checksum: i32,
    pub offset: i32,
    pub size: i32,
    pub size_aligned: i32,
    pub unk1: i32,
    pub unk2: i32,
    pub key: [u8; 16],
}

impl PackEntry {
    #[inline]
    pub fn decrypt_from<T: Accessor + Read>(&self, accessor: &mut T) -> Result<Vec<u8>> {
        let prepare_size = align_size(self.size.min(0x400) as usize);
        accessor.seek(SeekFrom::Start(self.offset as u64));
        let mut p_buffer = vec![0; prepare_size];
        let mut prepare = Snow2::new(accessor, self.key);
        prepare.read_exact(&mut p_buffer)?;
        let stream = prepare.into_inner();
        stream.seek(SeekFrom::Start((self.offset + prepare_size as i32) as u64));
        let mut stream_crypto = Snow2::with_buffer(stream, &p_buffer, self.key)?;
        let mut buffer = vec![0; self.size_aligned as usize];
        stream_crypto.read_exact(buffer.as_mut_slice())?;
        Ok(buffer)
    }
}

pub struct PackEntries {
    entries: Vec<PackEntry>,
}

impl Deref for PackEntries {
    type Target = [PackEntry];

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl TryFrom<&PackFile> for PackEntries {
    type Error = Error;

    fn try_from(file: &PackFile) -> std::result::Result<Self, Self::Error> {
        let mut offset = file.entry_pos;
        let data = file.source.open()?;
        let mut stream = BinaryAccessor::new(MapleTableNone, data);
        stream.seek(SeekFrom::Start(offset as u64));
        let mut snow2 = Snow2::new(stream, file.entry_key);
        let mut entries = Vec::with_capacity(file.entry_count);
        for _ in 0..file.entry_count {
            let mut temp = [0; 4];
            snow2.read_exact(&mut temp)?;
            let entry_name_len = i32::from_le_bytes(temp);
            let mut temp = vec![0; (entry_name_len as usize * 2) + 44];
            snow2.read_exact(&mut temp)?;
            let temp_size = temp.len();
            let mut reader = BinaryAccessor::new(MapleTableNone, temp);
            let name = reader.get_utf16_string(entry_name_len as usize);
            let mut entry = PackEntry {
                name,
                checksum: reader.get_i32_le(),
                flags: reader.get_i32_le(),
                offset: reader.get_i32_le().wrapping_mul(BLOCK_SIZE as i32),
                size: reader.get_i32_le(),
                size_aligned: reader.get_i32_le(),
                unk1: reader.get_i32_le(),
                unk2: reader.get_i32_le(),
                key: Default::default(),
            };
            // let calc_checksum = flags
            //     .wrapping_add(start_pos)
            //     .wrapping_add(size)
            //     .wrapping_add(size_aligned)
            //     .wrapping_add(unk1)
            //     .wrapping_add(entry_key.iter().copied().fold(0, |acc, k| acc + k as i32));
            reader.copy_to_slice(&mut entry.key);

            let salt = &file.image_key_salt;
            let salt_len = file.image_key_salt.len();
            let mut image_key = [0; 16];
            let entry_name = entry.name.as_bytes();
            let entry_name_len = entry_name.len();
            let entry_key = &entry.key[..];
            let entry_key_len = entry_key.len();
            image_key.iter_mut().enumerate().for_each(|(i, k)| {
                *k = i.wrapping_add(
                    (entry_name[i % entry_name_len] as usize).wrapping_mul(
                        (salt[i % salt_len] as usize % 2)
                            .wrapping_add(
                                entry_key[(salt[(i + 2) % salt_len] as usize + i) % entry_key_len]
                                    as usize,
                            )
                            .wrapping_add((salt[(i + 1) % salt_len] as usize + i) % 5),
                    ),
                ) as u8;
            });
            entry.key = image_key;
            offset += temp_size + 4;
            entries.push(entry)
        }

        let image_data_off = (offset + BLOCK_SIZE - 1) & !(BLOCK_SIZE - 1);
        entries
            .iter_mut()
            .for_each(|x| x.offset += image_data_off as i32);

        Ok(PackEntries { entries })
    }
}
