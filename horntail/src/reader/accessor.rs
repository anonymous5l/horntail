use crate::crypto::MapleCipher;
use crate::error::Error;
use std::io::SeekFrom;

macro_rules! buf_try_get_impl {
    ($this:ident, $typ:tt::$conv:tt) => {{
        const SIZE: usize = core::mem::size_of::<$typ>();
        let mut buf = [0; SIZE];
        $this.read(&mut buf)?;
        return Ok($typ::$conv(buf));
    }};
}

macro_rules! buf_get_impl {
    ($this:ident, $typ:tt::$conv:tt) => {{
        return (|| buf_try_get_impl!($this, $typ::$conv))()
            .unwrap_or_else(|e: crate::error::Error| panic!("{e}"));
    }};
    ($this:ident, [$($t:ty,)+]) => {{}};
}

#[derive(Debug)]
pub enum VarKind<T> {
    Positive(T),
    Negative(T),
}

pub enum StringKind {
    Zeroed,
    UTF8(Vec<u8>),
    UTF16(Vec<u16>),
}

#[allow(clippy::len_without_is_empty)]
pub trait Accessor: MapleCipher {
    fn pos(&self) -> usize;
    fn len(&self) -> usize;
    fn try_seek(&mut self, style: SeekFrom) -> Result<u64, Error>;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error>;

    #[inline]
    fn seek(&mut self, style: SeekFrom) {
        self.try_seek(style).unwrap_or_else(|e| panic!("seek: {e}"));
    }

    #[inline]
    fn has_remaining(&self) -> bool {
        self.remaining() > 0
    }

    #[inline]
    fn advance(&mut self, nbytes: usize) {
        self.seek(SeekFrom::Current(nbytes as i64))
    }

    #[inline]
    fn remaining(&self) -> usize {
        let len = self.len();
        if len > 0 { len - self.pos() } else { len }
    }

    #[inline]
    fn copy_to_slice(&mut self, dst: &mut [u8]) {
        let n = self.read(dst).unwrap_or_else(|e| panic!("read: {e}"));
        if n != dst.len() {
            panic!("copy_to_slice: {n} != {}", dst.len());
        }
    }

    #[inline]
    fn copy_to_vec(&mut self, n: usize) -> Vec<u8> {
        let mut buffer = vec![0; n];
        self.copy_to_slice(&mut buffer);
        buffer
    }

    #[inline]
    fn get_i8(&mut self) -> i8 {
        self.get_u8() as i8
    }

    #[inline]
    fn get_u8(&mut self) -> u8 {
        let mut temp = [0; 1];
        self.copy_to_slice(&mut temp); // do the advance
        temp[0]
    }

    fn get_i16_le(&mut self) -> i16 {
        buf_get_impl!(self, i16::from_le_bytes)
    }

    fn get_u16_le(&mut self) -> u16 {
        buf_get_impl!(self, u16::from_le_bytes)
    }

    fn get_i32_le(&mut self) -> i32 {
        buf_get_impl!(self, i32::from_le_bytes)
    }

    fn get_u32_le(&mut self) -> u32 {
        buf_get_impl!(self, u32::from_le_bytes)
    }

    fn get_i64_le(&mut self) -> i64 {
        buf_get_impl!(self, i64::from_le_bytes)
    }

    fn get_u64_le(&mut self) -> u64 {
        buf_get_impl!(self, u64::from_le_bytes)
    }

    #[inline]
    fn get_f32_le(&mut self) -> f32 {
        f32::from_bits(self.get_u32_le())
    }

    #[inline]
    fn get_f64_le(&mut self) -> f64 {
        f64::from_bits(self.get_u64_le())
    }

    #[inline]
    fn get_var_i32_le(&mut self) -> i32 {
        let num = self.get_i8();
        if num == i8::MIN {
            self.get_i32_le()
        } else {
            num as i32
        }
    }

    #[inline]
    fn get_var_u32_le_abs(&mut self) -> VarKind<u32> {
        let v = self.get_i8();

        let size = if v == i8::MIN || v == i8::MAX {
            self.get_u32_le()
        } else {
            v.unsigned_abs() as u32
        };

        if v.is_negative() {
            VarKind::Negative(size)
        } else {
            VarKind::Positive(size)
        }
    }

    #[inline]
    fn get_var_i64_le(&mut self) -> i64 {
        let num = self.get_i8();
        if num == i8::MIN {
            self.get_i64_le()
        } else {
            num as i64
        }
    }

    #[inline]
    fn get_var_i64_le_abs(&mut self) -> VarKind<i64> {
        let v = self.get_var_i64_le();
        if v.is_negative() {
            return VarKind::Negative(v);
        }
        VarKind::Positive(v)
    }

    #[inline]
    fn get_var_f32_le(&mut self) -> f32 {
        let num = self.get_i8();
        if num == i8::MIN {
            self.get_f32_le()
        } else {
            num as f32
        }
    }

    #[inline]
    fn get_utf8_string(&mut self, size: usize) -> String {
        let mut entry_name_raw = vec![0u8; size];
        self.copy_to_slice(&mut entry_name_raw);
        String::from_utf8(entry_name_raw).unwrap_or_else(|e| panic!("decode utf8 {e}"))
    }

    #[inline]
    fn get_utf16_string(&mut self, size: usize) -> String {
        let mut entry_name_raw = vec![0u8; size * 2];
        self.copy_to_slice(&mut entry_name_raw);
        let mut str = String::with_capacity(size);
        char::decode_utf16(
            entry_name_raw
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes(c.try_into().unwrap())),
        )
        .for_each(|c| {
            str.push(c.unwrap());
        });
        str
    }

    #[inline]
    fn decrypt_to_slice(&mut self, dst: &mut [u8]) {
        self.copy_to_slice(dst);
        self.crypt(dst);
    }

    #[inline]
    fn decrypt_string_slice(&mut self) -> Result<String, Error> {
        let kind = self.get_var_u32_le_abs();
        let size = match kind {
            VarKind::Negative(size) => size as usize,
            VarKind::Positive(size) => (size * 2) as usize,
        };
        if size == 0 {
            return Ok(String::with_capacity(0));
        }

        let mut chunk = self.copy_to_vec(size);

        match kind {
            VarKind::Negative(_) => {
                chunk
                    .iter_mut()
                    .enumerate()
                    .for_each(|(i, b)| *b ^= i.wrapping_add(0xaa) as u8);
                self.crypt(&mut chunk);

                // nx does put 0xd7 latin1 char in to bytes that is so grouse
                let (str, _, had_error) = encoding_rs::WINDOWS_1252.decode(&chunk);
                if had_error {
                    return Err(Error::InvalidCharacter);
                }
                Ok(str.into_owned())
            }
            VarKind::Positive(_) => {
                chunk.chunks_exact_mut(2).enumerate().for_each(|(i, b)| {
                    let b16 = u16::from_le_bytes([b[0], b[1]]);
                    let result = (b16 ^ i.wrapping_add(0xaaaa) as u16).to_le_bytes();
                    (b[0], b[1]) = (result[0], result[1]);
                });
                self.crypt(&mut chunk);
                Ok(char::decode_utf16(
                    chunk
                        .chunks_exact(2)
                        .map(|x| u16::from_le_bytes([x[0], x[1]])),
                )
                .collect::<Result<String, _>>()?)
            }
        }
    }

    #[inline]
    fn get_decrypt_string(&mut self) -> String {
        self.try_get_decrypt_string()
            .unwrap_or_else(|e| panic!("get_decrypt_string: {e}"))
    }

    fn try_get_decrypt_string(&mut self) -> crate::error::Result<String> {
        self.decrypt_string_slice()
    }

    #[inline]
    fn get_image_string(&mut self, parent_offset: usize) -> String {
        self.try_get_image_string(parent_offset)
            .unwrap_or_else(|e| panic!("get_image_string: {e}"))
    }

    #[inline]
    fn get_uol_string(&mut self, parent_offset: usize) -> String {
        self.try_get_uol_string(parent_offset)
            .unwrap_or_else(|e| panic!("get_uol_string: {e}"))
    }

    fn try_get_uol_string(&mut self, parent_offset: usize) -> Result<String, Error> {
        match self.get_u8() {
            0x00 => Ok(self.get_decrypt_string()),
            0x01 => {
                let offset = self.get_i32_le() as usize + parent_offset;
                Ok(seek_back(
                    self,
                    SeekFrom::Start(offset as u64),
                    |accessor| accessor.get_decrypt_string(),
                ))
            }
            flag => Err(Error::UnexpectedData(format!(
                "unexpected uol string flag {flag}"
            ))),
        }
    }

    fn try_get_image_string(&mut self, parent_offset: usize) -> Result<String, u8> {
        match self.get_u8() {
            0x73 => Ok(self.get_decrypt_string()),
            0x1b => {
                let offset = self.get_i32_le() as usize + parent_offset;
                Ok(seek_back(
                    self,
                    SeekFrom::Start(offset as u64),
                    |accessor| accessor.get_decrypt_string(),
                ))
            }
            flag => Err(flag),
        }
    }
}

#[inline]
pub fn seek_back<A, T, F>(accessor: &mut A, style: SeekFrom, mut f: F) -> T
where
    A: Accessor + ?Sized,
    F: FnMut(&mut A) -> T,
{
    let anchor = accessor.pos() as u64;
    accessor.seek(style);
    let result = f(accessor);
    accessor.seek(SeekFrom::Start(anchor));
    result
}
