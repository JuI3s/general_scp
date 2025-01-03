use std::collections::HashMap;

use digest::impl_oid_carrier;
use dsa::Signature;

use super::{
    crypto::{PrivateKey, PublicKey, SCPSignature},
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
#[derive(Clone)]
pub struct RootEntry {
    pub namespace_root_key: PublicKey,
    pub application_identifier: String,
    pub listing_sig: SCPSignature,
    pub allowance: u32,
    // TODO: This should point to some Merkle tree?
}

impl RootEntry {
    pub fn new(private_key: &PrivateKey, application_identifier: String) -> Self {
        let listing_sig = SCPSignature::sign(&private_key, &application_identifier.as_bytes());
        let namespace_root_key = PublicKey(private_key.0.verifying_key().to_owned());

        Self {
            namespace_root_key,
            application_identifier,
            listing_sig,
            allowance: 0,
        }
    }

    pub fn verify(&self) -> bool {
        todo!()
    }
}

#[derive(Default)]
pub struct RootListing(pub HashMap<String, RootEntry>);
