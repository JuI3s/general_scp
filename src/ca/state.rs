use std::{
    ascii::AsciiExt,
    collections::{btree_map::Entry, BTreeMap, BTreeSet},
    marker::PhantomData,
    os::unix::ffi::OsStrExt,
    sync::{Arc, Mutex},
};

use ct_merkle::inclusion::InclusionProof;
use digest::Key;
use log::Log;
use sha2::Sha256;
use syn::braced;

use crate::{herder::herder::HerderDriver, scp::nomination_protocol::NominationValue};

use super::{
    ca_type::PublicKey,
    cell::Cell,
    merkle::MerkleTree,
    operation::{CellMerkleProof, SetOperation, TableMerkleProof},
    root::{RootEntry, RootEntryKey, RootListing},
    table::{Table, TableEntry},
};

pub struct CAState {
    table_tree: MerkleTree,
    root_listing: RootListing,
}

pub struct Tables(BTreeMap<RootEntryKey, BTreeMap<String, Table>>);

#[derive(Hash, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct CANominationValue {}

pub type CAStateOpResult<T> = std::result::Result<T, CAStateOpError>;
pub enum CAStateOpError {
    MerkleTreeNotPresent,
    MerkleTreeChanged,
    MerkleProofInvalid,
    InvalidProof,
    InvalidCell,
    RootTableNotFound,
}

impl NominationValue for CANominationValue {}

impl Default for CAState {
    fn default() -> Self {
        Self {
            table_tree: Default::default(),
            root_listing: Default::default(),
        }
    }
}

impl CAState {
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

    pub fn validate_merkle_proof_for_cell<'a>(
        &self,
        root_key: &RootEntryKey,
        merkle_proof: &CellMerkleProof,
        tables: &Tables,
    ) -> CAStateOpResult<()> {
        if let Some(table) = self.get_table(root_key, merkle_proof.key, tables) {
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

    fn get_table<'a>(
        &self,
        root_entry: &'a RootEntryKey,
        table_key: &'a str,
        tables: &'a Tables,
    ) -> Option<&'a Table> {
        if let Some(root_table) = tables.0.get(root_entry) {
            root_table.get(table_key)
        } else {
            None
        }
    }

    fn get_table_mut<'a>(
        &self,
        root_entry: &RootEntryKey,
        table_key: &String,
        tables: &'a mut Tables,
    ) -> Option<&'a mut Table> {
        if let Some(root_table) = tables.0.get_mut(root_entry) {
            root_table.get_mut(table_key)
        } else {
            None
        }
    }

    fn find_cell<'b>(
        &self,
        root_entry: &RootEntryKey,
        table_key: &String,
        cell: &Cell,
        tables: &'b mut Tables,
    ) -> Option<&'b mut Cell> {
        todo!()

        // This implementation suffers from problem case 3 in here
        // https://github.com/rust-lang/rfcs/blob/master/text/2094-nll.md#problem-case-3-conditional-control-flow-across-functions

        // Assume self is well formed.

        // let mut next_key = String::new();

        // if let Some(current_table) = self.get_table_mut(root_entry, &table_key, tables) {
        //     if let Some(cell_key) = cell.name_space_or_value() {
        //         let res = match current_table
        //             .value_entries
        //             .iter_mut()
        //             .find(|e| e.cell.is_prefix_of(cell))
        //         {
        //             Some(_entry) => Some(&mut _entry.cell),
        //             None => None,
        //         };

        //         if let Some(_cell) = res {
        //             next_key = _cell.name_space_or_value().unwrap().clone();
        //             {
        //                 if _cell.is_value_cell() {
        //                     if _cell.name_space_or_value().unwrap() == cell_key {
        //                         // return Some(_cell);
        //                     } else {
        //                         return None;
        //                     }
        //                 } else {
        //                     return self.find_cell(root_entry, &next_key, cell, tables);
        //                 }
        //             }
        //         } else {
        //             return None;
        //         }

        //         // return None;
        //     } else {
        //         return None;
        //     }
        // } else {
        //     return None;
        // }

        // let current_table = self.get_table_mut(root_entry, &table_key, tables).unwrap();
        // match current_table
        //     .value_entries
        //     .iter_mut()
        //     .find(|e| e.cell.is_prefix_of(cell))
        // {
        //     Some(entry) => Some(&mut entry.cell),
        //     None => unreachable!(),
        // }

        // return self.find_cell(root_entry, &next_key, cell,  tables);
    }

    fn get_root_table_mut<'a>(
        &mut self,
        root_key: &RootEntryKey,
        tables: &'a mut Tables,
    ) -> Option<&'a mut Table> {
        self.get_table_mut(root_key, &String::from(""), tables)
    }

    pub fn get_cell_mut(
        &mut self,
        root_key: RootEntryKey,
        cell: &Cell,
    ) -> CAStateOpResult<Option<&mut Cell>> {
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
        application_identifier: String,
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

    #[test]
    fn box_value() {
        let a = Box::new(1);
        let mut b = a.clone();
        let y = &mut b;
        *y = Box::new(2);
        assert_eq!(*a, 1);
    }
}
