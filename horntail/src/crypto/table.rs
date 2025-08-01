use aes::cipher::{BlockEncrypt, Key, KeyInit};
use aes::{Aes256, Block};
use std::cell::RefCell;
use std::rc::Rc;

const BLOCK_SIZE: usize = 16;
const IV_SIZE: usize = 4;

pub const AES_KEY: [u8; 32] = [
    0x13, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0xB4, 0x00, 0x00, 0x00,
    0x1B, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x00, 0x33, 0x00, 0x00, 0x00, 0x52, 0x00, 0x00, 0x00,
];

pub trait MapleCipher {
    fn crypt(&mut self, dst: &mut [u8]);

    fn clone_boxed(&self) -> Box<dyn MapleCipher>;

    fn into_boxed(self) -> Box<dyn MapleCipher>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

#[derive(Clone)]
pub struct MapleTableNone;

impl MapleCipher for MapleTableNone {
    #[inline(always)]
    fn crypt(&mut self, _: &mut [u8]) {}

    #[inline(always)]
    fn clone_boxed(&self) -> Box<dyn MapleCipher> {
        Box::new(Self)
    }
}

#[derive(Clone)]
pub struct MapleTable {
    aes: Aes256,
    tab: Vec<u8>,
}

impl MapleTable {
    pub fn new(iv: [u8; IV_SIZE]) -> Self {
        let mut tab = std::iter::repeat_n(iv, BLOCK_SIZE / IV_SIZE)
            .flatten()
            .collect::<Vec<_>>();
        let aes = aes::Aes256::new(Key::<Aes256>::from_slice(&AES_KEY));
        // encrypt first block
        aes.encrypt_block(Block::from_mut_slice(&mut tab));
        Self { aes, tab }
    }

    #[inline]
    fn extend_table(&mut self, size: usize) {
        let cipher = &self.aes;
        let reserve_size = size.saturating_sub(self.tab.len());
        if reserve_size == 0 {
            return;
        }
        let aligned_size = (reserve_size + BLOCK_SIZE - 1) & !(BLOCK_SIZE - 1);
        self.tab.reserve(aligned_size);
        let mut block = [0u8; BLOCK_SIZE];
        (self.tab.len()..self.tab.capacity())
            .step_by(BLOCK_SIZE)
            .for_each(|i| {
                block.copy_from_slice(&self.tab[i - BLOCK_SIZE..i]);
                cipher.encrypt_block(Block::from_mut_slice(&mut block));
                self.tab.extend_from_slice(&block);
            });
    }

    fn alloc(&mut self, size: usize) -> &[u8] {
        if self.tab.len() < size {
            self.extend_table(size);
        }
        &self.tab.as_slice()[..size]
    }
}

impl MapleCipher for MapleTable {
    fn crypt(&mut self, dst: &mut [u8]) {
        let table = self.alloc(dst.len());
        dst.iter_mut().zip(table).for_each(|(dst, t)| *dst ^= *t);
    }

    #[inline(always)]
    fn clone_boxed(&self) -> Box<dyn MapleCipher> {
        Box::new(self.clone())
    }
}

impl<T> MapleCipher for Rc<RefCell<T>>
where
    T: MapleCipher + 'static,
{
    fn crypt(&mut self, dst: &mut [u8]) {
        let mut borrowed = self.borrow_mut();
        T::crypt(&mut borrowed, dst)
    }

    #[inline(always)]
    fn clone_boxed(&self) -> Box<dyn MapleCipher> {
        Box::new(self.clone())
    }
}
