use crate::Error;
use crate::extra::iter::ComponentIter;
use crate::extra::{Entry, EntryValue};
use std::cell::OnceCell;
use std::path::Path;

pub struct EntryCache {
    entry: Entry,
    cache: OnceCell<ahash::HashMap<String, EntryCache>>,
}

impl AsRef<Entry> for EntryCache {
    fn as_ref(&self) -> &Entry {
        &self.entry
    }
}

impl TryFrom<Entry> for EntryCache {
    type Error = Error;

    fn try_from(entry: Entry) -> Result<Self, Self::Error> {
        let cache = Self {
            entry,
            cache: OnceCell::new(),
        };
        cache.cache_map()?;
        Ok(cache)
    }
}

impl EntryCache {
    #[inline]
    fn cache_map(&self) -> Result<&ahash::HashMap<String, EntryCache>, Error> {
        if let Some(cache) = self.cache.get() {
            return Ok(cache);
        }
        let mut cache = ahash::HashMap::default();
        self.entry.try_iter()?.for_each(|e| {
            cache.insert(
                e.name().to_owned(),
                EntryCache {
                    entry: e,
                    cache: OnceCell::new(),
                },
            );
        });
        if self.cache.set(cache).is_ok() {
            return Ok(self.cache.get().unwrap());
        }
        panic!("init cache failed");
    }

    #[inline]
    pub fn name(&self) -> &str {
        self.entry.name()
    }

    #[inline]
    pub fn value(&self) -> &EntryValue {
        self.entry.value()
    }

    #[inline]
    pub fn has_children(&self) -> bool {
        self.entry.has_children()
    }

    #[inline]
    pub fn try_get(&self, name: &str) -> Result<Option<&EntryCache>, Error> {
        Ok(self.cache_map()?.get(name))
    }

    #[inline]
    pub fn get_exact(&self, name: &str) -> &EntryCache {
        self.get(name).unwrap_or_else(|| panic!("entry not exists"))
    }

    #[inline]
    pub fn get(&self, name: &str) -> Option<&EntryCache> {
        self.try_get(name)
            .unwrap_or_else(|e| panic!("get entry failed: {e}"))
    }

    pub fn try_get_by_path<P: AsRef<Path>>(&self, path: P) -> Result<Option<&EntryCache>, Error> {
        let path = path.as_ref();
        let mut components = ComponentIter::from(path.components());
        let Some(first) = components.next() else {
            return Ok(None);
        };
        let mut cursor = self.try_get(first)?;
        for name in components {
            if let Some(entry) = cursor {
                cursor = entry.try_get(name)?;
            } else {
                return Ok(None);
            }
        }
        Ok(cursor)
    }

    #[inline]
    pub fn get_by_path<P: AsRef<Path>>(&self, path: P) -> Option<&EntryCache> {
        self.try_get_by_path(path)
            .unwrap_or_else(|e| panic!("get_by_path: {e}"))
    }

    #[inline]
    pub fn get_by_path_exact<P: AsRef<Path>>(&self, path: P) -> &EntryCache {
        self.get_by_path(path)
            .unwrap_or_else(|| panic!("path not exists"))
    }

    #[inline]
    pub fn try_to<'a, T: TryFrom<&'a EntryCache, Error = impl Into<Error>>>(
        &'a self,
    ) -> Result<T, Error> {
        T::try_from(self).map_err(|e| e.into())
    }

    #[inline]
    pub fn to<'a, T: TryFrom<&'a EntryCache, Error = impl Into<Error>>>(&'a self) -> T {
        self.try_to().unwrap_or_else(|e| panic!("to: {e}"))
    }

    #[inline]
    pub fn try_get_value<'a, T: TryFrom<&'a EntryCache, Error = impl Into<Error>>>(
        &'a self,
        name: &str,
    ) -> Result<Option<T>, Error> {
        if let Some(value) = self.try_get(name)? {
            Ok(Some(value.try_to::<T>()?))
        } else {
            Ok(None)
        }
    }

    #[inline]
    pub fn get_value<'a, T: TryFrom<&'a EntryCache, Error = impl Into<Error>>>(
        &'a self,
        name: &str,
    ) -> Option<T> {
        self.try_get_value::<T>(name)
            .unwrap_or_else(|e| panic!("get_value failed: {e}"))
    }

    #[inline]
    pub fn try_get_default<'a, T: TryFrom<&'a EntryCache, Error = impl Into<Error>> + Default>(
        &'a self,
        name: &str,
    ) -> Result<T, Error> {
        Ok(self
            .try_get_value::<T>(name)?
            .unwrap_or_else(|| T::default()))
    }

    #[inline]
    pub fn get_default<'a, T: TryFrom<&'a EntryCache, Error = impl Into<Error>> + Default>(
        &'a self,
        name: &str,
    ) -> T {
        self.try_get_default::<T>(name)
            .unwrap_or_else(|e| panic!("get_default: {e}"))
    }

    #[inline]
    pub fn try_get_or_else<'a, T: TryFrom<&'a EntryCache, Error = impl Into<Error>>>(
        &'a self,
        name: &str,
        f: impl FnOnce() -> Result<T, T::Error>,
    ) -> Result<T, Error> {
        if let Some(value) = self.try_get_value::<T>(name)? {
            Ok(value)
        } else {
            Ok(f().map_err(|e| e.into())?)
        }
    }

    #[inline]
    pub fn get_or_else<'a, T: TryFrom<&'a EntryCache, Error = impl Into<Error>>>(
        &'a self,
        name: &str,
        f: impl FnOnce() -> T,
    ) -> T {
        self.try_get_or_else::<T>(name, || Ok(f()))
            .unwrap_or_else(|e| panic!("get_or_else: {e}"))
    }

    #[inline]
    pub fn try_iter<'a>(&'a self) -> Result<Box<dyn Iterator<Item = &'a EntryCache> + 'a>, Error> {
        let m = self.cache_map()?;
        Ok(Box::new(m.values()))
    }

    #[inline]
    pub fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a EntryCache> + 'a> {
        self.try_iter().unwrap_or_else(|e| panic!("iter: {e}"))
    }
}
