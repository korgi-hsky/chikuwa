#[derive(Debug, PartialEq, Eq)]
pub struct Expression(pub Vec<Instruction>);

impl<R: std::io::Read> super::decode::Decode<R> for Expression {
    type Tag = ();
    fn decode(bytes: &mut super::decode::ByteReader<R>, _: Self::Tag) -> anyhow::Result<Self> {
        let mut instructions = Vec::new();
        loop {
            let instr = bytes.decode()?;
            let is_end = matches!(instr, Instruction::End);
            instructions.push(instr);
            if is_end {
                break;
            }
        }
        Ok(Self(instructions))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Instruction {
    // parametric
    Nop,

    // variable
    LocalGet(u32),

    // numeric
    I32Add,

    End,
}

impl<R: std::io::Read> super::decode::Decode<R> for Instruction {
    type Tag = ();
    fn decode(bytes: &mut super::decode::ByteReader<R>, _: Self::Tag) -> anyhow::Result<Self> {
        use Instruction::*;
        Ok(match bytes.next()? {
            0x01 => Nop,

            0x20 => LocalGet(bytes.decode()?),

            0x6a => I32Add,

            0x0b => End,

            byte => anyhow::bail!("unimplemented instruction: '0x{byte:0>2x}'"),
        })
    }
}
