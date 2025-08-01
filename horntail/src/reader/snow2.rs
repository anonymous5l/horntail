use crate::Error;
use crate::crypto::Snow2 as Snow2Crypto;
use std::io;
use std::io::Read;

const SNOW2_128_KEY_SIZE: usize = 16;
const BLOCK_SIZE: usize = 4;

pub struct Snow2<T = ()> {
    base: T,
    buf: Vec<u8>,
    algo: Snow2Crypto,
}

impl<T: Clone> Clone for Snow2<T> {
    fn clone(&self) -> Self {
        Snow2 {
            base: self.base.clone(),
            buf: self.buf.clone(),
            algo: self.algo.clone(),
        }
    }
}

impl<T> Snow2<T> {
    #[inline]
    pub fn new(base: T, cipher: [u8; SNOW2_128_KEY_SIZE]) -> Snow2<T> {
        Snow2 {
            base,
            buf: Vec::new(),
            algo: Snow2Crypto::new(cipher, [0; 4]),
        }
    }

    #[inline]
    pub fn with_buffer(
        base: T,
        src: &[u8],
        cipher: [u8; SNOW2_128_KEY_SIZE],
    ) -> crate::error::Result<Snow2<T>> {
        if src.len() % BLOCK_SIZE != 0 {
            return Err(Error::UnexpectedData("invalid encrypt buffer".to_owned()));
        }
        let size = src.len();
        let mut snow = Snow2 {
            base,
            buf: vec![0; size],
            algo: Snow2Crypto::new(cipher, [0; 4]),
        };
        snow.buf.copy_from_slice(src);
        snow.algo.crypt(&mut snow.buf[..size], false);
        Ok(snow)
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.base
    }
}

impl<T: Read> Read for Snow2<T> {
    fn read(&mut self, dst: &mut [u8]) -> io::Result<usize> {
        let dst_size = dst.len();
        let mut dst = dst;

        let buf = &mut self.buf;
        let buf_size = buf.len();

        if dst_size <= buf_size {
            dst.copy_from_slice(&buf[..dst_size]);
            buf.copy_within(dst_size.., 0);
            unsafe { buf.set_len(buf_size - dst_size) };
            return Ok(dst_size);
        } else if buf_size > 0 {
            dst[..buf_size].copy_from_slice(&buf[..buf_size]);
            dst = &mut dst[buf_size..];
            buf.clear();
        }

        let aligned_size = align_size(dst.len());

        if aligned_size > dst.len() {
            buf.resize(aligned_size, 0);
            self.base.read_exact(buf.as_mut_slice())?;
            self.algo.crypt(buf, false);
            dst.copy_from_slice(&buf[..dst.len()]);
            buf.copy_within(dst.len().., 0);
            unsafe { buf.set_len(aligned_size.saturating_sub(dst.len())) };
        } else {
            self.base.read_exact(dst)?;
            self.algo.crypt(dst, false);
        }

        Ok(dst_size)
    }
}

#[inline]
pub(crate) fn align_size(size: usize) -> usize {
    (size + BLOCK_SIZE - 1) & !(BLOCK_SIZE - 1)
}
