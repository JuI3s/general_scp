use std::{
    collections::{BTreeMap, BTreeSet},
    os::unix::ffi::OsStrExt,
    sync::{Arc, Mutex},
};

use ct_merkle::inclusion::InclusionProof;
use sha2::Sha256;

use crate::{herder::herder::HerderDriver, scp::nomination_protocol::NominationValue};

use super::{
    ca_type::PublicKey,
    cell::Cell,
    merkle::MerkleTree,
    operation::{CellMerkleProof, SetOperation, TableMerkleProof},
    root::{RootEntry, RootEntryKey, RootListing},
    table::Table,
};

pub struct CAState<'a> {
    table_tree: MerkleTree,
    root_listing: RootListing<'a>,
    tables: BTreeMap<(RootEntryKey<'a>, &'static str), Table<'a>>,
}

#[derive(Hash, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct CANominationValue {}

pub type CAStateOpResult<T> = std::result::Result<T, CAStateOpError>;
pub enum CAStateOpError {
    MerkleTreeNotPresent,
    MerkleTreeChanged,
    MerkleProofInvalid,
    InvalidProof,
    InvalidCell,
}

impl NominationValue for CANominationValue {}

impl<'a> Default for CAState<'a> {
    fn default() -> Self {
        Self {
            table_tree: Default::default(),
            root_listing: Default::default(),
            tables: Default::default(),
        }
    }
}

impl<'a> CAState<'a> {
    pub fn validate_merkle_proof_for_table(
        &self,
        merkle_proof: &TableMerkleProof,
    ) -> CAStateOpResult<()> {
        if let Some(hash) = merkle_proof.table.to_merkle_hash() {
            if merkle_proof.root != self.table_tree.root() {
                return Err(CAStateOpError::MerkleTreeChanged);
            }

            match self.table_tree.veritfy_inclusion_proof(
                &hash,
                merkle_proof.idx,
                &merkle_proof.sibling_hashes,
            ) {
                Ok(_) => Ok(()),
                Err(_) => Err(CAStateOpError::InvalidProof),
            }
        } else {
            Err(CAStateOpError::InvalidProof)
        }
    }

    pub fn validate_merkle_proof_for_cell(
        &self,
        root_key: RootEntryKey,
        merkle_proof: &CellMerkleProof,
    ) -> CAStateOpResult<()> {
        if let Some(table) = self.tables.get(&(root_key, merkle_proof.key)) {
            // Check tree root has not changed.
            if table.merkle_tree.root() != merkle_proof.root {
                return Err(CAStateOpError::MerkleTreeChanged);
            }

            // Check inclusion proof
            if merkle_proof
                .entry_cell
                .to_merkle_hash()
                .is_some_and(|hash| {
                    table
                        .merkle_tree
                        .veritfy_inclusion_proof(
                            &hash,
                            merkle_proof.idx,
                            &merkle_proof.sibling_hashes,
                        )
                        .is_ok()
                })
            {
                Ok(())
            } else {
                Err(CAStateOpError::InvalidProof)
            }
        } else {
            Err(CAStateOpError::MerkleTreeNotPresent)
        }
    }

    pub fn validate_set_operation(&self, set_opt: &SetOperation) -> bool {
        todo!()
    }

    pub fn insert_cell(&mut self, cell: &Cell) {
        todo!()
    }

    pub fn insert_root_entry(&mut self, root_entry: &RootEntry) {
        todo!()
    }

    pub fn get_cell_mut(&mut self, cell: &Cell) -> CAStateOpResult<Option<&mut Cell<'a>>> {
        if let Some(key) = cell.name_space_or_value() {
            // if let Some(root_entry) = self.root_listing.
            todo!()
            // Ok(())
        } else {
            Err(CAStateOpError::InvalidCell)
        }
    }

    pub fn contains_root_entry(
        &self,
        namespace_root_key: &PublicKey,
        application_identifier: &'a str,
    ) -> bool {
        self.root_listing
            .get_entry(namespace_root_key, application_identifier)
            .is_some()
    }

    pub fn to_toml(&self) {
        todo!()
    }

    pub fn from_toml(&self) {
        todo!()
    }
}

// Creating or updating a cell at a specified path requires once again
// the full lookup key, as well as the new version of the cell to place.
// The new cell must be well-formed under the validation checks
// described in the previous section, else an "ERROR" is returned.  For
// example, updating a cell's owner without a signature by the previous
// owning key should not succeed.  Both value cells and new/updated
// delegations may be created through this method.  Removing cells from
// tables (after their commitment timestamps have expired) can be
// accomplished by replacing the value or delegated namespace with an
// empty value and setting the owner's key to that of the table
// authority.  Asking the consensus layer to approve a new root entry
// follows a similar process, although the application identifier and
// lookup key is unnecessary (see "SetRootOperation").  Nodes can also
// trigger votes to remove entries from the root key listing to redress
// misbehaving applications.

#[cfg(test)]
mod tests {

    use super::*;
}
