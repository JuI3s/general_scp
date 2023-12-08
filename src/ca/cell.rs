use core::time;
use std::time::{SystemTime, UNIX_EPOCH};

use syn::token::Or;

use super::{
    ca_type::{mock_public_key, PublicKey, Signature, Timestamp},
    operation::ReturnValueCell,
};

type CellOpResult<T> = std::result::Result<T, CellOpError>;

pub enum CellOpError {
    CommitmentNotExpires,
    InvalidSignature,
    Unknown,
}

#[derive(PartialEq)]
pub enum InnerCellType {
    Value,
    Delegate,
}
#[derive(Clone, Hash)]
pub enum InnerCell<'a> {
    ValueCell(ValueCell<'a>),
    DelegateCell(DelegateCell<'a>),
    // TODO: Needed for merkle tree library.
    Empty,
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
#[derive(Clone, Hash)]
pub struct DelegateCell<'a> {
    // opaque namespace<>
    name_space: &'a str,
    delegate: PublicKey,
    // Table authority controls delegations, not delegee
    delegatoin_sig: Signature,
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

#[derive(Clone, Hash)]
pub struct ValueCell<'a> {
    // opaque value<>
    value: &'a str,
    owner_key: PublicKey,
    value_sig: Signature,
}

// AsRef<[u8]>,
#[derive(Clone, Hash)]
pub struct Cell<'a> {
    // 64-bit UNIX timestamps
    create_time: Timestamp,
    revision_time: Timestamp,
    commitment_time: Timestamp,
    authority_sig: Signature,
    inner_cell: InnerCell<'a>,
}

fn timestamp_now() -> u64 {
    let now = SystemTime::now();
    now.duration_since(UNIX_EPOCH).unwrap().as_secs()
}

impl<'a> Cell<'a> {
    pub fn new_delegate_cell(name_space: &'a str, allowance: u32) -> Self {
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

    pub fn new_value_cell(value: &'a str) -> Self {
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

    pub fn name_space_or_value(&self) -> &'a str {
        match &self.inner_cell {
            InnerCell::ValueCell(val) => val.value,
            InnerCell::DelegateCell(del) => del.name_space,
            InnerCell::Empty => panic!("Inner cell is empty."),
        }
    }

    pub fn allowance(&self) -> u32 {
        match &self.inner_cell {
            InnerCell::ValueCell(_) => 1,
            InnerCell::DelegateCell(del) => del.allowance.to_owned(),
            InnerCell::Empty => panic!("Inner cell is empty."),
        }
    }

    pub fn inner_cell_type(&self) -> InnerCellType {
        match self.inner_cell {
            InnerCell::ValueCell(_) => InnerCellType::Value,
            InnerCell::DelegateCell(_) => InnerCellType::Delegate,
            InnerCell::Empty => panic!("Inner cell is empty."),
        }
    }

    pub fn is_prefix_of(&self, cell: &Cell) -> bool {
        cell.name_space_or_value()
            .starts_with(self.name_space_or_value())
    }

    pub fn contains_prefix(&self, cell: &Cell) -> bool {
        self.name_space_or_value()
            .starts_with(cell.name_space_or_value())
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
}

impl<'a> PartialEq for Cell<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.name_space_or_value() == other.name_space_or_value()
    }
}

impl<'a> Eq for Cell<'a> {}

impl<'a> PartialOrd for Cell<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.name_space_or_value()
            .partial_cmp(other.name_space_or_value())
    }
}

impl<'a> Ord for Cell<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name_space_or_value().cmp(other.name_space_or_value())
    }
}

impl<'a> Default for Cell<'a> {
    fn default() -> Self {
        Self {
            create_time: Default::default(),
            revision_time: Default::default(),
            commitment_time: Default::default(),
            authority_sig: Default::default(),
            inner_cell: InnerCell::Empty,
        }
    }
}
