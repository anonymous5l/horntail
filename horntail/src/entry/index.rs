use std::fmt::{Display, Formatter};

pub(crate) const FOLDER: &str = "Folder";
pub(crate) const PROPERTY: &str = "Property";
pub(crate) const CANVAS: &str = "Canvas";
pub(crate) const CANVAS_VIDEO: &str = "Canvas#Video";
pub(crate) const SHAPE2D_CONVEX2D: &str = "Shape2D#Convex2D";
pub(crate) const SHAPE2D_VECTOR2D: &str = "Shape2D#Vector2D";
pub(crate) const UOL: &str = "UOL";
pub(crate) const SOUND_DX8: &str = "Sound_DX8";
pub(crate) const RAW_DATA: &str = "RawData";
pub(crate) const SCRIPT: &str = "Script";

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum ImageKind {
    Canvas,
    Video,
    Convex2D,
    Vector2D,
    UOL,
    Sound,
    RawData,
    Script,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum PropertyKind {
    Plain,
    Encode,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum EntryKind {
    Folder,
    Image(ImageKind),
    Property(PropertyKind),
}

impl EntryKind {
    pub fn as_str(&self) -> &str {
        match self {
            EntryKind::Folder => FOLDER,
            EntryKind::Property(_) => PROPERTY,
            EntryKind::Image(k) => match k {
                ImageKind::Canvas => CANVAS,
                ImageKind::Video => CANVAS_VIDEO,
                ImageKind::Convex2D => SHAPE2D_CONVEX2D,
                ImageKind::Vector2D => SHAPE2D_VECTOR2D,
                ImageKind::UOL => UOL,
                ImageKind::Sound => SOUND_DX8,
                ImageKind::RawData => RAW_DATA,
                ImageKind::Script => SCRIPT,
            },
        }
    }
}

impl Display for EntryKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
