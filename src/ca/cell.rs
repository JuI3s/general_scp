use serde::Serialize;

use super::{
    crypto::{mock_public_key, mock_sig, PublicKey, SCPSignature},
    merkle::MerkleHash,
    table::{HTable, TableId},
};
use crate::ca::ca_type::Timestamp;
use std::{
    fmt::Debug,
    time::{SystemTime, UNIX_EPOCH},
};

type CellOpResult<T> = std::result::Result<T, CellOpError>;

#[derive(PartialEq)]
pub enum CellOpError {
    CommitmentNotExpires,
    InvalidSignature,
    Unknown,
}

#[derive(PartialEq)]
pub enum InnerCellType {
    Value,
    Delegate,
    Invalid,
}
#[derive(Clone)]
pub enum InnerCell {
    ValueCell(InnerValueCell),
    DelegateCell(InnerDelegateCell),
    // TODO: Needed for merkle tree library.
    Invalid,
}

// Delegate cells have a similar structure but different semantics.
// Rather than resolving to an individual mapping, they authorize the
// _delegee_ to create arbitrary value cells within a table mapped to
// the assigned namespace.  This namespace must be a subset of the
// _delegator_'s own namespace range.  Like the table authority, the
// delegee is uniquely identified by their public key.  Each delegate
// cell and subsequent updates to the cell are signed by the delegator -
// this ensures that the delegee cannot unilaterally modify its
// namespace, which limits the range of mappings they can create to
// those legitimately assigned to them.
#[derive(Clone, Debug)]
pub struct InnerDelegateCell {
    // opaque namespace<>
    pub name_space: String,
    pub allowance: u32,
    pub table: Option<TableId>,
}

// Value cells store individual mapping values.  They resolve a lookup
// key to an arbitrary value, for example, an encryption key associated
// with an email address or the zone files associated with a particular
// domain.  The public key of the cell's owner (e.g. the email account
// holder, the domain owner) is also included, as well as a signature
// authenticating the current version of the cell.  Since the cell's
// contents are controlled by the owner, its "value_sig" must be made by
// the "owner_key".  The cell owner may rotate ptheir public key at any
// time by signing the update with the old key.p

#[derive(Clone, Debug)]
pub struct InnerValueCell {
    // opaque value<>
    value: String,
}

#[derive(Clone)]
pub struct ValueCell {
    pub create_time: Timestamp,
    pub revision_time: Timestamp,
    pub commitment_time: Timestamp,
    pub inner_cell: Option<InnerValueCell>,
    pub authority_sig: SCPSignature,
}

#[derive(Clone)]
pub struct DelegateCell {
    pub create_time: Timestamp,
    pub revision_time: Timestamp,
    pub commitment_time: Timestamp,
    pub inner_cell: Option<InnerDelegateCell>,
    pub authority_sig: SCPSignature,
    pub table: Option<HTable>,
}

// AsRef<[u8]>,
#[derive(Clone)]
pub struct Cell {
    pub create_time: Timestamp,
    pub revision_time: Timestamp,
    pub commitment_time: Timestamp,
    pub sig: SCPSignature,
    pub owner_key: PublicKey,
    pub inner: CellData,
}

impl Debug for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cell")
            .field("create_time", &self.create_time)
            .field("revision_time", &self.revision_time)
            .field("commitment_time", &self.commitment_time)
            .field("inner", &self.inner)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub enum CellData {
    Value(InnerValueCell),
    Delegate(InnerDelegateCell),
}

pub fn timestamp_now() -> u64 {
    let now = SystemTime::now();
    now.duration_since(UNIX_EPOCH).unwrap().as_secs()
}

impl Cell {
    pub fn contains_prefix(&self, prefix: &str) -> bool {
        match &self.inner {
            CellData::Value(value_cell) => value_cell.value.starts_with(prefix),
            CellData::Delegate(delegate_cell) => delegate_cell.name_space.starts_with(prefix),
        }
    }

    pub fn set_modify_timestamp(&mut self) -> CellOpResult<()> {
        let now = timestamp_now();

        if now <= self.commitment_time {
            return Err(CellOpError::CommitmentNotExpires);
        }

        self.revision_time = now;
        self.commitment_time = now;

        Ok(())
    }

    pub fn is_valid(&self) -> CellOpResult<()> {
        // TODO: for now, just check the signature is valid
        // TODO: need to check this is safe
        if !self.sig.verify(b"Ok") {
            return Err(CellOpError::InvalidSignature);
        }

        Ok(())
    }

    pub fn is_value_cell(&self) -> bool {
        match &self.inner {
            CellData::Value(_) => true,
            CellData::Delegate(_) => false,
        }
    }

    fn commitment_expires(&self, timestamp: &Timestamp) -> CellOpResult<()> {
        if &self.commitment_time < timestamp {
            Ok(())
        } else {
            CellOpResult::Err(CellOpError::CommitmentNotExpires)
        }
    }

    fn modify(&mut self) -> CellOpResult<&Self> {
        let now = timestamp_now();
        if let Err(err) = self.commitment_expires(&now) {
            Err(err)
        } else {
            self.revision_time = now;

            todo!()
        }
    }

    pub fn name_space_or_value<'a>(&'a self) -> &String {
        match &self.inner {
            CellData::Value(value_cell) => &value_cell.value,
            CellData::Delegate(delegate_cell) => &delegate_cell.name_space,
        }
    }

    pub fn allowance(&self) -> u32 {
        match &self.inner {
            CellData::Value(_) => 1,
            CellData::Delegate(delegate_cell) => delegate_cell.allowance.to_owned(),
        }
    }

    pub fn inner_cell_type(&self) -> InnerCellType {
        match self.inner {
            CellData::Value(_) => InnerCellType::Value,
            CellData::Delegate(_) => InnerCellType::Delegate,
        }
    }

    pub fn is_prefix_of(&self, cell: &Cell) -> bool {
        cell.name_space_or_value()
            .starts_with(self.name_space_or_value())
    }

    pub fn contains_prefix_in_cell(&self, cell: &Cell) -> bool {
        self.name_space_or_value()
            .starts_with(cell.name_space_or_value())
    }
}

pub fn test_make_new_delegate_cell(name_space: String, allowance: u32) -> Cell {
    Cell {
        create_time: timestamp_now(),
        revision_time: timestamp_now(),
        commitment_time: timestamp_now(),
        sig: mock_sig(),
        owner_key: mock_public_key(),
        inner: CellData::Delegate(InnerDelegateCell {
            name_space,
            allowance,
            table: None,
        }),
    }
}
pub fn test_make_new_value_cell(value: String, commitment_time: Timestamp) -> Cell {
    Cell {
        create_time: timestamp_now(),
        revision_time: timestamp_now(),
        commitment_time,
        sig: mock_sig(),
        owner_key: mock_public_key(),
        inner: CellData::Value(InnerValueCell { value }),
    }
}

#[cfg(test)]
mod tests {
    use crate::ca::crypto::mock_fake_signature;

    use super::*;

    #[test]
    fn error_updating_before_commitment_timestamp_expires() {
        let cell = test_make_new_value_cell("".to_string(), 1);

        assert!(cell
            .commitment_expires(&0)
            .is_err_and(|err| { matches!(err, CellOpError::CommitmentNotExpires) }));

        assert!(cell
            .commitment_expires(&1)
            .is_err_and(|err| { matches!(err, CellOpError::CommitmentNotExpires) }));

        assert!(cell.commitment_expires(&2).is_ok())
    }

    #[test]
    fn invalid_cell() {
        let cell_invalid_sig = Cell {
            create_time: timestamp_now(),
            revision_time: timestamp_now(),
            commitment_time: timestamp_now(),
            sig: mock_fake_signature(),
            owner_key: mock_public_key(),
            inner: CellData::Value(InnerValueCell { value: "".into() }),
        };

        assert!(cell_invalid_sig
            .is_valid()
            .is_err_and(|err| { err == CellOpError::InvalidSignature }));
    }
}

impl<'a> PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.name_space_or_value() == other.name_space_or_value()
    }
}

impl<'a> Eq for Cell {}

impl<'a> PartialOrd for Cell {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.name_space_or_value()
            .partial_cmp(other.name_space_or_value())
    }
}

impl<'a> Default for Cell {
    fn default() -> Self {
        Cell {
            create_time: Default::default(),
            revision_time: Default::default(),
            commitment_time: Default::default(),
            sig: mock_sig(),
            owner_key: mock_public_key(),
            inner: CellData::Value(InnerValueCell { value: "".into() }),
        }
    }
}

impl Cell {
    pub fn to_merkle_hash(&self) -> Option<MerkleHash> {
        todo!()
        // Implement serialization for cell.
    }
}
