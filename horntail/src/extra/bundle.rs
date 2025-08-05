use crate::crypto::{MapleCipher, MapleVersion};
use crate::reader::BinaryBuilder;
use crate::reader::wizet::WizetFile;
use crate::{AccessorBuilder, Error};
use crate::{AccessorOpt, error};
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::{Path, PathBuf};

const INI_EXTENSION: &str = "ini";

pub struct Bundle {
    dir: PathBuf,
    version: MapleVersion,
    no_version: bool,
    files: Vec<WizetFile>,
}

pub type OptAndBuilder = (AccessorOpt, Box<dyn AccessorBuilder>);

impl Bundle {
    pub fn from_path<P: AsRef<Path>>(
        path: P,
        version: MapleVersion,
        no_version: bool,
    ) -> Result<Option<Bundle>, Error> {
        let path = path.as_ref();
        let file_name = path
            .file_name()
            .and_then(|f| f.to_str())
            .ok_or(error::io_err_invalid_input())?;
        let (path, parent_path) = if !path.is_file() {
            let file = path.join(file_name).with_extension(WizetFile::EXTENSION);
            if !file.exists() {
                return Ok(None);
            }
            (file, path)
        } else {
            (
                path.to_path_buf(),
                path.parent().ok_or(error::io_err_invalid_input())?,
            )
        };

        let (files, parent_path) =
            if let Some(files) = load_complex_struct(path.as_path(), version, no_version) {
                let parent_path = if parent_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .ok_or(error::io_err_invalid_input())?
                    == "Base"
                {
                    parent_path.parent().ok_or(error::io_err_invalid_input())?
                } else {
                    parent_path
                };
                (files, parent_path.to_path_buf())
            } else if let Some(file) = load_single_file(path.as_path(), version, no_version) {
                (vec![file], parent_path.to_path_buf())
            } else {
                return Ok(None);
            };

        Ok(Some(Bundle {
            dir: parent_path,
            version,
            no_version,
            files,
        }))
    }

    #[inline]
    pub fn load_by_name(&self, name: &str) -> Option<Bundle> {
        Bundle::from_path(self.dir.join(name), self.version, self.no_version)
            .unwrap_or_else(|e| panic!("load_by_name: {e}"))
    }

    pub fn builders(&self, cipher: &dyn MapleCipher) -> Result<Vec<OptAndBuilder>, Error> {
        let mut builders = Vec::with_capacity(self.files.len());
        for f in &self.files {
            let mmap = f.source().open()?;
            let builder = BinaryBuilder::from_boxed(cipher.clone_boxed(), mmap).into_boxed();
            builders.push((f.accessor_opt(), builder));
        }
        Ok(builders)
    }
}

fn load_single_file(path: &Path, version: MapleVersion, no_version: bool) -> Option<WizetFile> {
    WizetFile::new(path, version, no_version).ok()
}

fn load_complex_struct(
    path: &Path,
    version: MapleVersion,
    no_version: bool,
) -> Option<Vec<WizetFile>> {
    let parent_path = path.parent()?;
    let file_stem = path.file_stem().and_then(|s| s.to_str())?;
    let complex_struct = parent_path.join(file_stem).with_extension(INI_EXTENSION);
    let complex_file = File::open(complex_struct).ok()?;
    let mut complex_reader = io::BufReader::new(complex_file);
    let mut first_line = String::with_capacity(128);
    complex_reader.read_line(&mut first_line).ok()?;
    let (prefix, suffix) = first_line.trim().split_once('|')?;
    if prefix != "LastWzIndex" {
        return None;
    }
    let count = suffix.parse::<i16>().ok()?;
    let mut files = Vec::with_capacity((count + 1) as usize);
    files.push(load_single_file(path, version, no_version)?);
    for i in 0..=count {
        let path = parent_path
            .join(format!("{file_stem}_{i:0>3}"))
            .with_extension(WizetFile::EXTENSION);
        let file = load_single_file(path.as_path(), version, no_version)?;
        files.push(file);
    }
    Some(files)
}
