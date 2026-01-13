use std::io::Read as _;

use anyhow::Context as _;

pub trait Decode<R>: Sized {
    type Tag: Decode<R>;

    fn decode(bytes: &mut ByteReader<R>, tag: Self::Tag) -> anyhow::Result<Self>;
}

pub trait DecodeTag: Sized {
    fn decode_tag(byte: u8) -> Option<Self>;
}

impl<R: std::io::Read, T: DecodeTag> Decode<R> for T {
    type Tag = ();

    fn decode(bytes: &mut ByteReader<R>, _: Self::Tag) -> anyhow::Result<Self> {
        let byte = bytes.next()?;
        Self::decode_tag(byte).with_context(|| format!("unexpected byte: 0x{byte:0>2X}"))
    }
}

impl<R: std::io::Read> Decode<R> for () {
    type Tag = ();

    fn decode(_bytes: &mut ByteReader<R>, _: Self::Tag) -> anyhow::Result<Self> {
        Ok(())
    }
}

impl<R: std::io::Read, D: Decode<R>> Decode<R> for Vec<D>
where
    D::Tag: Decode<R, Tag = ()>,
{
    type Tag = ();

    fn decode(bytes: &mut ByteReader<R>, _: Self::Tag) -> anyhow::Result<Self> {
        let len: u32 = bytes.decode()?;
        let mut vec = Vec::with_capacity(len as usize);
        for _ in 0..len {
            vec.push(bytes.decode()?);
        }
        Ok(vec)
    }
}

impl<R: std::io::Read> Decode<R> for u32 {
    type Tag = ();

    fn decode(bytes: &mut ByteReader<R>, _: Self::Tag) -> anyhow::Result<Self> {
        let mut n: u32 = 0;
        let mut shift: u32 = 0;
        while shift < 32 {
            let byte = bytes.next()? as u32;
            let value = byte & 0b0111_1111;
            let carry = byte & 0b1000_0000;
            n |= value << shift;
            if carry == 0 {
                return Ok(n);
            }
            shift += 7;
        }
        anyhow::bail!("unsigned LEB128 overflowed");
    }
}

pub struct ByteReader<R> {
    bytes: std::io::Bytes<std::io::BufReader<R>>,
    next_byte: Option<std::io::Result<u8>>,
    offset: usize,
}

impl<R: std::io::Read> From<R> for ByteReader<R> {
    fn from(value: R) -> Self {
        Self::new(value)
    }
}

impl<R: std::io::Read> ByteReader<R> {
    pub fn new(reader: R) -> Self {
        let mut bytes = std::io::BufReader::new(reader).bytes();
        let next_byte = bytes.next();
        Self {
            bytes,
            next_byte,
            offset: 0,
        }
    }

    pub fn next(&mut self) -> anyhow::Result<u8> {
        let res = self.next_byte.take().context("EOF")??;
        self.next_byte = self.bytes.next();
        self.offset += 1;
        Ok(res)
    }

    pub fn is_finished(&self) -> bool {
        self.next_byte.is_none()
    }

    pub fn decode<D: Decode<R>>(&mut self) -> anyhow::Result<D>
    where
        D::Tag: Decode<R, Tag = ()>,
    {
        let tag = self.decode_with_tag(())?;
        self.decode_with_tag(tag)
    }

    pub fn decode_with_tag<D: Decode<R>>(&mut self, tag: D::Tag) -> anyhow::Result<D> {
        let start_offset = self.offset;
        D::decode(self, tag).with_context(|| {
            format!(
                "failed to decode `{}` at offset 0x{:0>8X}..=0x{:0>8X}",
                std::any::type_name::<D>(),
                start_offset,
                self.offset - 1,
            )
        })
    }

    pub fn consume_constant(&mut self, expecteds: &[u8]) -> anyhow::Result<()> {
        let mut actuals = Vec::with_capacity(expecteds.len());
        for _ in 0..expecteds.len() {
            actuals.push(self.next()?);
        }
        anyhow::ensure!(
            expecteds == actuals,
            "expected {expecteds:?}, got {actuals:?}"
        );
        Ok(())
    }
}
