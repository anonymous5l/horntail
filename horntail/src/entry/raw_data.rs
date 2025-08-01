use crate::reader::Accessor;
use crate::{AccessorOpt, Error, Properties, TryFromAccessor};

#[derive(Clone)]
pub struct RawData {
    pub properties: Option<Properties>,
    pub data: Vec<u8>,
}

impl TryFromAccessor for RawData {
    type Error = Error;

    fn try_from_accessor(
        opt: AccessorOpt,
        accessor: &mut dyn Accessor,
    ) -> Result<Self, Self::Error> {
        let properties = Properties::optional(opt, accessor)?;
        let data_size = accessor.get_var_i32_le() as usize;
        let mut buffer = vec![0; data_size];
        accessor.read(&mut buffer)?;
        Ok(RawData {
            properties,
            data: buffer,
        })
    }
}
