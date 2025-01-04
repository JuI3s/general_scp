use core::hash;
use std::{collections::HashMap, fmt::Debug, hash::Hash};

use digest::impl_oid_carrier;
use dsa::Signature;
use serde::{Deserialize, Serialize};

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
#[derive(Clone, Serialize, Deserialize)]
pub struct RootEntry {
    pub namespace_root_key: PublicKey,
    pub application_identifier: String,
    pub listing_sig: SCPSignature,
    pub allowance: u32,
    // TODO: This should point to some Merkle tree?
}

impl Hash for RootEntry {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.application_identifier.hash(state);
        self.allowance.hash(state);
    }
}

impl PartialEq for RootEntry {
    fn eq(&self, other: &Self) -> bool {
        self.namespace_root_key == other.namespace_root_key && self.allowance == other.allowance
    }
}

impl Eq for RootEntry {}

impl PartialOrd for RootEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RootEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.application_identifier
            .cmp(&other.application_identifier)
            .then(self.allowance.cmp(&other.allowance))
    }
}

impl Debug for RootEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RootEntry")
            .field("application_identifier", &self.application_identifier)
            .field("allowance", &self.allowance)
            .finish()
    }
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
