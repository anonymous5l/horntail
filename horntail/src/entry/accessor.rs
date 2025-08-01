use crate::reader::Accessor;
use std::io::SeekFrom;

pub trait AccessorBuilder {
    fn into_boxed(self) -> Box<dyn AccessorBuilder>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }

    fn clone_boxed(&self) -> Box<dyn AccessorBuilder>;

    fn accessor(&self) -> Box<dyn Accessor>;
}

#[derive(Default, Debug, Copy, Clone)]
pub struct AccessorOpt {
    pub offset: usize,
    pub ver_hash: u16,
    pub parent_offset: usize,
}

impl AccessorOpt {
    pub fn clone_with(&self, offset: usize) -> AccessorOpt {
        AccessorOpt {
            ver_hash: self.ver_hash,
            parent_offset: self.parent_offset,
            offset,
        }
    }

    #[inline]
    pub fn clone_with_parent(&self, offset: usize, parent_offset: usize) -> AccessorOpt {
        AccessorOpt {
            ver_hash: self.ver_hash,
            parent_offset,
            offset,
        }
    }

    pub fn accessor(&self, builder: &dyn AccessorBuilder) -> Box<dyn Accessor> {
        let mut accessor = builder.accessor();
        accessor.seek(SeekFrom::Start(self.offset as u64));
        accessor
    }
}

pub trait FromBuilder {
    fn from_builder(opt: AccessorOpt, builder: &dyn AccessorBuilder) -> Self;
}

impl<T> FromBuilder for T
where
    T: TryFromBuilder,
    T::Error: std::fmt::Display,
{
    fn from_builder(opt: AccessorOpt, builder: &dyn AccessorBuilder) -> Self {
        T::try_from_builder(opt, builder).unwrap_or_else(|e| panic!("from builder: {e}"))
    }
}

pub trait FromAccessor {
    fn from_accessor(opt: AccessorOpt, builder: &mut dyn Accessor) -> Self;
}

impl<T> FromAccessor for T
where
    T: TryFromAccessor,
    T::Error: std::fmt::Display,
{
    fn from_accessor(opt: AccessorOpt, accessor: &mut dyn Accessor) -> Self {
        T::try_from_accessor(opt, accessor).unwrap_or_else(|e| panic!("from accessor: {e}"))
    }
}

pub trait TryFromBuilder: Sized {
    type Error;

    fn try_from_builder(
        opt: AccessorOpt,
        builder: &dyn AccessorBuilder,
    ) -> Result<Self, Self::Error>;
}

impl<T> TryFromBuilder for T
where
    T: TryFromAccessor,
    T::Error: std::fmt::Display,
{
    type Error = T::Error;

    fn try_from_builder(
        opt: AccessorOpt,
        builder: &dyn AccessorBuilder,
    ) -> Result<Self, Self::Error> {
        let mut accessor = opt.accessor(builder);
        Self::try_from_accessor(opt, accessor.as_mut())
    }
}

pub trait TryFromAccessor: Sized {
    type Error;

    fn try_from_accessor(
        opt: AccessorOpt,
        accessor: &mut dyn Accessor,
    ) -> Result<Self, Self::Error>;
}
