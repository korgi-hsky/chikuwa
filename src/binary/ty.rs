#[derive(Debug, PartialEq, Eq)]
pub struct Recursive(pub Vec<Sub>);

pub enum RecursiveTag {
    Recursive,
    Sub(SubTag),
}

impl super::decode::DecodeTag for RecursiveTag {
    fn decode_tag(byte: u8) -> Option<Self> {
        Some(match byte {
            0x4e => Self::Recursive,
            _ => Self::Sub(SubTag::decode_tag(byte)?),
        })
    }
}

impl<R: std::io::Read> super::decode::Decode<R> for Recursive {
    type Tag = RecursiveTag;

    fn decode(bytes: &mut super::decode::ByteReader<R>, tag: Self::Tag) -> anyhow::Result<Self> {
        Ok(match tag {
            RecursiveTag::Recursive => Self(bytes.decode()?),
            RecursiveTag::Sub(tag) => Self(vec![bytes.decode_with_tag(tag)?]),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Sub {
    pub is_final: bool,
    pub supers: Vec<u32>,
    pub ty: Composite,
}

pub enum SubTag {
    Final,
    NonFinal,
    Composite(CompositeTag),
}

impl super::decode::DecodeTag for SubTag {
    fn decode_tag(byte: u8) -> Option<Self> {
        Some(match byte {
            0x4f => Self::Final,
            0x50 => Self::NonFinal,
            _ => Self::Composite(CompositeTag::decode_tag(byte)?),
        })
    }
}

impl<R: std::io::Read> super::decode::Decode<R> for Sub {
    type Tag = SubTag;

    fn decode(bytes: &mut super::decode::ByteReader<R>, tag: Self::Tag) -> anyhow::Result<Self> {
        Ok(match tag {
            SubTag::Final => Self {
                is_final: true,
                supers: bytes.decode()?,
                ty: bytes.decode()?,
            },
            SubTag::NonFinal => Self {
                is_final: false,
                supers: bytes.decode()?,
                ty: bytes.decode()?,
            },
            SubTag::Composite(tag) => Self {
                is_final: true,
                supers: vec![],
                ty: bytes.decode_with_tag(tag)?,
            },
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Composite {
    Array(Field),
    Struct(Vec<Field>),
    Func {
        params: Vec<Value>,
        returns: Vec<Value>,
    },
}

pub enum CompositeTag {
    Array,
    Struct,
    Func,
}

impl super::decode::DecodeTag for CompositeTag {
    fn decode_tag(byte: u8) -> Option<Self> {
        match byte {
            0x5e => Some(Self::Array),
            0x5f => Some(Self::Struct),
            0x60 => Some(Self::Func),
            _ => None,
        }
    }
}

impl<R: std::io::Read> super::decode::Decode<R> for Composite {
    type Tag = CompositeTag;

    fn decode(bytes: &mut super::decode::ByteReader<R>, tag: Self::Tag) -> anyhow::Result<Self> {
        Ok(match tag {
            CompositeTag::Array => Self::Array(bytes.decode()?),
            CompositeTag::Struct => Self::Struct(bytes.decode()?),
            CompositeTag::Func => Self::Func {
                params: bytes.decode()?,
                returns: bytes.decode()?,
            },
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Field {
    pub storage: Storage,
    pub mutability: Mutability,
}

impl<R: std::io::Read> super::decode::Decode<R> for Field {
    type Tag = ();

    fn decode(bytes: &mut super::decode::ByteReader<R>, _: Self::Tag) -> anyhow::Result<Self> {
        Ok(Self {
            storage: bytes.decode()?,
            mutability: bytes.decode()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Storage {
    Value(Value),
    Pack(Pack),
}

impl super::decode::DecodeTag for Storage {
    fn decode_tag(byte: u8) -> Option<Self> {
        Value::decode_tag(byte)
            .map(Self::Value)
            .or_else(|| Pack::decode_tag(byte).map(Self::Pack))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Pack {
    I16,
    I8,
}

impl super::decode::DecodeTag for Pack {
    fn decode_tag(byte: u8) -> Option<Self> {
        match byte {
            0x77 => Some(Self::I16),
            0x78 => Some(Self::I8),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Mutability {
    Immutable,
    Mutable,
}

impl super::decode::DecodeTag for Mutability {
    fn decode_tag(byte: u8) -> Option<Self> {
        match byte {
            0x00 => Some(Self::Immutable),
            0x01 => Some(Self::Mutable),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    Num(Number),
    Vec(Vector),
}

impl super::decode::DecodeTag for Value {
    fn decode_tag(byte: u8) -> Option<Self> {
        Number::decode_tag(byte)
            .map(Self::Num)
            .or_else(|| Vector::decode_tag(byte).map(Self::Vec))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Number {
    F64,
    F32,
    I64,
    I32,
}

impl super::decode::DecodeTag for Number {
    fn decode_tag(byte: u8) -> Option<Self> {
        match byte {
            0x7c => Some(Self::F64),
            0x7d => Some(Self::F32),
            0x7e => Some(Self::I64),
            0x7f => Some(Self::I32),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Vector {
    Vec,
}

impl super::decode::DecodeTag for Vector {
    fn decode_tag(byte: u8) -> Option<Self> {
        match byte {
            0x7b => Some(Self::Vec),
            _ => None,
        }
    }
}
