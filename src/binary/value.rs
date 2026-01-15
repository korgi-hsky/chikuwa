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
            let unused_bits_mask = !((1 << remaining_bit_count) - 1);
            anyhow::ensure!(0 == last_byte & unused_bits_mask, "overflowed `u{N}`");
        }
        result |= I::from(last_byte) << shift;
        Ok(Self(result))
    }
}

#[cfg(test)]
mod uleb128_tests {
    use super::super::decode::ByteReader;
    use super::*;

    #[test]
    fn decode_trailing_zeroes() -> anyhow::Result<()> {
        let mut bytes = ByteReader::from([0b0000_0011]);
        assert_eq!(3, bytes.decode::<UnsignedInt<8, u8>>()?.0);
        let mut bytes = ByteReader::from([0b1000_0011, 0]);
        assert_eq!(3, bytes.decode::<UnsignedInt<8, u8>>()?.0);

        let mut bytes = ByteReader::from([0b1000_0011, 0b0001_0000]);
        assert!(bytes.decode::<UnsignedInt<8, u8>>().is_err());
        Ok(())
    }

    #[test]
    fn decode_overflow() -> anyhow::Result<()> {
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

pub struct SignedInt<const N: u8, I>(pub I);

pub enum SignedIntByte {
    Next(u8),
    LastPositive(u8),
    LastNegative(u8),
}

impl SignedIntByte {
    const BIT_COUNT: u8 = 7;
    const LAST_BIT_COUNT: u8 = 6;
}

impl From<u8> for SignedIntByte {
    fn from(byte: u8) -> Self {
        if 0 < byte & 0b1000_0000 {
            Self::Next(byte & 0b0111_1111)
        } else if 0 < byte & 0b0100_0000 {
            Self::LastNegative(byte)
        } else {
            Self::LastPositive(byte)
        }
    }
}

impl super::decode::DecodeTag for SignedIntByte {
    fn decode_tag(byte: u8) -> Option<Self> {
        Some(byte.into())
    }
}

impl<R: std::io::Read, const N: u8, I> super::decode::Decode<R> for SignedInt<N, I>
where
    I: From<i8> //
        + std::ops::BitOr<Output = I>
        + std::ops::BitOrAssign
        + std::ops::Not<Output = I>
        + std::ops::Shl<u8, Output = I>,
{
    type Tag = SignedIntByte;

    fn decode(bytes: &mut super::decode::ByteReader<R>, tag: Self::Tag) -> anyhow::Result<Self> {
        assert!(N as usize <= std::mem::size_of::<I>() * 8);
        let mut result = I::from(0);
        let mut shift = 0;
        let mut handle_byte = |byte| {
            Ok(match byte {
                SignedIntByte::LastPositive(byte) => Some((byte, true)),
                SignedIntByte::LastNegative(byte) => Some((byte, false)),
                SignedIntByte::Next(byte) => {
                    result |= I::from(byte.cast_signed()) << shift;
                    shift += SignedIntByte::BIT_COUNT;
                    anyhow::ensure!(shift < N, "too many bytes encoding `s{N}`");
                    None
                }
            })
        };
        let (last_byte, is_positive) = match handle_byte(tag)? {
            Some(value) => value,
            None => loop {
                if let Some(value) = handle_byte(bytes.next()?.into())? {
                    break value;
                }
            },
        };
        let remaining_bit_count = N - shift;
        if remaining_bit_count < SignedIntByte::LAST_BIT_COUNT {
            let unused_bit_count = SignedIntByte::LAST_BIT_COUNT - remaining_bit_count;
            let unused_bits_mask = ((1 << unused_bit_count) - 1) << remaining_bit_count;
            let expected_unused_bits = if is_positive { 0 } else { unused_bits_mask };
            anyhow::ensure!(
                expected_unused_bits == last_byte & unused_bits_mask,
                "overflowed `s{N}`"
            );
        }
        let sign_extended = if is_positive {
            I::from(0)
        } else {
            !I::from(0) << SignedIntByte::LAST_BIT_COUNT
        };
        result |= (sign_extended | I::from(last_byte.cast_signed())) << shift;
        Ok(Self(result))
    }
}

#[cfg(test)]
mod sleb128_tests {
    use super::super::decode::ByteReader;
    use super::*;

    #[test]
    fn decode_trailing_zeroes() -> anyhow::Result<()> {
        let mut bytes = ByteReader::from([0b0111_1110]);
        assert_eq!(-2, bytes.decode::<SignedInt<16, i16>>()?.0);
        let mut bytes = ByteReader::from([0b1111_1110, 0b0111_1111]);
        assert_eq!(-2, bytes.decode::<SignedInt<16, i16>>()?.0);
        let mut bytes = ByteReader::from([0b1111_1110, 0b1111_1111, 0b0111_1111]);
        assert_eq!(-2, bytes.decode::<SignedInt<16, i16>>()?.0);

        let mut bytes = ByteReader::from([0b1000_0011, 0b0011_1110]);
        assert!(bytes.decode::<SignedInt<8, i8>>().is_err());
        let mut bytes = ByteReader::from([0b1111_1111, 0b0111_1011]);
        assert!(bytes.decode::<SignedInt<8, i8>>().is_err());
        Ok(())
    }

    #[test]
    fn decode_overflow() -> anyhow::Result<()> {
        let mut bytes = ByteReader::from([0b1000_0011, 0]);
        assert_eq!(3, bytes.decode::<SignedInt<8, i8>>()?.0);
        let mut bytes = ByteReader::from([0b1000_0011, 0]);
        assert!(bytes.decode::<SignedInt<7, i8>>().is_err());

        let mut bytes = ByteReader::from([0b0101_1100]);
        assert_eq!(-36, bytes.decode::<SignedInt<6, i8>>()?.0);
        let mut bytes = ByteReader::from([0b0101_1100]);
        assert!(bytes.decode::<SignedInt<5, i8>>().is_err());
        Ok(())
    }
}
