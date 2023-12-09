use super::{
    ca_type::{PublicKey, Signature},
    merkle::MerkleTree,
};

pub type RootOpResult<T> = std::result::Result<T, RootOpError>;
pub enum RootOpError {
    RootEntryNotExists,
    Unknown,
}

// Each linked group of delegation tables for a particular namespace is
// rooted by a public key stored in a flat root key listing, which is
// the entry point for lookup operations.  Well-known application
// identifier strings denote the namespace they control.

// If an application begins to run out of allowance (too many cells or large delegations),
// it can sign and nominate a new "rootentry" for the same application
// identifier with a larger value, at which point the other nodes can
// (given global knowledge of table sizes and growth rates, along with
// additional real-world information, if applicable) determine whether
// or not to accept the change.

// TODO: my understanding is that each root entry represents a merkle tree?
pub struct RootEntry<'a> {
    namespace_root_key: PublicKey,
    application_identifier: &'a str,
    listing_sig: Signature,
    allowance: u32,
    // TODO: This should point to some Merkle tree?
}

pub struct RootListing<'a> {
    roots: Vec<RootEntry<'a>>,
    merkle_tree: MerkleTree,
}

impl<'a> Default for RootListing<'a> {
    fn default() -> Self {
        Self {
            roots: Default::default(),
            merkle_tree: Default::default(),
        }
    }
}

impl<'a> RootListing<'a> {
    pub fn get_entry(
        &mut self,
        namespace_root_key: &PublicKey,
        application_identifier: &'a str,
    ) -> RootOpResult<&mut RootEntry<'a>> {
        if let Some(entry) = self.roots.iter_mut().find(|entry| {
            entry.namespace_root_key == *namespace_root_key
                && entry.application_identifier == application_identifier
        }) {
            Ok(entry)
        } else {
            Err(RootOpError::RootEntryNotExists)
        }
    }
}

impl<'a> RootEntry<'a> {}
