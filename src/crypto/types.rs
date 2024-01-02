use std::marker::PhantomData;

use blake2::{Blake2b512, Blake2s256, Digest};
use serde::Serialize;

pub type Blake2Hash = [u8; 64];

pub trait Blake2Hashable
where
    Self: Serialize,
{
    fn to_blake2(&self) -> Blake2Hash {
        let mut hasher = Blake2b512::new();
        let encoded: Vec<u8> = bincode::serialize(&self).unwrap();
        hasher.update(encoded);
        hasher.finalize().into()
    }
}

pub struct Blake2Hasher<N>
where
    N: Serialize,
{
    phantom: PhantomData<N>,
}

impl<N> Blake2Hasher<N>
where
    N: Serialize,
{
    pub fn hash(value: &N) -> Blake2Hash {
        let mut hasher = Blake2b512::new();
        let encoded: Vec<u8> = bincode::serialize(&value).unwrap();
        hasher.update(encoded);
        hasher.finalize().into()
    }
}

pub fn test_default_blake2() -> Blake2Hash {
    [0; 64]
}

mod serde_bytes_array {
    use core::convert::TryInto;

    use serde::de::Error;
    use serde::{Deserializer, Serializer};

    /// This just specializes [`serde_bytes::serialize`] to `<T = [u8]>`.
    pub(crate) fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serde_bytes::serialize(bytes, serializer)
    }

    /// This takes the result of [`serde_bytes::deserialize`] from `[u8]` to
    /// `[u8; N]`.
    pub(crate) fn deserialize<'de, D, const N: usize>(deserializer: D) -> Result<[u8; N], D::Error>
    where
        D: Deserializer<'de>,
    {
        let slice: &[u8] = serde_bytes::deserialize(deserializer)?;
        let array: [u8; N] = slice.try_into().map_err(|_| {
            let expected = format!("[u8; {}]", N);
            D::Error::invalid_length(slice.len(), &expected.as_str())
        })?;
        Ok(array)
    }
}

#[cfg(test)]
mod tests {
    use blake2::{Blake2b512, Blake2s256, Digest};
    use hex_literal::hex;

    use crate::crypto::types::Blake2Hash;

    #[test]
    fn blake2() {
        let mut hasher = Blake2b512::new();

        // write input message
        hasher.update(b"hello world");

        // read hash digest and consume hasher
        let res: Blake2Hash = hasher.finalize().into();
        assert_eq!(
            res[..],
            hex!(
                "
    021ced8799296ceca557832ab941a50b4a11f83478cf141f51f933f653ab9fbc
    c05a037cddbed06e309bf334942c4e58cdf1a46e237911ccd7fcf9787cbc7fd0
"
            )[..]
        );
    }
}
