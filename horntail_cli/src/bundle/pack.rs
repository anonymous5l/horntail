use crate::bundle::bundle::SharedStructure;
use crate::error::Error;
use crate::optimize::{string_empty, string_pool_get};
use crate::row::{HorntailRow, IndexGroup, IndexKind, ROW_FLAG_INITIALIZED};
use horntail::crypto::MapleTableNone;
use horntail::reader::{BinaryAccessor, BinaryBuilder, PackFile};
use horntail::{AccessorBuilder, AccessorOpt, EntryKind, FromBuilder, Image};
use std::path::{Component, Components, Path, PathBuf};
use std::rc::Rc;

pub struct PackBundle {
    pub structure: Rc<SharedStructure>,
    pub files: Vec<PathBuf>,
}

impl PackBundle {
    pub fn with_paths<I, P>(path: I, ss: Rc<SharedStructure>) -> Option<PackBundle>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = P>,
    {
        let files = path
            .into_iter()
            .map(|x| x.as_ref().to_path_buf())
            .collect::<Vec<_>>();
        if files.is_empty() {
            return None;
        }
        Some(PackBundle {
            structure: ss,
            files,
        })
    }

    fn insert_virtual_row(
        &self,
        from: Rc<PathBuf>,
        root: &mut Vec<HorntailRow>,
        full_path: &mut Components,
        row: HorntailRow,
    ) {
        if let Some(Component::Normal(dir)) = full_path.next() {
            if let Some(f) = root.iter_mut().find(|x| x.name() == dir) {
                if let Some(leaf) = f.leaf.as_mut() {
                    return self.insert_virtual_row(from, leaf, full_path, row);
                }
            }
            let mut leaf = vec![];
            self.insert_virtual_row(from.clone(), &mut leaf, full_path, row);
            root.push(HorntailRow {
                name: string_pool_get(dir.to_str().map(|s| s.to_owned()).unwrap()),
                desc: string_empty(),
                offset: 0,
                group: Rc::new(IndexGroup {
                    parent_offset: 0,
                    file: from.clone(),
                    structure: self.structure.clone(),
                    builder: None,
                }),
                leaf: Some(leaf),
                flag_and_size: ROW_FLAG_INITIALIZED
                    | u64::from(IndexKind::Element(EntryKind::Folder)),
            });
        } else {
            root.push(row);
        }
    }

    pub fn build_cache(&self) -> Result<Vec<HorntailRow>, Error> {
        let mut caches = Vec::new();
        for f in self.files.iter() {
            let path = Rc::new(f.clone());
            let packfile = PackFile::new(path.as_path())?;
            let indexes = packfile.entries()?;

            let mut accessor = BinaryAccessor::new(MapleTableNone, packfile.source().open()?);
            for index in indexes.iter() {
                let data = index.decrypt_from(&mut accessor)?;
                let builder = BinaryBuilder::new(MapleTableNone, data);
                let image = Image::from_builder(
                    AccessorOpt {
                        offset: 0,
                        ver_hash: self.structure.ver_hash,
                        parent_offset: 0,
                    },
                    &builder,
                );
                let fullpath = PathBuf::from(&index.name);
                let row = HorntailRow {
                    name: string_pool_get(
                        fullpath
                            .file_name()
                            .and_then(|x| x.to_str())
                            .unwrap_or("")
                            .to_string(),
                    ),
                    offset: image.offset,
                    group: Rc::new(IndexGroup {
                        parent_offset: 0,
                        file: path.clone(),
                        structure: self.structure.clone(),
                        builder: Some(builder.clone_boxed()),
                    }),
                    desc: string_empty(),
                    leaf: Some(vec![]),
                    flag_and_size: u64::from(IndexKind::Element(image.kind)),
                };

                if let Some(mut components) = fullpath.parent().map(|p| p.components()) {
                    self.insert_virtual_row(path.clone(), &mut caches, &mut components, row);
                } else {
                    caches.push(row);
                }
            }
        }
        Ok(caches)
    }
}
