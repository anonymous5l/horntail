use crate::bundle::analyze::{BASE_NAME, Structure};
use crate::error::Error;
use crate::row::{HorntailRow, IndexGroup, IndexKind, build_rows};
use horntail::crypto::{MapleCipher, MapleVersion};
use horntail::reader::BinaryBuilder;
use horntail::reader::wizet::WizetFile;
use horntail::{AccessorBuilder, AccessorOpt, EntryKind};
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub struct SharedStructure {
    pub ver: MapleVersion,
    pub ver_hash: u16,
    pub cipher: Box<dyn MapleCipher>,
    pub no_version: bool,
    pub structure: Structure,
}

pub struct WizetBundle {
    pub parent: PathBuf,
    pub structure: Rc<SharedStructure>,
    pub files: Vec<WizetFile>,
}

impl WizetBundle {
    pub fn with_path<P: AsRef<Path>>(
        path: P,
        ss: Rc<SharedStructure>,
    ) -> Result<WizetBundle, Error> {
        let path = path.as_ref();
        let stem = path.file_stem().unwrap();
        let parent_path = if path.is_file() {
            path.parent().unwrap()
        } else {
            path
        };

        let (parent_path, files) = match ss.structure {
            Structure::Nested => (
                if stem == BASE_NAME {
                    parent_path.parent().unwrap()
                } else {
                    parent_path
                },
                Some(load_nested_files(stem, parent_path, ss.ver, ss.no_version)?),
            ),
            Structure::Flattened => {
                if let (true, Ok(e)) = (
                    parent_path.is_dir(),
                    load_nested_files(stem, parent_path, ss.ver, ss.no_version),
                ) {
                    (parent_path, Some(e))
                } else {
                    (
                        parent_path,
                        Some(vec![load_single_file(
                            parent_path.join(stem).with_extension(WizetFile::EXTENSION),
                            ss.ver,
                            ss.no_version,
                        )?]),
                    )
                }
            }
        };

        Ok(WizetBundle {
            parent: PathBuf::from(parent_path),
            structure: ss,
            files: files.ok_or(Error::InvalidStructure)?,
        })
    }

    pub fn build_cache(&self) -> Result<Vec<HorntailRow>, Error> {
        let mut caches = Vec::new();
        for f in self.files.iter() {
            let source = f.source();
            let file_path = source.path();
            let file_name = file_path
                .file_name()
                .and_then(|x| x.to_str())
                .unwrap()
                .to_owned();

            let group = Rc::new(IndexGroup {
                parent_offset: f.data_pos(),
                file: Rc::new(file_path.to_path_buf()),
                structure: self.structure.clone(),
                builder: Some(
                    BinaryBuilder::from_boxed(self.structure.cipher.clone_boxed(), source.open()?)
                        .into_boxed(),
                ),
            });

            let rows = build_rows(
                &file_name,
                AccessorOpt {
                    offset: f.offset(),
                    ver_hash: self.structure.ver_hash,
                    parent_offset: f.data_pos(),
                },
                IndexKind::Element(EntryKind::Folder),
                &group,
            );

            if let Some(rows) = rows {
                caches.extend(rows);
            }
        }
        Ok(caches)
    }
}

fn load_nested_files(
    stem: &OsStr,
    parent: &Path,
    ver: MapleVersion,
    no_version: bool,
) -> Result<Vec<WizetFile>, Error> {
    let stem_ini = parent.join(stem).with_extension("ini");
    let f_ini = File::open(stem_ini)?;
    let mut reader = BufReader::new(f_ini);
    let mut first_line = String::with_capacity(1024);
    reader.read_line(&mut first_line)?;
    let Some((prefix, suffix)) = first_line.split_once('|') else {
        return Err(Error::InvalidStructure);
    };
    if prefix != "LastWzIndex" {
        return Err(Error::InvalidStructure);
    }
    let count = suffix
        .trim()
        .parse::<i16>()
        .map_err(|_| Error::InvalidStructure)?;
    let mut root = Vec::with_capacity((count + 1) as usize);

    let base = load_single_file(
        parent.join(stem).with_extension(WizetFile::EXTENSION),
        ver,
        no_version,
    )?;

    root.push(base);
    for index in 0..=count {
        root.push(load_single_file(
            parent
                .join(format!("{}_{index:03}", stem.to_string_lossy()))
                .with_extension(WizetFile::EXTENSION),
            ver,
            no_version,
        )?)
    }
    Ok(root)
}

pub fn load_single_file<P: AsRef<Path>>(
    path: P,
    ver: MapleVersion,
    no_version: bool,
) -> Result<WizetFile, Error> {
    Ok(WizetFile::new(path, ver, no_version)?)
}
