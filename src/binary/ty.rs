use anyhow::Context;

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
    pub composite: Composite,
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
                composite: bytes.decode()?,
            },
            SubTag::NonFinal => Self {
                is_final: false,
                supers: bytes.decode()?,
                composite: bytes.decode()?,
            },
            SubTag::Composite(tag) => Self {
                is_final: true,
                supers: vec![],
                composite: bytes.decode_with_tag(tag)?,
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
    pub is_mutable: bool,
}

impl<R: std::io::Read> super::decode::Decode<R> for Field {
    type Tag = ();

    fn decode(bytes: &mut super::decode::ByteReader<R>, _: Self::Tag) -> anyhow::Result<Self> {
        Ok(Self {
            storage: bytes.decode()?,
            is_mutable: matches!(bytes.decode()?, Mutability::Mutable),
        })
    }
}

enum Mutability {
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
pub enum Storage {
    Value(Value),
    Pack(Pack),
}

pub enum StorageTag {
    Value(ValueTag),
    Pack(Pack),
}

impl super::decode::DecodeTag for StorageTag {
    fn decode_tag(byte: u8) -> Option<Self> {
        None.or_else(|| ValueTag::decode_tag(byte).map(Self::Value))
            .or_else(|| Pack::decode_tag(byte).map(Self::Pack))
    }
}

impl<R: std::io::Read> super::decode::Decode<R> for Storage {
    type Tag = StorageTag;

    fn decode(bytes: &mut super::decode::ByteReader<R>, tag: Self::Tag) -> anyhow::Result<Self> {
        Ok(match tag {
            StorageTag::Value(tag) => Storage::Value(bytes.decode_with_tag(tag)?),
            StorageTag::Pack(pack) => Storage::Pack(pack),
        })
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
pub enum Value {
    Num(Number),
    Vec(Vector),
    Ref(Reference),
}

pub enum ValueTag {
    Num(Number),
    Vec(Vector),
    Ref(ReferenceTag),
}

impl super::decode::DecodeTag for ValueTag {
    fn decode_tag(byte: u8) -> Option<Self> {
        None.or_else(|| Number::decode_tag(byte).map(Self::Num))
            .or_else(|| Vector::decode_tag(byte).map(Self::Vec))
            .or_else(|| ReferenceTag::decode_tag(byte).map(Self::Ref))
    }
}

impl<R: std::io::Read> super::decode::Decode<R> for Value {
    type Tag = ValueTag;

    fn decode(bytes: &mut super::decode::ByteReader<R>, tag: Self::Tag) -> anyhow::Result<Self> {
        Ok(match tag {
            ValueTag::Num(num) => Value::Num(num),
            ValueTag::Vec(vec) => Value::Vec(vec),
            ValueTag::Ref(tag) => Value::Ref(bytes.decode_with_tag(tag)?),
        })
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

#[derive(Debug, PartialEq, Eq)]
pub struct Reference {
    pub heap: Heap,
    pub is_nullable: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReferenceTag {
    Nullable,
    NonNullable,
    AbsHeap(AbsHeap),
}

impl super::decode::DecodeTag for ReferenceTag {
    fn decode_tag(byte: u8) -> Option<Self> {
        Some(match byte {
            0x63 => Self::Nullable,
            0x64 => Self::NonNullable,
            _ => Self::AbsHeap(AbsHeap::decode_tag(byte)?),
        })
    }
}

impl<R: std::io::Read> super::decode::Decode<R> for Reference {
    type Tag = ReferenceTag;

    fn decode(bytes: &mut super::decode::ByteReader<R>, tag: Self::Tag) -> anyhow::Result<Self> {
        Ok(match tag {
            ReferenceTag::Nullable => Self {
                heap: bytes.decode()?,
                is_nullable: true,
            },
            ReferenceTag::NonNullable => Self {
                heap: bytes.decode()?,
                is_nullable: false,
            },
            ReferenceTag::AbsHeap(abs) => Self {
                heap: Heap::Abstract(abs),
                is_nullable: true,
            },
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Heap {
    Abstract(AbsHeap),
    Concrete(u32), // type index encoded as `s33`
}

pub enum HeapTag {
    Abstract(AbsHeap),
    Concrete(super::value::SignedIntByte),
}

impl super::decode::DecodeTag for HeapTag {
    fn decode_tag(byte: u8) -> Option<Self> {
        if let Some(abs) = AbsHeap::decode_tag(byte) {
            return Some(Self::Abstract(abs));
        }
        let byte = super::value::SignedIntByte::decode_tag(byte)?;
        if !matches!(byte, crate::binary::value::SignedIntByte::LastNegative(_)) {
            Some(Self::Concrete(byte))
        } else {
            None
        }
    }
}

impl<R: std::io::Read> super::decode::Decode<R> for Heap {
    type Tag = HeapTag;

    fn decode(bytes: &mut super::decode::ByteReader<R>, tag: Self::Tag) -> anyhow::Result<Self> {
        Ok(match tag {
            HeapTag::Abstract(abs) => Self::Abstract(abs),
            HeapTag::Concrete(byte) => {
                let int = bytes.decode_with_tag::<super::value::SignedInt<33, i64>>(byte)?.0;
                Self::Concrete(int.try_into().context("invalid type index")?)
            }
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AbsHeap {
    Exception,
    Array,
    Struct,
    I31,
    Eq,
    Any,
    Extern,
    Func,
    None,
    NoExtern,
    NoFunc,
    NoException,
}

impl super::decode::DecodeTag for AbsHeap {
    fn decode_tag(byte: u8) -> Option<Self> {
        Some(match byte {
            0x69 => Self::Exception,
            0x6a => Self::Array,
            0x6b => Self::Struct,
            0x6c => Self::I31,
            0x6d => Self::Eq,
            0x6e => Self::Any,
            0x6f => Self::Extern,
            0x70 => Self::Func,
            0x71 => Self::NoExtern,
            0x72 => Self::NoFunc,
            0x73 => Self::NoException,
            _ => return None,
        })
    }
}
