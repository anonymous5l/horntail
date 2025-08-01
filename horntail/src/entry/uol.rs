use crate::reader::Accessor;
use crate::{AccessorOpt, Error, TryFromAccessor};

#[derive(Clone)]
pub struct UOL {
    pub flag: u8,
    pub path: String,
}

impl TryFromAccessor for UOL {
    type Error = Error;

    fn try_from_accessor(
        opt: AccessorOpt,
        accessor: &mut dyn Accessor,
    ) -> Result<Self, Self::Error> {
        let flag = accessor.get_u8();
        let path = accessor.try_get_uol_string(opt.parent_offset)?;
        Ok(UOL { flag, path })
    }
}
