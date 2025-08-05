use crate::bundle::analyze::{BASE_NAME, Structure};
use crate::bundle::{SharedStructure, WizetBundle};
use crate::optimize::{RefString, string_empty, string_pool_get};
use horntail::reader::wizet::WizetFile;
use horntail::reader::{Accessor, seek_back};
use horntail::{
    AccessorBuilder, AccessorOpt, Directories, EntryKind, FromAccessor, FromBuilder, ImageKind,
    PlainPrimitive, PlainProperties, Primitive, Properties, Property, PropertyKind, RawData, UOL,
    Vector2D,
};
use horntail::{Canvas, CanvasAttribute, VideoAttribute};
use horntail::{SoundAttribute, WaveFormat};
use std::io::SeekFrom;
use std::path::PathBuf;
use std::rc::Rc;

pub const ROW_FLAG_INITIALIZED: u64 = 1 << 63;
pub const ROW_FLAG_EXPANDED: u64 = 1 << 62;

#[derive(Copy, Clone)]
pub enum PrimitiveKind {
    Nil,
    Int16,
    Int32,
    Int64,
    Float32,
    Float64,
    String,
}

impl PrimitiveKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            PrimitiveKind::Nil => "Nil",
            PrimitiveKind::Int16 => "Int16",
            PrimitiveKind::Int32 => "Int32",
            PrimitiveKind::Int64 => "Int64",
            PrimitiveKind::Float32 => "Float32",
            PrimitiveKind::Float64 => "Float64",
            PrimitiveKind::String => "String",
        }
    }
}

#[derive(Copy, Clone)]
pub enum IndexKind {
    Primitive(PrimitiveKind),
    Element(EntryKind),
}

impl IndexKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            IndexKind::Primitive(p) => p.as_str(),
            IndexKind::Element(e) => match e {
                EntryKind::Folder => "Folder",
                EntryKind::Image(i) => match i {
                    ImageKind::Canvas => "Canvas",
                    ImageKind::Video => "Video",
                    ImageKind::Convex2D => "Convex2D",
                    ImageKind::Vector2D => "Vector2D",
                    ImageKind::UOL => "UOL",
                    ImageKind::Sound => "Sound",
                    ImageKind::RawData => "RawData",
                    ImageKind::Script => "Script",
                },
                EntryKind::Property(kind) => match kind {
                    PropertyKind::Plain => "Plain",
                    PropertyKind::Encode => "Property",
                },
            },
        }
    }
}

impl From<u64> for IndexKind {
    fn from(value: u64) -> Self {
        let v = (value >> 56) & 0x1f;
        match v {
            1 => IndexKind::Primitive(PrimitiveKind::Nil),
            2 => IndexKind::Primitive(PrimitiveKind::Int16),
            3 => IndexKind::Primitive(PrimitiveKind::Int32),
            4 => IndexKind::Primitive(PrimitiveKind::Int64),
            5 => IndexKind::Primitive(PrimitiveKind::Float32),
            6 => IndexKind::Primitive(PrimitiveKind::Float64),
            7 => IndexKind::Primitive(PrimitiveKind::String),
            8 => IndexKind::Element(EntryKind::Folder),
            9 => IndexKind::Element(EntryKind::Image(ImageKind::Canvas)),
            10 => IndexKind::Element(EntryKind::Image(ImageKind::Video)),
            11 => IndexKind::Element(EntryKind::Image(ImageKind::Convex2D)),
            12 => IndexKind::Element(EntryKind::Image(ImageKind::Vector2D)),
            13 => IndexKind::Element(EntryKind::Image(ImageKind::UOL)),
            14 => IndexKind::Element(EntryKind::Image(ImageKind::Sound)),
            15 => IndexKind::Element(EntryKind::Image(ImageKind::RawData)),
            16 => IndexKind::Element(EntryKind::Image(ImageKind::Script)),
            17 => IndexKind::Element(EntryKind::Property(PropertyKind::Encode)),
            18 => IndexKind::Element(EntryKind::Property(PropertyKind::Plain)),
            _ => unreachable!(),
        }
    }
}

impl From<IndexKind> for u64 {
    fn from(value: IndexKind) -> Self {
        let flag: u64 = match value {
            IndexKind::Primitive(PrimitiveKind::Nil) => 1,
            IndexKind::Primitive(PrimitiveKind::Int16) => 2,
            IndexKind::Primitive(PrimitiveKind::Int32) => 3,
            IndexKind::Primitive(PrimitiveKind::Int64) => 4,
            IndexKind::Primitive(PrimitiveKind::Float32) => 5,
            IndexKind::Primitive(PrimitiveKind::Float64) => 6,
            IndexKind::Primitive(PrimitiveKind::String) => 7,
            IndexKind::Element(EntryKind::Folder) => 8,
            IndexKind::Element(EntryKind::Image(ImageKind::Canvas)) => 9,
            IndexKind::Element(EntryKind::Image(ImageKind::Video)) => 10,
            IndexKind::Element(EntryKind::Image(ImageKind::Convex2D)) => 11,
            IndexKind::Element(EntryKind::Image(ImageKind::Vector2D)) => 12,
            IndexKind::Element(EntryKind::Image(ImageKind::UOL)) => 13,
            IndexKind::Element(EntryKind::Image(ImageKind::Sound)) => 14,
            IndexKind::Element(EntryKind::Image(ImageKind::RawData)) => 15,
            IndexKind::Element(EntryKind::Image(ImageKind::Script)) => 16,
            IndexKind::Element(EntryKind::Property(PropertyKind::Encode)) => 17,
            IndexKind::Element(EntryKind::Property(PropertyKind::Plain)) => 18,
        };
        flag << 56
    }
}

pub struct IndexGroup {
    pub parent_offset: usize,
    pub file: Rc<PathBuf>,
    pub structure: Rc<SharedStructure>,
    pub builder: Option<Box<dyn AccessorBuilder>>,
}

pub struct HorntailRow {
    pub offset: usize,
    pub name: RefString,
    pub desc: RefString,
    pub group: Rc<IndexGroup>,
    pub flag_and_size: u64,
    pub leaf: Option<Vec<HorntailRow>>,
}

impl HorntailRow {
    pub fn get_by_name_paths(&mut self, paths: &str) -> Option<&HorntailRow> {
        let mut cursor = self;
        let name = paths.split('/');
        for next in name {
            cursor.initialize();
            cursor = cursor
                .leaf
                .as_mut()
                .and_then(|leaf| leaf.iter_mut().find(|c| &*c.name == next))?;
        }
        Some(cursor)
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn value(&self) -> &str {
        self.desc.as_ref()
    }

    #[inline]
    pub fn children(&self) -> &[HorntailRow] {
        self.leaf.as_deref().unwrap_or(&[])
    }

    #[inline]
    pub fn is_initialized(&self) -> bool {
        self.flag_and_size & ROW_FLAG_INITIALIZED == ROW_FLAG_INITIALIZED
    }

    #[inline]
    pub fn is_expand(&self) -> bool {
        self.flag_and_size & ROW_FLAG_EXPANDED == ROW_FLAG_EXPANDED
    }

    pub fn sort_rows(rows: &mut [HorntailRow]) {
        let is_all_word = rows
            .iter()
            .any(|r| r.name.chars().any(|x| !x.is_ascii_digit()));
        if is_all_word {
            rows.sort_by(|a, b| a.name.cmp(&b.name))
        } else {
            rows.sort_by(|a, b| {
                a.name
                    .parse::<isize>()
                    .and_then(|a| b.name.parse::<isize>().map(|b| (a, b)))
                    .map(|(a, b)| a.cmp(&b))
                    .unwrap_or_else(|_| a.name.cmp(&b.name))
            })
        }
    }

    #[inline]
    fn merge_rows(a: Vec<HorntailRow>, b: &mut Vec<HorntailRow>) {
        a.into_iter().for_each(|row| {
            if let Some(target) = b.iter_mut().find(|x| x.name() == row.name()) {
                target.flag_and_size &= !ROW_FLAG_INITIALIZED;
                target.name = row.name;
                target.desc = row.desc;
                target.offset = row.offset;
                target.group = row.group;
                if let Some(leaf) = row.leaf {
                    if let Some(t) = target.leaf.as_mut() {
                        t.extend(leaf);
                    } else {
                        target.leaf = Some(leaf)
                    }
                }
            } else {
                b.push(row)
            }
        })
    }

    pub fn initialize(&mut self) {
        let kind = IndexKind::from(self.flag_and_size);
        if !has_leaf(kind) || self.is_initialized() {
            return;
        }

        self.flag_and_size |= ROW_FLAG_INITIALIZED;
        let rows = build_rows(
            &self.name,
            AccessorOpt {
                offset: self.offset,
                ver_hash: self.group.structure.ver_hash,
                parent_offset: self.group.parent_offset,
            },
            kind,
            &self.group,
        );

        if let Some(rows) = rows {
            if let Some(sl) = self.leaf.as_mut() {
                Self::merge_rows(rows, sl);
            } else {
                self.leaf = Some(rows)
            }
        }

        if let Some(leaf) = self.leaf.as_mut() {
            Self::sort_rows(leaf);
        }
    }

    #[inline]
    fn adjust_expand_size(&mut self) {
        self.flag_and_size &= 0xFF00000000000000;
        if self.is_expand() {
            if let Some(leaf) = self.leaf.as_ref() {
                self.flag_and_size |= leaf
                    .iter()
                    .fold(0, |acc, leaf| acc + leaf.expand_size() + 1);
            }
        }
    }

    #[inline]
    fn get_mut_by_paths(&mut self, paths: &[u64]) -> Option<&mut HorntailRow> {
        paths.iter().copied().try_fold(&mut *self, |cursor, index| {
            if let Some(leaf) = cursor.leaf.as_mut() {
                leaf.get_mut(index as usize)
            } else {
                None
            }
        })
    }

    #[inline]
    pub fn get_with_paths(&self, paths: &[u64]) -> (PathBuf, &HorntailRow) {
        let p = PathBuf::default();
        paths.iter().copied().fold((p, self), |(p, cursor), index| {
            if let Some(leaf) = cursor.leaf.as_ref() {
                let leaf = &leaf[index as usize];
                (p.join::<&str>(leaf.name.as_ref()), leaf)
            } else {
                panic!("invalid index")
            }
        })
    }

    #[inline]
    pub fn get_by_paths(&self, paths: &[u64]) -> &HorntailRow {
        paths.iter().copied().fold(self, |cursor, index| {
            if let Some(leaf) = cursor.leaf.as_ref() {
                &leaf[index as usize]
            } else {
                panic!("invalid index")
            }
        })
    }

    #[inline]
    #[allow(dead_code)]
    pub fn toggle(&mut self) {
        if self.is_expand() {
            self.collapse();
        } else {
            self.expand();
        }
    }

    #[inline]
    pub fn toggle_paths(&mut self, paths: &[u64]) {
        let Some(target) = self.get_mut_by_paths(paths) else {
            return;
        };
        if target.is_expand() {
            self.collapse_paths(paths);
        } else {
            self.expand_paths(paths);
        }
    }

    #[inline]
    pub fn toggle_recursive(&mut self, paths: &[u64], depth: Option<usize>) {
        let Some(target) = self.get_mut_by_paths(paths) else {
            return;
        };
        if target.is_expand() {
            self.collapse_recursive(paths, depth);
        } else {
            self.expand_recursive(paths, depth);
        }
    }

    pub fn expand(&mut self) {
        if !has_leaf(IndexKind::from(self.flag_and_size)) {
            return;
        }

        self.initialize();
        self.flag_and_size |= ROW_FLAG_EXPANDED;
        self.adjust_expand_size();
    }

    pub fn expand_paths(&mut self, paths: &[u64]) {
        fn _expand(cache: &mut HorntailRow, paths: &[u64]) {
            if !cache.is_expand() {
                cache.expand();
            }
            if let Some((first, last)) = paths.split_first() {
                if let Some(leaf) = cache.leaf.as_mut() {
                    _expand(&mut leaf[*first as usize], last);
                }
                cache.adjust_expand_size();
            }
        }
        _expand(self, paths);
    }

    pub fn expand_recursive(&mut self, paths: &[u64], depth: Option<usize>) {
        fn _expand_recursive_internal(cache: &mut HorntailRow, depth: Option<usize>) {
            if let Some(0) = depth {
                return;
            }
            if !cache.is_expand() {
                cache.expand();
            }
            if let Some(leaf) = cache.leaf.as_mut() {
                leaf.iter_mut().for_each(|leaf| {
                    _expand_recursive_internal(leaf, depth.map(|x| x.saturating_sub(1)));
                });
            }
            cache.adjust_expand_size();
        }
        fn _expand_recursive(cache: &mut HorntailRow, paths: &[u64], depth: Option<usize>) {
            let Some((first, last)) = paths.split_first() else {
                _expand_recursive_internal(cache, depth);
                return;
            };

            if !cache.is_expand() {
                cache.expand();
            }

            if let Some(leaf) = cache.leaf.as_mut() {
                _expand_recursive(&mut leaf[*first as usize], last, depth);
            }
            cache.adjust_expand_size();
        }
        _expand_recursive(self, paths, depth);
    }

    pub fn collapse(&mut self) {
        self.flag_and_size &= !ROW_FLAG_EXPANDED;
        self.adjust_expand_size();
    }

    pub fn collapse_paths(&mut self, paths: &[u64]) {
        fn _collapse(cache: &mut HorntailRow, paths: &[u64]) {
            let Some((first, last)) = paths.split_first() else {
                cache.collapse();
                return;
            };

            if let Some(leaf) = cache.leaf.as_mut() {
                _collapse(&mut leaf[*first as usize], last);
            }
            cache.adjust_expand_size();
        }
        _collapse(self, paths);
    }

    pub fn collapse_recursive(&mut self, paths: &[u64], depth: Option<usize>) {
        fn _collapse_recursive_internal(cache: &mut HorntailRow, depth: Option<usize>) {
            if let Some(0) = depth {
                return;
            }
            if let Some(leaf) = cache.leaf.as_mut() {
                leaf.iter_mut().for_each(|leaf| {
                    _collapse_recursive_internal(leaf, depth.map(|x| x.saturating_sub(1)));
                });
            }
            if cache.is_expand() {
                cache.collapse();
            }
        }
        fn _collapse_recursive(cache: &mut HorntailRow, paths: &[u64], depth: Option<usize>) {
            let Some((first, last)) = paths.split_first() else {
                _collapse_recursive_internal(cache, depth);
                return;
            };
            if let Some(leaf) = cache.leaf.as_mut() {
                _collapse_recursive(&mut leaf[*first as usize], last, depth);
            }
            cache.adjust_expand_size();
        }
        _collapse_recursive(self, paths, depth);
    }

    #[inline]
    pub fn expand_size(&self) -> u64 {
        if self.is_expand() {
            self.flag_and_size & !0xFF00000000000000
        } else {
            0
        }
    }

    pub fn to_canvas(&self) -> Option<Canvas> {
        let IndexKind::Element(EntryKind::Image(ImageKind::Canvas)) =
            IndexKind::from(self.flag_and_size)
        else {
            return None;
        };

        let builder = self.group.builder.as_ref().map(|x| x.as_ref())?;

        Some(Canvas::from_builder(
            AccessorOpt {
                offset: self.offset,
                ver_hash: self.group.structure.ver_hash,
                parent_offset: self.group.parent_offset,
            },
            builder,
        ))
    }
}

fn get_image_value(opt: AccessorOpt, kind: ImageKind, accessor: &mut dyn Accessor) -> RefString {
    match kind {
        ImageKind::Canvas => {
            let attr = CanvasAttribute::from_accessor(opt, accessor);
            string_pool_get(format!(
                "{} (W:{}, H:{})",
                attr.format, attr.size.x, attr.size.y
            ))
        }
        ImageKind::Video => {
            let attr = VideoAttribute::from_accessor(opt, accessor);
            string_pool_get(format!(
                "{:#x} (W:{}, H:{}, S:{}",
                attr.fourcc, attr.width, attr.height, attr.size
            ))
        }
        ImageKind::Vector2D => {
            let vec2d = Vector2D::from_accessor(opt, accessor);
            string_pool_get(format!("(X:{}, Y:{})", vec2d.x, vec2d.y))
        }
        ImageKind::UOL => {
            let uol = UOL::from_accessor(opt, accessor);
            string_pool_get(uol.path)
        }
        ImageKind::Sound => {
            let attr = SoundAttribute::from_accessor(opt, accessor);
            if let Some(format) = attr.media_type.pb_format.as_ref() {
                match format {
                    WaveFormat::MP3(_) => string_pool_get("MP3".to_owned()),
                    WaveFormat::PCM(_) => string_pool_get("WAV".to_owned()),
                }
            } else {
                string_pool_get("Data".to_owned())
            }
        }
        ImageKind::RawData => {
            let raw_data = RawData::from_accessor(opt, accessor);
            string_pool_get(format!("RawData({})", raw_data.data.len()))
        }
        _ => string_empty(),
    }
}

fn get_element_kind_value(
    opt: AccessorOpt,
    ik: EntryKind,
    accessor: &mut dyn Accessor,
) -> RefString {
    match ik {
        EntryKind::Folder | EntryKind::Property(_) => string_empty(),
        EntryKind::Image(img) => get_image_value(opt, img, accessor),
    }
}

#[inline]
fn process_property(
    opt: AccessorOpt,
    group: &Rc<IndexGroup>,
    props: Vec<Property>,
    accessor: &mut dyn Accessor,
) -> Option<Vec<HorntailRow>> {
    accessor.seek(SeekFrom::Start(0));
    let rows = props
        .into_iter()
        .map(|p| {
            let mut offset = p.offset;
            let (kind, value) = match &p.value {
                Primitive::Image(img) => {
                    offset = img.offset;
                    seek_back(accessor, SeekFrom::Start(offset as u64), |accessor| {
                        (
                            IndexKind::Element(img.kind),
                            get_element_kind_value(opt, img.kind, accessor),
                        )
                    })
                }
                Primitive::Nil => (IndexKind::Primitive(PrimitiveKind::Nil), string_empty()),
                Primitive::Int16(i) => (
                    IndexKind::Primitive(PrimitiveKind::Int16),
                    string_pool_get(i.to_string()),
                ),
                Primitive::Int32(i) => (
                    IndexKind::Primitive(PrimitiveKind::Int32),
                    string_pool_get(i.to_string()),
                ),
                Primitive::Int64(i) => (
                    IndexKind::Primitive(PrimitiveKind::Int64),
                    string_pool_get(i.to_string()),
                ),
                Primitive::Float32(f) => (
                    IndexKind::Primitive(PrimitiveKind::Float32),
                    string_pool_get(f.to_string()),
                ),
                Primitive::Float64(f) => (
                    IndexKind::Primitive(PrimitiveKind::Float64),
                    string_pool_get(f.to_string()),
                ),
                Primitive::String(i) => (
                    IndexKind::Primitive(PrimitiveKind::String),
                    string_pool_get(i.to_owned()),
                ),
            };

            HorntailRow {
                name: string_pool_get(p.name),
                offset,
                group: group.clone(),
                desc: string_pool_get(value.to_string()),
                leaf: None,
                flag_and_size: u64::from(kind),
            }
        })
        .collect::<Vec<_>>();
    if rows.is_empty() { None } else { Some(rows) }
}

#[inline]
fn process_folder(
    name: &str,
    opt: AccessorOpt,
    group: &Rc<IndexGroup>,
    accessor: &mut dyn Accessor,
) -> Option<Vec<HorntailRow>> {
    let mut rows = Directories::from_accessor(opt, accessor)
        .into_inner()
        .into_iter()
        .map(|f| HorntailRow {
            name: string_pool_get(f.name),
            offset: f.offset,
            group: Rc::new(IndexGroup {
                parent_offset: f.parent_offset,
                file: group.file.clone(),
                structure: group.structure.clone(),
                builder: group.builder.as_ref().map(|x| x.clone_boxed()),
            }),
            desc: string_empty(),
            leaf: None,
            flag_and_size: u64::from(IndexKind::Element(f.kind)),
        })
        .collect::<Vec<_>>();

    let parent = if group.file.file_stem().unwrap() == BASE_NAME {
        if let Structure::Flattened = group.structure.structure {
            group.file.parent().unwrap()
        } else {
            group.file.parent().and_then(|p| p.parent()).unwrap()
        }
    } else {
        group.file.parent().unwrap()
    };

    let mut try_path = parent.join(name);
    if let Structure::Flattened = group.structure.structure {
        try_path = try_path.with_extension(WizetFile::EXTENSION);
    }
    if try_path != group.file.as_path() {
        if let Ok(bundle) = WizetBundle::with_path(try_path, group.structure.clone()) {
            rows.extend(bundle.build_cache().unwrap());
        }
    }

    if rows.is_empty() { None } else { Some(rows) }
}

fn build_plain_properties_rows(
    opt: &AccessorOpt,
    group: &Rc<IndexGroup>,
    plain_properties: PlainProperties,
) -> Option<Vec<HorntailRow>> {
    let rows = plain_properties
        .into_inner()
        .into_iter()
        .map(|row| {
            let mut leaf = None;
            let mut value = string_empty();
            let mut kind = IndexKind::Element(EntryKind::Property(PropertyKind::Plain));
            match row.value {
                PlainPrimitive::Value(str) => {
                    kind = IndexKind::Primitive(PrimitiveKind::String);
                    value = string_pool_get(str);
                }
                PlainPrimitive::Nested(nested) => {
                    leaf = build_plain_properties_rows(opt, group, nested)
                }
            }

            HorntailRow {
                name: string_pool_get(row.name.to_string()),
                desc: value,
                offset: opt.offset,
                group: group.clone(),
                leaf,
                flag_and_size: ROW_FLAG_INITIALIZED | u64::from(kind),
            }
        })
        .collect::<Vec<_>>();
    if rows.is_empty() { None } else { Some(rows) }
}
#[inline]
pub fn has_leaf(kind: IndexKind) -> bool {
    !matches!(
        kind,
        IndexKind::Element(EntryKind::Image(
            ImageKind::Vector2D | ImageKind::UOL | ImageKind::Convex2D,
        )) | IndexKind::Primitive(_)
    )
}

pub fn build_rows(
    name: &str,
    opt: AccessorOpt,
    kind: IndexKind,
    group: &Rc<IndexGroup>,
) -> Option<Vec<HorntailRow>> {
    let IndexKind::Element(e) = kind else {
        return None;
    };
    let mut accessor = opt.accessor(group.builder.as_ref().map(|x| x.as_ref()).unwrap());
    match e {
        EntryKind::Folder => {
            return process_folder(name, opt, group, accessor.as_mut());
        }
        EntryKind::Image(img) => match img {
            ImageKind::Canvas | ImageKind::Video => {
                if let Some(props) = Properties::builtin(opt, accessor.as_mut()).unwrap() {
                    return process_property(opt, group, props.into_inner(), accessor.as_mut());
                }
            }
            ImageKind::Sound | ImageKind::RawData => {
                if let Some(props) = Properties::optional(opt, accessor.as_mut()).unwrap() {
                    return process_property(opt, group, props.into_inner(), accessor.as_mut());
                }
            }
            _ => {}
        },
        EntryKind::Property(kind) => {
            return match kind {
                PropertyKind::Plain => {
                    let plain_prop = PlainProperties::from_accessor(opt, accessor.as_mut());
                    build_plain_properties_rows(&opt, group, plain_prop)
                }
                PropertyKind::Encode => {
                    let prop = Properties::from_accessor(opt, accessor.as_mut());
                    process_property(opt, group, prop.into_inner(), accessor.as_mut())
                }
            };
        }
    };
    None
}
