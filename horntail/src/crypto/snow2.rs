use crate::crypto::snow2_box::{ALPHA_INV_MUL, ALPHA_MUL, S0, S1, S2, S3};

const U32_SIZE: usize = size_of::<u32>();

#[derive(Default, Debug, Clone)]
pub struct Snow2 {
    state: [u32; 16],
    r1: u32,
    r2: u32,
    cur: usize,
}

macro_rules! u32_from_array {
    ($key:expr) => {
        (($key[0] as i8) as u32) << 24
            | (($key[1] as i8) as u32) << 16
            | (($key[2] as i8) as u32) << 8
            | (($key[3] as i8) as u32)
    };
}

impl Snow2 {
    pub fn new<const SIZE: usize>(key: [u8; SIZE], iv: [u32; 4]) -> Self {
        let mut state = [0; 16];

        if SIZE == 16 {
            state[15] = u32_from_array!(&key[..4]);
            state[14] = u32_from_array!(&key[4..8]);
            state[13] = u32_from_array!(&key[8..12]);
            state[12] = u32_from_array!(&key[12..16]);
            state[11] = !state[15];
            state[10] = !state[14];
            state[9] = !state[13];
            state[8] = !state[12];

            state[7] = state[15];
            state[6] = state[14];
            state[5] = state[13];
            state[4] = state[12];

            state[3] = !state[15];
            state[2] = !state[14];
            state[1] = !state[13];
            state[0] = !state[12];
        } else if SIZE == 32 {
            state[15] = u32_from_array!(&key[..4]);
            state[14] = u32_from_array!(&key[4..8]);
            state[13] = u32_from_array!(&key[8..12]);
            state[12] = u32_from_array!(&key[12..16]);
            state[11] = u32_from_array!(&key[16..20]);
            state[10] = u32_from_array!(&key[20..24]);
            state[9] = u32_from_array!(&key[24..28]);
            state[8] = u32_from_array!(&key[28..32]);

            state[7] = state[15];
            state[6] = state[14];
            state[5] = state[13];
            state[4] = state[12];

            state[3] = !state[11];
            state[2] = !state[10];
            state[1] = !state[9];
            state[0] = !state[8];
        } else {
            panic!("invalid key length");
        }

        state[15] ^= iv[0];
        state[12] ^= iv[1];
        state[10] ^= iv[2];
        state[9] ^= iv[3];

        let mut r1 = 0;
        let mut r2 = 0;

        (0..32).for_each(|index| {
            (r1, r2) = round_state(&mut state, r1, r2, index);
        });

        Snow2 {
            state,
            r1,
            r2,
            cur: 0,
        }
    }

    #[inline]
    pub fn crypt(&mut self, dst: &mut [u8], is_enc: bool) {
        if dst.len() % U32_SIZE != 0 {
            panic!("invalid destination length");
        }

        dst.chunks_mut(U32_SIZE).zip(self).for_each(|(dst, k)| {
            let dst_len = dst.len();
            let u32val = u32::from_le_bytes([dst[0], dst[1], dst[2], dst[3]]);
            let result = if is_enc {
                u32val.wrapping_add(k).to_le_bytes()
            } else {
                u32val.wrapping_sub(k).to_le_bytes()
            };
            dst.copy_from_slice(&result[..dst_len]);
        });
    }
}

impl Iterator for Snow2 {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        let state_size = self.state.len();
        let cur = self.cur;
        self.state[cur] = a_mul(self.state[cur])
            ^ self.state[(self.cur + 2) % state_size]
            ^ ainv_mul(self.state[(self.cur + 11) % state_size]);
        let fsm = self
            .r2
            .wrapping_add(self.state[(self.cur + 5) % state_size]);
        self.r2 = sbox(self.r1);
        self.r1 = fsm;
        self.cur = (self.cur + 1) % state_size;
        Some(self.r1.wrapping_add(self.state[cur]) ^ self.r2 ^ self.state[self.cur])
    }
}

#[inline]
fn round_state(state: &mut [u32], r1: u32, r2: u32, index: usize) -> (u32, u32) {
    let ss = state.len();
    state[index % ss] = a_mul(state[index % ss])
        ^ state[(index + 2) % ss]
        ^ ainv_mul(state[(index + 11) % ss])
        ^ (r1.wrapping_add(state[(index + 15) % ss]) ^ r2);
    (r2.wrapping_add(state[(index + 5) % ss]), sbox(r1))
}

#[inline]
fn sbox(n: u32) -> u32 {
    S0[(n & 0xff) as usize]
        ^ S1[((n >> 8) & 0xff) as usize]
        ^ S2[((n >> 16) & 0xff) as usize]
        ^ S3[((n >> 24) & 0xff) as usize]
}

#[inline]
fn a_mul(n: u32) -> u32 {
    (n << 8) ^ ALPHA_MUL[(n >> 24) as usize]
}

#[inline]
fn ainv_mul(n: u32) -> u32 {
    (n >> 8) ^ ALPHA_INV_MUL[(n & 0xff) as usize]
}
