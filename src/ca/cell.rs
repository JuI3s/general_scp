use core::time;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use syn::token::{Or, SelfValue};

use super::{
    ca_type::{mock_public_key, PublicKey, SCPSignature, Timestamp},
    merkle::MerkleHash,
    operation::ReturnValueCell,
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
pub enum InnerCell<'a> {
    ValueCell(ValueCell<'a>),
    DelegateCell(DelegateCell<'a>),
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
#[derive(Clone)]
pub struct DelegateCell<'a> {
    // opaque namespace<>
    name_space: &'a str,
    delegate: PublicKey,
    // Table authority controls delegations, not delegee
    delegatoin_sig: SCPSignature,
    allowance: u32,
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

#[derive(Clone)]
pub struct ValueCell<'a> {
    // opaque value<>
    value: &'a str,
    owner_key: PublicKey,
    value_sig: SCPSignature,
}

// AsRef<[u8]>,
#[derive(Clone)]
pub struct Cell<'a> {
    // 64-bit UNIX timestamps
    create_time: Timestamp,
    revision_time: Timestamp,
    commitment_time: Timestamp,
    inner_cell: InnerCell<'a>,
    authority_sig: SCPSignature,
}

fn timestamp_now() -> u64 {
    let now = SystemTime::now();
    now.duration_since(UNIX_EPOCH).unwrap().as_secs()
}

impl<'a> Cell<'a> {
    fn signature(&self) -> Option<&SCPSignature> {
        match &self.inner_cell {
            InnerCell::ValueCell(val) => Some(&val.value_sig),
            InnerCell::DelegateCell(delegate) => Some(&delegate.delegatoin_sig),
            InnerCell::Invalid => None,
        }
    }

    pub fn is_valid(&self) -> CellOpResult<()> {
        // TODO: for now, just check the signature is valid
        if self.signature().is_some_and(|sig| !sig.verify()) {
            return Err(CellOpError::InvalidSignature);
        }

        Ok(())
    }

    pub fn test_new_delegate_cell(name_space: &'a str, allowance: u32) -> Self {
        Cell {
            create_time: timestamp_now(),
            revision_time: timestamp_now(),
            commitment_time: timestamp_now(),
            authority_sig: Default::default(),
            inner_cell: InnerCell::DelegateCell(DelegateCell {
                name_space,
                delegate: mock_public_key(),
                delegatoin_sig: Default::default(),
                allowance: allowance,
            }),
        }
    }

    pub fn test_new_value_cell(value: &'a str) -> Self {
        Cell {
            create_time: timestamp_now(),
            revision_time: timestamp_now(),
            commitment_time: timestamp_now(),
            authority_sig: Default::default(),
            inner_cell: InnerCell::ValueCell(ValueCell {
                value: value,
                owner_key: mock_public_key(),
                value_sig: Default::default(),
            }),
        }
    }

    fn check_commitment_expires(&self, timestamp: Timestamp) -> CellOpResult<()> {
        if self.commitment_time < timestamp {
            Ok(())
        } else {
            CellOpResult::Err(CellOpError::CommitmentNotExpires)
        }
    }

    fn modify(&mut self) -> CellOpResult<&'a Self> {
        if let Err(err) = self.check_commitment_expires(timestamp_now()) {
            Err(err)
        } else {
            self.revision_time = timestamp_now();

            todo!()
        }
    }

    pub fn name_space_or_value(&self) -> Option<&'a str> {
        match &self.inner_cell {
            InnerCell::ValueCell(val) => Some(val.value),
            InnerCell::DelegateCell(del) => Some(del.name_space),
            InnerCell::Invalid => None,
        }
    }

    pub fn allowance(&self) -> Option<u32> {
        match &self.inner_cell {
            InnerCell::ValueCell(_) => Some(1),
            InnerCell::DelegateCell(del) => Some(del.allowance.to_owned()),
            InnerCell::Invalid => None,
        }
    }

    pub fn inner_cell_type(&self) -> InnerCellType {
        match self.inner_cell {
            InnerCell::ValueCell(_) => InnerCellType::Value,
            InnerCell::DelegateCell(_) => InnerCellType::Delegate,
            InnerCell::Invalid => InnerCellType::Invalid,
        }
    }

    pub fn is_prefix_of(&self, cell: &Cell) -> bool {
        cell.name_space_or_value().is_some_and(|val| {
            self.name_space_or_value()
                .is_some_and(|self_val| val.starts_with(self_val))
        })
    }

    pub fn contains_prefix(&self, cell: &Cell) -> bool {
        self.name_space_or_value().is_some_and(|self_val| {
            cell.name_space_or_value()
                .is_some_and(|val| self_val.starts_with(val))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_value_cell<'a>(commitment_timestamp: Timestamp) -> Cell<'a> {
        Cell {
            create_time: 0,
            revision_time: 0,
            commitment_time: commitment_timestamp,
            authority_sig: Default::default(),
            inner_cell: InnerCell::ValueCell(ValueCell {
                value: "",
                owner_key: mock_public_key(),
                value_sig: Default::default(),
            }),
        }
    }

    #[test]
    fn error_updating_before_commitment_timestamp_expires() {
        let cell = make_test_value_cell(1);

        assert!(cell
            .check_commitment_expires(0)
            .is_err_and(|err| { matches!(err, CellOpError::CommitmentNotExpires) }));

        assert!(cell
            .check_commitment_expires(1)
            .is_err_and(|err| { matches!(err, CellOpError::CommitmentNotExpires) }));

        assert!(cell.check_commitment_expires(2).is_ok())
    }

    #[test]
    fn valid_cell() {
        let cell = Cell::default();
        assert!(cell.is_valid().is_ok());
    }

    #[test]
    fn invalid_cell() {
        let cell_invalid_sig = Cell {
            create_time: timestamp_now(),
            revision_time: timestamp_now(),
            commitment_time: timestamp_now(),
            authority_sig: Default::default(),
            inner_cell: InnerCell::ValueCell(ValueCell {
                value: "",
                owner_key: mock_public_key(),
                value_sig: SCPSignature::test_gen_fake_signature(),
            }),
        };

        assert!(cell_invalid_sig
            .is_valid()
            .is_err_and(|err| { err == CellOpError::InvalidSignature }));
    }
}

impl<'a> PartialEq for Cell<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.name_space_or_value() == other.name_space_or_value()
    }
}

impl<'a> Eq for Cell<'a> {}

impl<'a> PartialOrd for Cell<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if let Some(self_val) = self.name_space_or_value() {
            if let Some(other_val) = other.name_space_or_value() {
                return self_val.partial_cmp(other_val);
            }
        }

        None
    }
}

impl<'a> Default for Cell<'a> {
    fn default() -> Self {
        Self {
            create_time: Default::default(),
            revision_time: Default::default(),
            commitment_time: Default::default(),
            authority_sig: Default::default(),
            inner_cell: InnerCell::Invalid,
        }
    }
}

impl<'a> Cell<'a> {
    pub fn to_merkle_hash(&self) -> Option<MerkleHash> {
        todo!()
        // Implement serialization for cell.
    }
}
