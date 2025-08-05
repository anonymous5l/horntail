use crate::crypto::{MapleCipher, MapleVersion};
use crate::extra::bundle::Bundle;
use crate::extra::cache::EntryCache;
use crate::extra::iter::ComponentIter;
use crate::{
    AccessorBuilder, AccessorOpt, Directories, Directory, EntryKind, Error, ImageKind,
    PlainPrimitive, PlainProperties, PlainProperty, Primitive, Properties, Property, PropertyKind,
    TryFromBuilder, error,
};
use std::io::SeekFrom;
use std::path::Path;

enum EntryBuilder {
    Value,
    File(Box<dyn AccessorBuilder>),
    Bundle(Bundle),
    PlainProperties(PlainProperties),
    Complex(Box<dyn AccessorBuilder>, Bundle),
}

#[derive(Debug, Clone)]
pub enum EntryPrimitive {
    Nil,
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    String(String),
}

#[derive(Debug, Clone)]
pub enum EntryValue {
    Kind(EntryKind),
    Primitive(EntryPrimitive),
}

pub struct Entry {
    name: String,
    opt: AccessorOpt,
    value: EntryValue,
    builder: EntryBuilder,
    cipher: Box<dyn MapleCipher>,
}

impl Entry {
    pub fn from_path<P: AsRef<Path>>(
        path: P,
        cipher: Box<dyn MapleCipher>,
        version: MapleVersion,
        no_version: bool,
    ) -> Result<Self, Error> {
        let path = path.as_ref();
        let file_name = path
            .file_name()
            .and_then(|f| f.to_str())
            .ok_or(error::io_err_invalid_input())?;
        let bundle = Bundle::from_path(path, version, no_version)?.ok_or(Error::InvalidArgument)?;
        Ok(Entry {
            name: file_name.to_owned(),
            cipher,
            opt: AccessorOpt {
                offset: 0,
                ver_hash: version.hash(),
                parent_offset: 0,
            },
            value: EntryValue::Kind(EntryKind::Folder),
            builder: EntryBuilder::Bundle(bundle),
        })
    }

    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    #[inline]
    pub fn value(&self) -> &EntryValue {
        &self.value
    }

    #[inline]
    pub fn has_children(&self) -> bool {
        match &self.value {
            EntryValue::Kind(k) => match k {
                EntryKind::Image(img) => match img {
                    ImageKind::Canvas
                    | ImageKind::Video
                    | ImageKind::Sound
                    | ImageKind::RawData => true,
                    _ => false,
                },
                EntryKind::Folder | EntryKind::Property(_) => true,
            },
            EntryValue::Primitive(_) => false,
        }
    }

    #[inline]
    pub fn builder(&self) -> Option<&dyn AccessorBuilder> {
        match &self.builder {
            EntryBuilder::File(f) => Some(f.as_ref()),
            _ => None,
        }
    }

    #[inline]
    pub fn accessor_opt(&self) -> AccessorOpt {
        self.opt
    }

    #[inline]
    pub fn try_get(&self, name: &str) -> Result<Option<Entry>, Error> {
        Ok(self.try_iter()?.find(|e| e.name == name))
    }

    #[inline]
    pub fn get_exact(&self, name: &str) -> Entry {
        self.get(name).unwrap_or_else(|| panic!("entry not exists"))
    }

    #[inline]
    pub fn get(&self, name: &str) -> Option<Entry> {
        self.try_get(name)
            .unwrap_or_else(|e| panic!("get entry failed: {e}"))
    }

    pub fn try_get_by_path<P: AsRef<Path>>(&self, path: P) -> Result<Option<Entry>, Error> {
        let path = path.as_ref();
        let mut components = ComponentIter::from(path.components());
        let Some(first) = components.next() else {
            return Ok(None);
        };
        let mut cursor = self.try_get(first)?;
        while let Some(name) = components.next() {
            if let Some(entry) = cursor {
                cursor = entry.try_get(name)?;
            } else {
                return Ok(None);
            }
        }
        Ok(cursor)
    }

    #[inline]
    pub fn get_by_path<P: AsRef<Path>>(&self, path: P) -> Option<Entry> {
        self.try_get_by_path(path)
            .unwrap_or_else(|e| panic!("get_by_path: {e}"))
    }

    #[inline]
    pub fn get_by_path_exact<P: AsRef<Path>>(&self, path: P) -> Entry {
        self.get_by_path(path)
            .unwrap_or_else(|| panic!("path not exists"))
    }

    #[inline]
    pub fn try_to<'a, E: Into<Error>, T: TryFrom<&'a Entry, Error = E>>(
        &'a self,
    ) -> Result<T, Error> {
        T::try_from(self).map_err(|e| e.into())
    }

    #[inline]
    pub fn to<'a, E: Into<Error>, T: TryFrom<&'a Entry, Error = E>>(&'a self) -> T {
        self.try_to().unwrap_or_else(|e| panic!("to: {e}"))
    }

    /// `try_iter` don't cache result. each call will access IO. be careful.
    pub fn try_iter<'a>(&'a self) -> Result<Box<dyn Iterator<Item = Entry> + 'a>, Error> {
        let (builder, bundle) = match &self.builder {
            EntryBuilder::File(builder) => match self.value {
                EntryValue::Kind(kind) => match kind {
                    EntryKind::Folder => (Some(builder), None),
                    EntryKind::Image(img) => {
                        return match img {
                            ImageKind::Canvas | ImageKind::Video => {
                                let mut accessor = builder.accessor();
                                accessor.seek(SeekFrom::Start(self.opt.offset as u64));
                                let Some(props) = Properties::builtin(self.opt, accessor.as_mut())?
                                else {
                                    return Ok(Box::new(std::iter::empty()));
                                };
                                Ok(Box::new(
                                    props
                                        .into_inner()
                                        .into_iter()
                                        .map(|p| property_to_entry(p, self, builder.as_ref())),
                                ))
                            }
                            ImageKind::Sound | ImageKind::RawData => {
                                let mut accessor = builder.accessor();
                                accessor.seek(SeekFrom::Start(self.opt.offset as u64));
                                let Some(props) =
                                    Properties::optional(self.opt, accessor.as_mut())?
                                else {
                                    return Ok(Box::new(std::iter::empty()));
                                };
                                Ok(Box::new(
                                    props
                                        .into_inner()
                                        .into_iter()
                                        .map(|p| property_to_entry(p, self, builder.as_ref())),
                                ))
                            }
                            _ => Ok(Box::new(std::iter::empty())),
                        };
                    }
                    EntryKind::Property(props) => {
                        return match props {
                            PropertyKind::Plain => Ok(Box::new(
                                PlainProperties::try_from_builder(self.opt, builder.as_ref())?
                                    .into_inner()
                                    .into_iter()
                                    .map(|p| plain_property_to_entry(p, self)),
                            )),
                            PropertyKind::Encode => Ok(Box::new(
                                Properties::try_from_builder(self.opt, builder.as_ref())?
                                    .into_inner()
                                    .into_iter()
                                    .map(|p| property_to_entry(p, self, builder.as_ref())),
                            )),
                        };
                    }
                },
                _ => {
                    return Ok(Box::new(std::iter::empty()));
                }
            },
            EntryBuilder::PlainProperties(plain_props) => {
                return Ok(Box::new(
                    plain_props
                        .iter()
                        .cloned()
                        .map(|p| plain_property_to_entry(p, self)),
                ));
            }
            EntryBuilder::Value => return Ok(Box::new(std::iter::empty())),
            EntryBuilder::Bundle(bundle) => match self.value {
                EntryValue::Kind(EntryKind::Folder) => (None, Some(bundle)),
                _ => {
                    return Err(Error::UnexpectedData(format!(
                        "bundle builder parse `{:?}`",
                        self.value
                    )));
                }
            },
            EntryBuilder::Complex(builder, bundle) => match self.value {
                EntryValue::Kind(EntryKind::Folder) => (Some(builder), Some(bundle)),
                _ => {
                    return Err(Error::UnexpectedData(format!(
                        "complex builder parse `{:?}`",
                        self.value
                    )));
                }
            },
        };

        // only process `Folder` kind

        let mut entries = if let Some(builder) = builder {
            directories_to_entries(self, self.opt, builder.as_ref(), bundle)?
        } else {
            vec![]
        };

        if let Some(bundle) = bundle {
            for (opt, builder) in bundle.builders(self.cipher.as_ref())? {
                entries.extend(directories_to_entries(
                    self,
                    opt,
                    builder.as_ref(),
                    Some(bundle),
                )?);
            }
        }

        Ok(Box::new(entries.into_iter()))
    }

    #[inline]
    pub fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = Entry> + 'a> {
        self.try_iter().unwrap_or_else(|e| panic!("iter: {e}"))
    }

    #[inline]
    pub fn try_into_cache(self) -> Result<EntryCache, Error> {
        EntryCache::try_from(self)
    }

    #[inline]
    pub fn into_cache(self) -> EntryCache {
        EntryCache::try_from(self).unwrap_or_else(|e| panic!("into_cache error: {e}"))
    }
}

#[inline]
fn directories_to_entries(
    entry: &Entry,
    opt: AccessorOpt,
    builder: &dyn AccessorBuilder,
    bundle: Option<&Bundle>,
) -> Result<Vec<Entry>, Error> {
    Ok(Directories::try_from_builder(opt, builder)?
        .into_inner()
        .into_iter()
        .map(|x| {
            let sub = bundle.and_then(|bundle| bundle.load_by_name(x.name.as_str()));
            directory_to_entry(x, entry, builder, sub)
        })
        .collect::<Vec<_>>())
}

#[inline]
fn directory_to_entry(
    dir: Directory,
    entry: &Entry,
    builder: &dyn AccessorBuilder,
    bundle: Option<Bundle>,
) -> Entry {
    Entry {
        name: dir.name,
        opt: entry.opt.clone_with_parent(dir.offset, dir.parent_offset),
        value: EntryValue::Kind(dir.kind),
        builder: match bundle {
            Some(b) => EntryBuilder::Complex(builder.clone_boxed(), b),
            None => EntryBuilder::File(builder.clone_boxed()),
        },
        cipher: entry.cipher.clone_boxed(),
    }
}

#[inline]
fn property_to_entry(prop: Property, entry: &Entry, builder: &dyn AccessorBuilder) -> Entry {
    let mut offset = prop.offset;
    let value = match prop.value {
        Primitive::Nil => EntryValue::Primitive(EntryPrimitive::Nil),
        Primitive::Int16(i) => EntryValue::Primitive(EntryPrimitive::Int16(i)),
        Primitive::Int32(i) => EntryValue::Primitive(EntryPrimitive::Int32(i)),
        Primitive::Int64(i) => EntryValue::Primitive(EntryPrimitive::Int64(i)),
        Primitive::Float32(f) => EntryValue::Primitive(EntryPrimitive::Float32(f)),
        Primitive::Float64(f) => EntryValue::Primitive(EntryPrimitive::Float64(f)),
        Primitive::String(s) => EntryValue::Primitive(EntryPrimitive::String(s)),
        Primitive::Image(img) => {
            offset = img.offset;
            EntryValue::Kind(img.kind)
        }
    };
    Entry {
        name: prop.name.to_owned(),
        opt: entry.opt.clone_with(offset),
        value,
        builder: EntryBuilder::File(builder.clone_boxed()),
        cipher: entry.cipher.clone_boxed(),
    }
}

#[inline]
fn plain_property_to_entry(prop: PlainProperty, entry: &Entry) -> Entry {
    match prop.value {
        PlainPrimitive::Value(val) => Entry {
            name: prop.name,
            opt: entry.opt,
            value: EntryValue::Primitive(EntryPrimitive::String(val)),
            builder: EntryBuilder::Value,
            cipher: entry.cipher.clone_boxed(),
        },
        PlainPrimitive::Nested(nested) => Entry {
            name: prop.name,
            opt: entry.opt,
            value: EntryValue::Kind(EntryKind::Property(PropertyKind::Plain)),
            builder: EntryBuilder::PlainProperties(nested),
            cipher: entry.cipher.clone_boxed(),
        },
    }
}
