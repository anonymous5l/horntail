#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct MapleVersion(u16);

impl From<u16> for MapleVersion {
    fn from(value: u16) -> Self {
        MapleVersion(value)
    }
}

impl From<MapleVersion> for u16 {
    fn from(value: MapleVersion) -> Self {
        value.0
    }
}

impl MapleVersion {
    #[inline]
    pub fn into_inner(self) -> u16 {
        self.0
    }

    #[inline]
    pub fn hash(self) -> u16 {
        self.0
            .to_string()
            .into_bytes()
            .into_iter()
            .fold(0, |hash, v| (hash << 5) + v.wrapping_add(1) as u16)
    }

    #[inline]
    pub fn hash_enc(self) -> u16 {
        let hash = self.hash();
        (0..4).fold(0xff, |enc, i| {
            enc ^ (hash.checked_shr(i << 3).unwrap_or(0) & 0xff)
        })
    }
}
