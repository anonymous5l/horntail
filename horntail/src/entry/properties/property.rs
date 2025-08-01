use crate::Image;
use crate::entry::properties::lexer::{Lexer, TokenKind};
use crate::reader::Accessor;
use crate::{AccessorOpt, Error, TryFromAccessor};
use std::fmt::{Display, Formatter};
use std::io::SeekFrom;
use std::ops::Deref;

mod kind {
    pub const NIL: u8 = 0x00;
    pub const I16_1: u8 = 0x02;
    pub const I16_2: u8 = 0x0B;
    pub const I32_1: u8 = 0x03;
    pub const I32_2: u8 = 0x13;
    pub const I64: u8 = 0x14;
    pub const F32: u8 = 0x04;
    pub const F64: u8 = 0x05;
    pub const STRING: u8 = 0x08;
    pub const IMAGE: u8 = 0x09;
}

#[derive(Clone)]
pub enum Primitive {
    Nil,
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    String(String),
    Image(Image),
}

impl Display for Primitive {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Primitive::Nil => write!(f, "Nil"),
            Primitive::Int16(v) => write!(f, "Int16({v})"),
            Primitive::Int32(v) => write!(f, "Int32({v})"),
            Primitive::Int64(v) => write!(f, "Int64({v})"),
            Primitive::Float32(v) => write!(f, "Float32({v})"),
            Primitive::Float64(v) => write!(f, "Float64({v})"),
            Primitive::String(v) => write!(f, "String({v})"),
            Primitive::Image(v) => write!(f, "Image({})", v.kind),
        }
    }
}

#[derive(Clone)]
pub struct Property {
    pub name: String,
    pub offset: usize,
    pub value: Primitive,
}

impl TryFromAccessor for Property {
    type Error = Error;

    fn try_from_accessor(
        opt: AccessorOpt,
        accessor: &mut dyn Accessor,
    ) -> Result<Self, Self::Error> {
        let offset = accessor.pos();
        let name = accessor.get_uol_string(opt.parent_offset);
        let kind = accessor.get_u8();
        let value = match kind {
            kind::IMAGE => {
                let image_size = accessor.get_i32_le() as usize;
                let current = accessor.pos();
                let mut image = Image::try_from_accessor(opt.clone_with(current), accessor)?;
                image.size = image_size;
                accessor.seek(SeekFrom::Start((image_size + current) as u64));
                Primitive::Image(image)
            }
            kind::NIL => Primitive::Nil,
            kind::I16_1 | kind::I16_2 => Primitive::Int16(accessor.get_i16_le()),
            kind::I32_1 | kind::I32_2 => Primitive::Int32(accessor.get_var_i32_le()),
            kind::I64 => Primitive::Int64(accessor.get_var_i64_le()),
            kind::F32 => Primitive::Float32(accessor.get_var_f32_le()),
            kind::F64 => Primitive::Float64(accessor.get_f64_le()),
            kind::STRING => Primitive::String(accessor.get_uol_string(opt.parent_offset)),
            _ => {
                panic!("unexpected property kind {kind}");
            }
        };
        Ok(Property {
            name,
            offset,
            value,
        })
    }
}

#[derive(Clone)]
pub struct Properties {
    properties: Vec<Property>,
}

impl Deref for Properties {
    type Target = [Property];

    fn deref(&self) -> &Self::Target {
        &self.properties
    }
}

impl Properties {
    pub fn builtin(
        opt: AccessorOpt,
        accessor: &mut dyn Accessor,
    ) -> Result<Option<Properties>, Error> {
        if (accessor.get_u16_le() >> 8) != 1 {
            return Ok(None);
        }
        Ok(Some(Properties::try_from_accessor(
            opt.clone_with(accessor.pos()),
            accessor,
        )?))
    }

    pub fn optional(
        opt: AccessorOpt,
        accessor: &mut dyn Accessor,
    ) -> Result<Option<Properties>, Error> {
        if accessor.get_u8() == 1 && accessor.get_u8() == 1 {
            Ok(Some(Properties::try_from_accessor(
                opt.clone_with(accessor.pos()),
                accessor,
            )?))
        } else {
            Ok(None)
        }
    }

    #[inline]
    fn get_properties_count(accessor: &mut dyn Accessor) -> usize {
        accessor.advance(2);
        accessor.get_var_i32_le() as usize
    }

    pub fn into_inner(self) -> Vec<Property> {
        self.properties
    }
}

impl TryFromAccessor for Properties {
    type Error = Error;

    fn try_from_accessor(
        opt: AccessorOpt,
        accessor: &mut dyn Accessor,
    ) -> Result<Self, Self::Error> {
        let count = Self::get_properties_count(accessor);
        let mut properties = Vec::with_capacity(count);
        for _ in 0..count {
            properties.push(Property::try_from_accessor(
                opt.clone_with(accessor.pos()),
                accessor,
            )?)
        }
        Ok(Properties { properties })
    }
}

#[derive(Clone)]
pub enum PlainPrimitive {
    Value(String),
    Nested(PlainProperties),
}

#[derive(Clone)]
pub struct PlainProperty {
    pub name: String,
    pub value: PlainPrimitive,
}

#[derive(Clone)]
pub struct PlainProperties {
    properties: Vec<PlainProperty>,
}

impl Deref for PlainProperties {
    type Target = [PlainProperty];

    fn deref(&self) -> &Self::Target {
        &self.properties
    }
}

impl PlainProperties {
    #[inline]
    pub fn into_inner(self) -> Vec<PlainProperty> {
        self.properties
    }
}

impl TryFromAccessor for PlainProperties {
    type Error = Error;

    fn try_from_accessor(_: AccessorOpt, accessor: &mut dyn Accessor) -> Result<Self, Self::Error> {
        let buffer = accessor.copy_to_vec(accessor.remaining());
        let buffer = unsafe { String::from_utf8_unchecked(buffer) };
        let mut lexer = Lexer::new(&buffer);
        let mut properties = Vec::new();
        while let Some(p) = parse_value(&mut lexer) {
            properties.push(p);
        }
        Ok(Self { properties })
    }
}

fn parse_equal<'a>(lexer: &mut Lexer<'a>) -> Option<&'a str> {
    loop {
        let token = lexer.next()?;
        if let TokenKind::Equal = token.kind {
            return Some(token.origin);
        } else if let TokenKind::EndOfLine = token.kind {
            continue;
        } else {
            return None;
        }
    }
}

fn parse_value(lexer: &mut Lexer) -> Option<PlainProperty> {
    let key = parse_equal(lexer)?;
    let next = lexer.next()?;
    match next.kind {
        TokenKind::LeftBrace => {
            let mut properties = Vec::new();
            while let Some(value) = parse_value(lexer) {
                properties.push(value);
            }
            Some(PlainProperty {
                name: key.to_owned(),
                value: PlainPrimitive::Nested(PlainProperties { properties }),
            })
        }
        TokenKind::EndOfLine => Some(PlainProperty {
            name: key.to_owned(),
            value: PlainPrimitive::Value(next.origin.to_owned()),
        }),
        _ => None,
    }
}
