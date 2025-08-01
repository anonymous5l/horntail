use crate::reader::Accessor;
use crate::{AccessorOpt, Error, Image, TryFromAccessor};
use std::io::SeekFrom;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct Vector2D {
    pub x: i32,
    pub y: i32,
}

impl TryFromAccessor for Vector2D {
    type Error = Error;

    fn try_from_accessor(_: AccessorOpt, accessor: &mut dyn Accessor) -> Result<Self, Self::Error> {
        let x = accessor.get_var_i32_le();
        let y = accessor.get_var_i32_le();
        Ok(Vector2D { x, y })
    }
}

#[derive(Clone)]
pub struct Convex2D {
    pub convexities: Vec<Vector2D>,
}

impl TryFromAccessor for Convex2D {
    type Error = Error;

    fn try_from_accessor(
        opt: AccessorOpt,
        accessor: &mut dyn Accessor,
    ) -> Result<Self, Self::Error> {
        let elem_size = accessor.get_var_i32_le();
        let mut convexities = Vec::with_capacity(elem_size as usize);
        for _ in 0..elem_size {
            let img = Image::try_from_accessor(opt.clone_with(accessor.pos()), accessor)?;
            accessor.seek(SeekFrom::Start(img.offset as u64));
            convexities.push(Vector2D::try_from_accessor(
                opt.clone_with(img.offset),
                accessor,
            )?);
        }
        Ok(Convex2D { convexities })
    }
}
