impl<R: std::io::Read, D: super::decode::Decode<R>> super::decode::Decode<R> for Vec<D>
where
    D::Tag: super::decode::Decode<R, Tag = ()>,
{
    type Tag = ();

    fn decode(bytes: &mut super::decode::ByteReader<R>, _: Self::Tag) -> anyhow::Result<Self> {
        let len: u32 = bytes.decode()?;
        let mut vec = Vec::with_capacity(len as usize);
        for _ in 0..len {
            vec.push(bytes.decode()?);
        }
        Ok(vec)
    }
}

impl<R: std::io::Read> super::decode::Decode<R> for u32 {
    type Tag = ();

    fn decode(bytes: &mut super::decode::ByteReader<R>, _: Self::Tag) -> anyhow::Result<Self> {
        bytes.decode::<UnsignedInt<32, u32>>().map(|i| i.0)
    }
}

pub struct UnsignedInt<const N: u8, I>(pub I);

pub enum UnsignedIntByte {
    Next(u8),
    Last(u8),
}

impl UnsignedIntByte {
    const BIT_COUNT: u8 = 7;
}

impl From<u8> for UnsignedIntByte {
    fn from(byte: u8) -> Self {
        if 0 < byte & 0b1000_0000 {
            Self::Next(byte & 0b0111_1111)
        } else {
            Self::Last(byte)
        }
    }
}

impl<R: std::io::Read, const N: u8, I> super::decode::Decode<R> for UnsignedInt<N, I>
where
    I: From<u8> //
        + std::ops::BitOrAssign
        + std::ops::Shl<u8, Output = I>,
{
    type Tag = ();

    fn decode(bytes: &mut super::decode::ByteReader<R>, _: Self::Tag) -> anyhow::Result<Self> {
        assert!(N as usize <= std::mem::size_of::<I>() * 8);
        let mut result = I::from(0);
        let mut shift = 0;
        let last_byte = loop {
            match UnsignedIntByte::from(bytes.next()?) {
                UnsignedIntByte::Last(byte) => break byte,
                UnsignedIntByte::Next(byte) => {
                    result |= I::from(byte) << shift;
                    shift += UnsignedIntByte::BIT_COUNT;
                }
            }
            anyhow::ensure!(shift < N, "too many bytes encoding `u{N}`");
        };
        let remaining_bit_count = N - shift;
        if remaining_bit_count < UnsignedIntByte::BIT_COUNT {
            let unusable_bit_mask = !((1 << remaining_bit_count) - 1);
            // unusable bits must be 0
            anyhow::ensure!(0 == last_byte & unusable_bit_mask, "overflowed `u{N}`");
        }
        result |= I::from(last_byte) << shift;
        Ok(Self(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::decode::ByteReader;

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
