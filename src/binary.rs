mod decode;
pub mod instr;
pub mod ty;
pub mod value;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Module {
    pub type_section: Option<TypeSection>,
    pub func_section: Option<FuncSection>,
    pub code_section: Option<CodeSection>,
}

impl Module {
    pub fn decode(reader: impl std::io::Read) -> anyhow::Result<Self> {
        let mut bytes = decode::ByteReader::new(reader);
        bytes.decode()
    }

    pub fn decode_bytes(bytes: Vec<u8>) -> anyhow::Result<Self> {
        Self::decode(std::io::Cursor::new(bytes))
    }
}

impl<R: std::io::Read> decode::Decode<R> for Module {
    type Tag = ();

    fn decode(bytes: &mut decode::ByteReader<R>, _: Self::Tag) -> anyhow::Result<Self> {
        let mut module = Self::default();
        bytes.consume_constant("\0asm".as_bytes())?;
        bytes.consume_constant(&1u32.to_le_bytes())?;
        while !bytes.is_finished() {
            let section_id: SectionId = bytes.decode()?;
            let _byte_count: u32 = bytes.decode()?;
            match section_id {
                SectionId::Type => module.type_section = Some(bytes.decode()?),
                SectionId::Func => module.func_section = Some(bytes.decode()?),
                SectionId::Code => module.code_section = Some(bytes.decode()?),
                _ => anyhow::bail!("unimplemented section ID: {section_id:?}"),
            }
        }
        Ok(module)
    }
}

#[derive(Debug)]
pub enum SectionId {
    Custom,
    Type,
    Import,
    Func,
    Table,
    Memory,
    Global,
    Export,
    Start,
    Element,
    Code,
    Data,
    DataCount,
    Tag,
}

impl decode::DecodeTag for SectionId {
    fn decode_tag(byte: u8) -> Option<Self> {
        use SectionId::*;
        Some(match byte {
            0 => Custom,
            1 => Type,
            2 => Import,
            3 => Func,
            4 => Table,
            5 => Memory,
            6 => Global,
            7 => Export,
            8 => Start,
            9 => Element,
            10 => Code,
            11 => Data,
            12 => DataCount,
            13 => Tag,
            _ => return None,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TypeSection(pub Vec<ty::Recursive>);

impl<R: std::io::Read> decode::Decode<R> for TypeSection {
    type Tag = ();

    fn decode(bytes: &mut decode::ByteReader<R>, _: Self::Tag) -> anyhow::Result<Self> {
        bytes.decode().map(Self)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct FuncSection(pub Vec<u32>);

impl<R: std::io::Read> decode::Decode<R> for FuncSection {
    type Tag = ();

    fn decode(bytes: &mut decode::ByteReader<R>, _: Self::Tag) -> anyhow::Result<Self> {
        bytes.decode().map(Self)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct CodeSection(pub Vec<Func>);

impl<R: std::io::Read> decode::Decode<R> for CodeSection {
    type Tag = ();

    fn decode(bytes: &mut decode::ByteReader<R>, _: Self::Tag) -> anyhow::Result<Self> {
        bytes.decode().map(Self)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Func {
    pub locals: Vec<Local>,
    pub expr: instr::Expression,
}

impl<R: std::io::Read> decode::Decode<R> for Func {
    type Tag = ();

    fn decode(bytes: &mut decode::ByteReader<R>, _: Self::Tag) -> anyhow::Result<Self> {
        let _byte_count: u32 = bytes.decode()?;
        Ok(Self {
            locals: bytes.decode()?,
            expr: bytes.decode()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Local {
    pub num: u32,
    pub ty: ty::Value,
}

impl<R: std::io::Read> decode::Decode<R> for Local {
    type Tag = ();

    fn decode(bytes: &mut decode::ByteReader<R>, _: Self::Tag) -> anyhow::Result<Self> {
        Ok(Self {
            num: bytes.decode()?,
            ty: bytes.decode()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_minimal_module() -> anyhow::Result<()> {
        let wasm = wat::parse_str("(module)")?;
        assert_eq!(Module::default(), Module::decode_bytes(wasm)?);
        Ok(())
    }

    #[test]
    fn decode_empty_function() -> anyhow::Result<()> {
        let wasm = wat::parse_str("(module (func))")?;
        assert_eq!(
            Module {
                type_section: Some(TypeSection(vec![ty::Recursive(vec![ty::Sub {
                    is_final: true,
                    supers: vec![],
                    ty: ty::Composite::Func {
                        params: vec![],
                        returns: vec![],
                    },
                }])])),
                func_section: Some(FuncSection(vec![0])),
                code_section: Some(CodeSection(vec![Func {
                    locals: vec![],
                    expr: instr::Expression(vec![instr::Instruction::End]),
                }])),
            },
            Module::decode_bytes(wasm)?,
        );
        Ok(())
    }

    #[test]
    fn decode_function_with_params() -> anyhow::Result<()> {
        let wasm = wat::parse_str("(module (func (param i32 i64)))")?;
        assert_eq!(
            Module {
                type_section: Some(TypeSection(vec![ty::Recursive(vec![ty::Sub {
                    is_final: true,
                    supers: vec![],
                    ty: ty::Composite::Func {
                        params: vec![
                            ty::Value::Num(ty::Number::I32),
                            ty::Value::Num(ty::Number::I64),
                        ],
                        returns: vec![],
                    },
                }])])),
                func_section: Some(FuncSection(vec![0])),
                code_section: Some(CodeSection(vec![Func {
                    locals: vec![],
                    expr: instr::Expression(vec![instr::Instruction::End]),
                }])),
            },
            Module::decode_bytes(wasm)?,
        );
        Ok(())
    }

    #[test]
    fn decode_function_with_locals() -> anyhow::Result<()> {
        let wasm = wat::parse_str("(module (func (local i32) (local i64 i64)))")?;
        assert_eq!(
            Module {
                type_section: Some(TypeSection(vec![ty::Recursive(vec![ty::Sub {
                    is_final: true,
                    supers: vec![],
                    ty: ty::Composite::Func {
                        params: vec![],
                        returns: vec![],
                    },
                }])])),
                func_section: Some(FuncSection(vec![0])),
                code_section: Some(CodeSection(vec![Func {
                    locals: vec![
                        Local {
                            num: 1,
                            ty: ty::Value::Num(ty::Number::I32),
                        },
                        Local {
                            num: 2,
                            ty: ty::Value::Num(ty::Number::I64),
                        },
                    ],
                    expr: instr::Expression(vec![instr::Instruction::End]),
                }])),
            },
            Module::decode_bytes(wasm)?,
        );
        Ok(())
    }

    #[test]
    fn decode_function_i32_add() -> anyhow::Result<()> {
        let wasm = wat::parse_str(
            "
(module
    (func (param i32 i32) (result i32)
        (local.get 0)
        (local.get 1)
        i32.add))",
        )?;
        assert_eq!(
            Module {
                type_section: Some(TypeSection(vec![ty::Recursive(vec![ty::Sub {
                    is_final: true,
                    supers: vec![],
                    ty: ty::Composite::Func {
                        params: vec![
                            ty::Value::Num(ty::Number::I32),
                            ty::Value::Num(ty::Number::I32),
                        ],
                        returns: vec![ty::Value::Num(ty::Number::I32)],
                    },
                }])])),
                func_section: Some(FuncSection(vec![0])),
                code_section: Some(CodeSection(vec![Func {
                    locals: vec![],
                    expr: instr::Expression(vec![
                        instr::Instruction::LocalGet(0),
                        instr::Instruction::LocalGet(1),
                        instr::Instruction::I32Add,
                        instr::Instruction::End,
                    ]),
                }])),
            },
            Module::decode_bytes(wasm)?,
        );
        Ok(())
    }
}
