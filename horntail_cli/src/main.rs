mod app;
mod bundle;
mod error;
mod optimize;
mod row;
mod widget;

use crate::bundle::analyze::{BASE_NAME, Structure, analyze_wizet_structure};
use crate::bundle::pack::PackBundle;
use crate::bundle::{SharedStructure, WizetBundle};
use crate::error::Error;
use crate::optimize::{string_empty, string_pool_get};
use crate::row::{HorntailRow, IndexGroup, IndexKind, ROW_FLAG_EXPANDED, ROW_FLAG_INITIALIZED};
use clap::{Parser, Subcommand, ValueEnum};
use horntail::EntryKind;
use horntail::crypto::{MapleCipher, MapleTable, MapleTableNone, MapleVersion};
use horntail::reader::PackFile;
use horntail::reader::wizet::get_encrypt_version;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, ValueEnum)]
pub enum Vector {
    GMS,
    EMS,
    NIL,
}

#[derive(Parser)]
#[command(name = "horntail")]
#[command(about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// view wz file
    View {
        /// wz folder or file path
        #[arg(short, long, value_name = "FILE | DIRECTORY", value_hint = clap::ValueHint::AnyPath)]
        path: String,
        /// wz file vector used
        #[arg(short, long, value_enum)]
        key: Vector,
        /// optional version of the wz client used
        #[arg(short, long, value_name = "VERSION")]
        version: u16,
        /// only for kmst 777 client
        #[arg(short, long, value_name = "NO_VERSION", action = clap::ArgAction::SetTrue)]
        no_version: bool,
        /// disable image preview
        #[arg(short, long, value_name = "DISABLE_PREVIEW", action = clap::ArgAction::SetTrue)]
        disable_preview: Option<bool>,
    },
    /// probe wz file version
    Probe {
        /// wz file path
        #[arg(short, long, value_name = "FILE", value_hint = clap::ValueHint::AnyPath)]
        path: String,
    },
}

#[inline]
fn try_load_packs(structure: Rc<SharedStructure>, path: &Path) -> Option<Vec<HorntailRow>> {
    let packs = path.join("Packs");
    if !packs.is_dir() {
        return None;
    }
    PackBundle::with_paths(
        walkdir::WalkDir::new(packs)
            .max_depth(1)
            .follow_links(false)
            .follow_root_links(false)
            .into_iter()
            .filter_map(|x| x.ok())
            .filter(|x| {
                let path = x.path();
                path.is_file()
                    && path
                        .extension()
                        .map(|x| x == PackFile::EXTENSION)
                        .unwrap_or(false)
            })
            .map(|x| x.into_path()),
        structure.clone(),
    )?
    .build_cache()
    .ok()
}

fn view_command(
    path: String,
    key: Vector,
    version: u16,
    no_version: bool,
    disable_preview: bool,
) -> Result<(), Error> {
    let cipher = match key {
        Vector::GMS => Rc::new(RefCell::new(MapleTable::new(
            horntail::consts::MAPLE_VECTOR_GLOBAL,
        )))
        .into_boxed(),
        Vector::EMS => Rc::new(RefCell::new(MapleTable::new(
            horntail::consts::MAPLE_VECTOR_EUROPE,
        )))
        .into_boxed(),
        Vector::NIL => MapleTableNone.into_boxed(),
    };

    let root = if let Some(structure) = analyze_wizet_structure(path.as_str()) {
        let shared_structure = Rc::new(SharedStructure {
            ver: MapleVersion::from(version),
            ver_hash: MapleVersion::from(version).hash(),
            cipher,
            no_version,
            structure,
        });

        let root = WizetBundle::with_path(path.as_str(), shared_structure.clone())?;
        let mut leaf = root.build_cache()?;

        let p: &Path = path.as_ref();
        if p.file_stem().map(|x| x == BASE_NAME).unwrap_or(false) {
            // try load packs file
            if let Structure::Nested = structure {
                let dir = if p.is_file() {
                    p.parent().and_then(|p| p.parent()).unwrap()
                } else {
                    p.parent().unwrap()
                };
                if let Some(pack_rows) = try_load_packs(shared_structure.clone(), dir) {
                    pack_rows.into_iter().for_each(|row| {
                        if let Some(target) = leaf.iter_mut().find(|x| x.name() == row.name()) {
                            target.leaf = row.leaf;
                        } else {
                            leaf.push(row);
                        }
                    });
                }
            }
        }

        let size = leaf.len();
        HorntailRow::sort_rows(&mut leaf);
        HorntailRow {
            name: string_pool_get("Root".to_string()),
            offset: 0,
            kind: IndexKind::Element(EntryKind::Folder),
            group: Rc::new(IndexGroup {
                parent_offset: 0,
                file: Rc::new(root.parent.clone()),
                structure: root.structure.clone(),
                builder: None,
            }),
            desc: string_empty(),
            leaf: Some(leaf),
            flag: ROW_FLAG_EXPANDED | ROW_FLAG_INITIALIZED,
            size,
        }
    } else {
        let shared_structure = Rc::new(SharedStructure {
            ver: MapleVersion::from(version),
            ver_hash: MapleVersion::from(version).hash(),
            cipher,
            no_version,
            structure: Structure::Flattened,
        });

        let p: &Path = path.as_ref();
        let bundle = if p.is_file() {
            PackBundle::with_paths(Some(p), shared_structure.clone())
        } else {
            PackBundle::with_paths(
                walkdir::WalkDir::new(path)
                    .max_depth(1)
                    .follow_links(false)
                    .follow_root_links(false)
                    .into_iter()
                    .filter_map(|x| x.ok())
                    .filter(|x| x.path().extension().map(|x| x == "ms").unwrap_or(false))
                    .map(|x| x.into_path()),
                shared_structure.clone(),
            )
        };

        let Some(bundle) = bundle else {
            return Err(Error::InvalidPackPaths);
        };

        let mut rows = bundle.build_cache()?;
        let size = rows.len();

        HorntailRow::sort_rows(&mut rows);
        HorntailRow {
            name: string_pool_get("Root".to_string()),
            offset: 0,
            kind: IndexKind::Element(EntryKind::Folder),
            desc: string_empty(),
            group: Rc::new(IndexGroup {
                parent_offset: 0,
                file: Rc::new(PathBuf::default()),
                structure: shared_structure.clone(),
                builder: None,
            }),
            leaf: Some(rows),
            flag: ROW_FLAG_EXPANDED | ROW_FLAG_INITIALIZED,
            size,
        }
    };

    let result = app::serve(root, disable_preview);
    ratatui::restore();
    result
}

fn probe_command(path: String) -> Result<(), Error> {
    let enc_ver = get_encrypt_version(path)?;
    let possible = (0..u16::MAX)
        .filter(|client_version| MapleVersion::from(*client_version).hash_enc() == enc_ver)
        .collect::<Vec<_>>();
    println!("{possible:?}");
    Ok(())
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.command {
        Commands::View {
            path,
            key,
            version,
            no_version,
            disable_preview,
        } => view_command(
            path,
            key,
            version,
            no_version,
            disable_preview.unwrap_or_default(),
        ),
        Commands::Probe { path } => probe_command(path),
    }
}
