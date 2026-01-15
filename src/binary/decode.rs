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
        bytes.decode::<UnsignedInt<32, u32>>().map(|i| i.0)
    }
}

pub struct UnsignedInt<const N: u8, I>(pub I);

impl<R: std::io::Read, const N: u8, I> Decode<R> for UnsignedInt<N, I>
where
    I: From<u8> //
        + std::ops::BitOrAssign
        + std::ops::Shl<u8, Output = I>,
{
    type Tag = ();

    fn decode(bytes: &mut ByteReader<R>, _: Self::Tag) -> anyhow::Result<Self> {
        assert!(N as usize <= std::mem::size_of::<I>() * 8);
        let mut result = I::from(0);
        let mut shift = 0;
        loop {
            let byte = bytes.next()?;
            if 0 < byte & 0b1000_0000 {
                result |= I::from(byte & 0b0111_1111) << shift;
                shift += 7;
                anyhow::ensure!(shift < N, "too many bytes encoding `u{N}`");
                continue;
            }
            let remaining_bit_count = N - shift;
            if remaining_bit_count < 7 {
                let unusable_bit_mask = !((1 << remaining_bit_count) - 1);
                // unusable bits must be 0
                anyhow::ensure!(0 == byte & unusable_bit_mask, "overflowed `u{N}`");
            }
            result |= I::from(byte) << shift;
            break;
        }
        Ok(Self(result))
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

impl<T: AsRef<[u8]>> From<T> for ByteReader<std::io::Cursor<T>> {
    fn from(value: T) -> Self {
        std::io::Cursor::new(value).into()
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
                "failed to decode `{}` at byte offset 0x{:0>8X}..=0x{:0>8X}",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_uleb128_trailing_zeroes() -> anyhow::Result<()> {
        let mut bytes = ByteReader::from([0b0000_0011]);
        assert_eq!(3, bytes.decode::<UnsignedInt<8, u8>>()?.0);
        let mut bytes = ByteReader::from([0b1000_0011, 0]);
        assert_eq!(3, bytes.decode::<UnsignedInt<8, u8>>()?.0);

        let mut bytes = ByteReader::from([0b1000_0011, 0b0001_0000]);
        assert!(bytes.decode::<UnsignedInt<8, u8>>().is_err());
        Ok(())
    }

    #[test]
    fn decode_uleb128_overflow() -> anyhow::Result<()> {
        let mut bytes = ByteReader::from([0b1000_0011, 0]);
        assert_eq!(3, bytes.decode::<UnsignedInt<8, u8>>()?.0);
        let mut bytes = ByteReader::from([0b1000_0011, 0]);
        assert!(bytes.decode::<UnsignedInt<7, u8>>().is_err());

        let mut bytes = ByteReader::from([0b0100_0000]);
        assert_eq!(64, bytes.decode::<UnsignedInt<7, u8>>()?.0);
        let mut bytes = ByteReader::from([0b0100_0000]);
        assert!(bytes.decode::<UnsignedInt<6, u8>>().is_err());
        Ok(())
    }
}
