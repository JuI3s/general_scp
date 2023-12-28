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
