pub type PublicKey = [u8; 64];
pub type Timestamp = u64;
pub struct Signature {
    pk: PublicKey,
}

impl Default for Signature {
    fn default() -> Self {
        Self { pk: [0; 64] }
    }
}
