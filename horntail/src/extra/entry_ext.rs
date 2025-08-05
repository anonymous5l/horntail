use crate::extra::EntryCache;
use crate::extra::entry::{Entry, EntryPrimitive, EntryValue};
use crate::{
    Canvas, CanvasAttribute, Convex2D, EntryKind, Error, ImageKind, RawData, Script, Sound,
    SoundAttribute, TryFromBuilder, UOL, Vector2D,
};

macro_rules! impl_try_from_entry_cache {
    ($typ:tt) => {
        impl<'a> TryFrom<&'a EntryCache> for $typ {
            type Error = Error;

            fn try_from(entry: &'a EntryCache) -> Result<Self, Self::Error> {
                Self::try_from(entry.as_ref())
            }
        }
    };
}
macro_rules! impl_try_from_numerics {
    ($($typ:tt,)*) => {
        $(
        impl<'a> TryFrom<&'a Entry> for $typ {
            type Error = Error;

            fn try_from(entry: &'a Entry) -> Result<Self, Self::Error> {
                let value = entry.value();
                match value {
                    EntryValue::Kind(_) => Err(Error::InvalidDataType),
                    EntryValue::Primitive(primitive) => match primitive {
                        EntryPrimitive::Int16(i) => Ok(*i as $typ),
                        EntryPrimitive::Int32(i) => Ok(*i as $typ),
                        EntryPrimitive::Int64(i) => Ok(*i as $typ),
                        EntryPrimitive::Float32(f) => Ok(*f as $typ),
                        EntryPrimitive::Float64(f) => Ok(*f as $typ),
                        EntryPrimitive::String(s) => {
                            Ok(s.parse().map_err(|_| Error::InvalidDataType)?)
                        }
                        _ => Err(Error::InvalidDataType),
                    },
                }
            }
        }

        impl_try_from_entry_cache!($typ);
        )*
    };
}

impl_try_from_numerics!(i8, i16, i32, i64, f32, f64, u8, u16, u32, u64, isize, usize,);

impl<'a> TryFrom<&'a Entry> for String {
    type Error = Error;

    fn try_from(entry: &'a Entry) -> Result<Self, Self::Error> {
        let value = entry.value();
        match value {
            EntryValue::Kind(_) => Err(Error::InvalidDataType),
            EntryValue::Primitive(primitive) => match primitive {
                EntryPrimitive::Int16(i) => Ok(i.to_string()),
                EntryPrimitive::Int32(i) => Ok(i.to_string()),
                EntryPrimitive::Int64(i) => Ok(i.to_string()),
                EntryPrimitive::Float32(f) => Ok(f.to_string()),
                EntryPrimitive::Float64(f) => Ok(f.to_string()),
                EntryPrimitive::String(s) => Ok(s.clone()),
                _ => Err(Error::InvalidDataType),
            },
        }
    }
}

impl_try_from_entry_cache!(String);

impl<'a> TryFrom<&'a Entry> for bool {
    type Error = Error;

    fn try_from(entry: &'a Entry) -> Result<Self, Self::Error> {
        let value = entry.value();
        match value {
            EntryValue::Kind(_) => Err(Error::InvalidDataType),
            EntryValue::Primitive(primitive) => match primitive {
                EntryPrimitive::Nil => Ok(false),
                EntryPrimitive::Int16(i) => Ok(*i != 0),
                EntryPrimitive::Int32(i) => Ok(*i != 0),
                EntryPrimitive::Int64(i) => Ok(*i != 0),
                EntryPrimitive::Float32(f) => Ok(*f != 0.0),
                EntryPrimitive::Float64(f) => Ok(*f != 0.0),
                EntryPrimitive::String(s) => Ok(s
                    .chars()
                    .next()
                    .map(|c| c == 't' || c == 'T')
                    .unwrap_or_default()),
            },
        }
    }
}

impl_try_from_entry_cache!(bool);

macro_rules! impl_try_from_builder {
    ([$(($typ:ty,$pattern:pat),)*]) => {
        $(
        impl<'a> TryFrom<&'a Entry> for $typ {
            type Error = <$typ as TryFromBuilder>::Error;

            fn try_from(entry: &'a Entry) -> Result<Self, Self::Error> {
                if !matches!(entry.value(), $pattern) {
                    return Err(Error::InvalidDataType);
                }
                let builder = entry.builder().ok_or(Error::InvalidDataType)?;
                Self::try_from_builder(entry.accessor_opt(), builder)
            }
        }

        impl_try_from_entry_cache!($typ);
        )*
    };
}

impl_try_from_builder!([
    (
        CanvasAttribute,
        EntryValue::Kind(EntryKind::Image(ImageKind::Canvas))
    ),
    (
        Canvas,
        EntryValue::Kind(EntryKind::Image(ImageKind::Canvas))
    ),
    (
        SoundAttribute,
        EntryValue::Kind(EntryKind::Image(ImageKind::Sound))
    ),
    (Sound, EntryValue::Kind(EntryKind::Image(ImageKind::Sound))),
    (
        RawData,
        EntryValue::Kind(EntryKind::Image(ImageKind::RawData))
    ),
    (
        Script,
        EntryValue::Kind(EntryKind::Image(ImageKind::Script))
    ),
    (
        Vector2D,
        EntryValue::Kind(EntryKind::Image(ImageKind::Vector2D))
    ),
    (
        Convex2D,
        EntryValue::Kind(EntryKind::Image(ImageKind::Convex2D))
    ),
    (UOL, EntryValue::Kind(EntryKind::Image(ImageKind::UOL))),
]);

impl<'a, T> TryFrom<&'a Entry> for Vec<T>
where
    for<'b> T: TryFrom<&'b Entry>,
    for<'b> <T as TryFrom<&'b Entry>>::Error: Into<Error>,
{
    type Error = Error;

    fn try_from(entry: &'a Entry) -> Result<Self, Self::Error> {
        let mut result = vec![];
        for x in entry.try_iter()? {
            result.push(T::try_from(&x).map_err(Into::into)?)
        }
        Ok(result)
    }
}

impl<'a, T> TryFrom<&'a EntryCache> for Vec<T>
where
    for<'b> T: TryFrom<&'b EntryCache>,
    for<'b> <T as TryFrom<&'b EntryCache>>::Error: Into<Error>,
{
    type Error = Error;

    fn try_from(entry: &'a EntryCache) -> Result<Self, Self::Error> {
        let mut result = vec![];
        for x in entry.try_iter()? {
            result.push(T::try_from(x).map_err(Into::into)?)
        }
        Ok(result)
    }
}
