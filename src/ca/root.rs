use digest::impl_oid_carrier;
use dsa::Signature;

use super::{
    ca_type::{PublicKey, SCPSignature},
    merkle::MerkleTree,
};

pub type RootOpResult<T> = std::result::Result<T, RootOpError>;
pub enum RootOpError {
    Unknown,
}

// Each linked group of delegation tables for a particular namespace is
// rooted by a public key stored in a flat root key listing, which is
// the entry point for lookup operations.  Well-known application
// identifier strings denote the namespace they control.

// If an application begins to run out of allowance (too many cells or large
// delegations), it can sign and nominate a new "rootentry" for the same
// application identifier with a larger value, at which point the other nodes
// can (given global knowledge of table sizes and growth rates, along with
// additional real-world information, if applicable) determine whether
// or not to accept the change.

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct RootEntryKey(pub String);

// TODO: my understanding is that each root entry represents a merkle tree?
pub struct RootEntry {
    namespace_root_key: PublicKey,
    application_identifier: String,
    listing_sig: SCPSignature,
    allowance: u32,
    // TODO: This should point to some Merkle tree?
}

pub struct RootListing {
    roots: Vec<RootEntry>,
    merkle_tree: Box<MerkleTree>,
}

impl Default for RootListing {
    fn default() -> Self {
        Self {
            roots: Default::default(),
            merkle_tree: Default::default(),
        }
    }
}

impl RootListing {
    pub fn get_entry_mut(
        &mut self,
        namespace_root_key: &PublicKey,
        application_identifier: String,
    ) -> Option<&mut RootEntry> {
        if let Some(entry) = self.roots.iter_mut().find(|entry| {
            entry.namespace_root_key == *namespace_root_key
                && entry.application_identifier == application_identifier
        }) {
            Some(entry)
        } else {
            None
        }
    }

    pub fn get_entry(
        &self,
        namespace_root_key: &PublicKey,
        application_identifier: String,
    ) -> Option<&RootEntry> {
        if let Some(entry) = self.roots.iter().find(|entry| {
            entry.namespace_root_key == *namespace_root_key
                && entry.application_identifier == application_identifier
        }) {
            Some(entry)
        } else {
            None
        }
    }
}

impl RootEntry {}
