use crate::PropertyKind;
use crate::entry::index;
use crate::reader::Accessor;
use crate::{AccessorOpt, EntryKind, Error, ImageKind, TryFromAccessor};

#[derive(Clone)]
pub struct Image {
    pub kind: EntryKind,
    pub offset: usize,
    pub size: usize,
}

impl TryFromAccessor for Image {
    type Error = Error;

    fn try_from_accessor(
        opt: AccessorOpt,
        accessor: &mut dyn Accessor,
    ) -> Result<Self, Self::Error> {
        let kind = accessor.try_get_image_string(opt.parent_offset);

        match kind {
            Ok(kind) => Ok(Image {
                kind: match kind.as_str() {
                    index::PROPERTY => EntryKind::Property(PropertyKind::Encode),
                    index::CANVAS => EntryKind::Image(ImageKind::Canvas),
                    index::CANVAS_VIDEO => EntryKind::Image(ImageKind::Video),
                    index::SHAPE2D_CONVEX2D => EntryKind::Image(ImageKind::Convex2D),
                    index::SHAPE2D_VECTOR2D => EntryKind::Image(ImageKind::Vector2D),
                    index::UOL => EntryKind::Image(ImageKind::UOL),
                    index::SOUND_DX8 => EntryKind::Image(ImageKind::Sound),
                    index::RAW_DATA => EntryKind::Image(ImageKind::RawData),
                    _ => {
                        return Err(Error::UnexpectedData(format!(
                            "unsupported image kind `{kind}`",
                        )));
                    }
                },
                size: 0,
                offset: accessor.pos(),
            }),
            Err(flag) => {
                if flag == 0x23 && accessor.get_utf8_string(8) == "Property" {
                    Ok(Image {
                        kind: EntryKind::Property(PropertyKind::Plain),
                        size: 0,
                        offset: accessor.pos(),
                    })
                } else if flag == 0x01 {
                    // lua script
                    Ok(Image {
                        kind: EntryKind::Image(ImageKind::Script),
                        size: 0,
                        offset: accessor.pos() - 1,
                    })
                } else {
                    Err(Error::UnexpectedData(format!(
                        "unsupported image string flag `{flag}` {}",
                        accessor.pos(),
                    )))
                }
            }
        }
    }
}
