pub type PublicKey = [u8; 64];
pub type Timestamp = u64;

#[derive(Clone)]
pub struct Signature {
    pk: PublicKey,
}

pub fn mock_public_key() -> PublicKey {
    [0; 64]
}

impl Default for Signature {
    fn default() -> Self {
        Self { pk: [0; 64] }
    }
}
