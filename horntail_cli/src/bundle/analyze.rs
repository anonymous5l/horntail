/// analyze path structure for file or directory
/// 1. flatten for example
///    Base.wz
///    Canvas.wz
///    Sound.wz
///    ...
/// 2. nested but Base.wz singleton for example
///    Data/Base.wz
///    Data/Sound/Sound.wz
///    Data/Sound/Sound_000.wz
///    Data/Sound/Sound.ini
///    ...
/// 3. nested for example
///    Data/Base/Base.wz
///    Data/Base/Base.init
///    Data/Sound/Sound.wz
///    Data/Sound/Sound_000.wz
///    Data/Sound/Sound.ini
///    ...
/// 4. pack file
///    Mob_00000.ms
///    Mob_00001.ms
///    Mob_00002.ms
///    ...
use horntail::reader::wizet::WizetFile;
use std::fs::Metadata;
use std::path::Path;

pub const BASE_NAME: &str = "Base";

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum Structure {
    Nested,
    Flattened,
}

pub fn analyze_wizet_structure<P: AsRef<Path>>(p: P) -> Option<Structure> {
    let path = p.as_ref();
    let metadata = path.metadata().ok()?;

    if is_wizet_file(&metadata, path) {
        if has_ini_file(path, true) {
            return Some(Structure::Nested);
        }
        return Some(Structure::Flattened);
    }

    if metadata.is_dir() {
        if has_ini_file(path, false) {
            return Some(Structure::Nested);
        }
        if path
            .join(BASE_NAME)
            .with_extension(WizetFile::EXTENSION)
            .exists()
        {
            return Some(Structure::Flattened);
        }
    }

    None
}

#[inline]
fn is_wizet_file(metadata: &Metadata, p: &Path) -> bool {
    metadata.is_file()
        && p.extension()
            .map(|x| x.to_ascii_lowercase() == WizetFile::EXTENSION)
            .unwrap_or(false)
}

#[inline]
fn has_ini_file(p: &Path, parent: bool) -> bool {
    let Some(file_stem) = p.file_stem() else {
        return false;
    };
    let parent = if parent {
        let Some(parent) = p.parent() else {
            return false;
        };
        parent
    } else {
        p
    };
    parent.join(file_stem).with_extension("ini").exists()
}
