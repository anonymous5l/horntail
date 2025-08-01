use crate::crypto::MapleCipher;
use crate::reader::Accessor;
use crate::{AccessorBuilder, Error};
use std::io;
use std::io::{ErrorKind, Read, SeekFrom};
use std::rc::Rc;

pub struct BinaryAccessor<T> {
    cipher: Box<dyn MapleCipher>,
    data: T,
    pos: u64,
    size: u64,
}

impl<T: AsRef<[u8]>> BinaryAccessor<T> {
    #[inline]
    pub fn new<C: MapleCipher + 'static>(cipher: C, data: T) -> BinaryAccessor<T> {
        Self::from_boxed(cipher.into_boxed(), data)
    }

    pub fn from_boxed(cipher: Box<dyn MapleCipher>, data: T) -> BinaryAccessor<T> {
        let size = data.as_ref().len() as u64;
        BinaryAccessor {
            cipher,
            data,
            pos: 0,
            size,
        }
    }
}

impl<T: AsRef<[u8]>> Read for BinaryAccessor<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let data = self.data.as_ref();
        let pos = self.pos.min(data.len() as u64);
        let (_, mut r) = data.split_at(pos as usize);
        let n = Read::read(&mut r, buf)?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl<T: AsRef<[u8]>> MapleCipher for BinaryAccessor<T> {
    #[inline]
    fn crypt(&mut self, dst: &mut [u8]) {
        self.cipher.crypt(dst);
    }

    #[inline]
    fn clone_boxed(&self) -> Box<dyn MapleCipher> {
        self.cipher.clone_boxed()
    }
}

impl<T: AsRef<[u8]>> Accessor for BinaryAccessor<T> {
    #[inline]
    fn pos(&self) -> usize {
        self.pos as usize
    }

    #[inline]
    fn len(&self) -> usize {
        self.size as usize
    }

    #[inline]
    fn try_seek(&mut self, style: SeekFrom) -> Result<u64, Error> {
        let (base_pos, offset) = match style {
            SeekFrom::Start(n) => {
                self.pos = n;
                return Ok(n);
            }
            SeekFrom::End(n) => (self.data.as_ref().len() as u64, n),
            SeekFrom::Current(n) => (self.pos, n),
        };

        match base_pos.checked_add_signed(offset) {
            Some(n) => {
                self.pos = n;
                Ok(self.pos)
            }
            None => Err(Error::IO(io::Error::new(
                ErrorKind::InvalidInput,
                "invalid seek to a negative or overflowing position",
            ))),
        }
    }

    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        Ok(Read::read(self, buf)?)
    }
}

#[derive(Clone)]
struct RcSliceWrapper(Rc<dyn AsRef<[u8]>>);

impl AsRef<[u8]> for RcSliceWrapper {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref().as_ref()
    }
}

pub struct BinaryBuilder {
    cipher: Box<dyn MapleCipher>,
    slice: RcSliceWrapper,
}

impl BinaryBuilder {
    #[inline]
    pub fn new<C: MapleCipher + 'static, T: AsRef<[u8]> + 'static>(
        cipher: C,
        source: T,
    ) -> BinaryBuilder {
        Self::from_boxed(cipher.into_boxed(), source)
    }

    pub fn from_boxed<T: AsRef<[u8]> + 'static>(
        cipher: Box<dyn MapleCipher>,
        source: T,
    ) -> BinaryBuilder {
        BinaryBuilder {
            cipher,
            slice: RcSliceWrapper(Rc::new(source)),
        }
    }
}

impl AccessorBuilder for BinaryBuilder {
    fn clone_boxed(&self) -> Box<dyn AccessorBuilder> {
        Box::new(BinaryBuilder {
            cipher: self.cipher.clone_boxed(),
            slice: self.slice.clone(),
        })
    }

    fn accessor(&self) -> Box<dyn Accessor> {
        Box::new(BinaryAccessor::from_boxed(
            self.cipher.clone_boxed(),
            self.slice.clone(),
        ))
    }
}
