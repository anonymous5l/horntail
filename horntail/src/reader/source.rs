use crate::Error;
use memmap2::Mmap;
use std::fs::File;
use std::path::{Path, PathBuf};

pub struct Source {
    path: PathBuf,
}

impl Source {
    pub fn new<P: AsRef<Path>>(p: P) -> Self {
        Source {
            path: p.as_ref().to_path_buf(),
        }
    }

    #[inline]
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    #[inline]
    pub fn open(&self) -> Result<Mmap, Error> {
        Ok(unsafe { Mmap::map(&File::open(self.path.as_path())?)? })
    }
}
