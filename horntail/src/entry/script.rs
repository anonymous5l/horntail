use crate::crypto::MapleCipher;
use crate::reader::Accessor;
use crate::{AccessorOpt, Error, TryFromAccessor};

pub struct Script {
    pub data: Vec<u8>,
}

impl Script {
    #[inline]
    pub fn decrypt_data(mut self, cipher: &mut dyn MapleCipher) -> Vec<u8> {
        cipher.crypt(&mut self.data);
        self.data
    }

    #[inline]
    pub fn decrypt_to_string(
        mut self,
        cipher: &mut dyn MapleCipher,
    ) -> Result<String, std::string::FromUtf8Error> {
        cipher.crypt(&mut self.data);
        String::from_utf8(self.data)
    }
}

impl TryFromAccessor for Script {
    type Error = Error;

    fn try_from_accessor(_: AccessorOpt, accessor: &mut dyn Accessor) -> Result<Self, Self::Error> {
        let flag = accessor.get_u8();
        let script = if flag == 0x01 {
            let size = accessor.get_var_i32_le();
            accessor.copy_to_vec(size as usize)
        } else {
            return Err(Error::UnexpectedData(format!(
                "unexpected script flag `{flag}`"
            )));
        };
        Ok(Script { data: script })
    }
}
