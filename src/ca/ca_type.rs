// pub type PublicKey = [u8; 64];
// pub type PublicKey = String;
#[derive(Clone, PartialEq, Hash)]
pub struct PublicKey {
    key: String,
}

impl PublicKey {
    pub fn to_text(&self) -> String {
        self.key.to_owned()
    }
}

pub type Timestamp = u64;

#[derive(Clone, Hash)]
pub struct Signature {
    pk: PublicKey,
}

pub fn mock_public_key() -> PublicKey {
    PublicKey { key: "".into() }
}

impl Default for Signature {
    fn default() -> Self {
        Self {
            pk: mock_public_key(),
        }
    }
}
