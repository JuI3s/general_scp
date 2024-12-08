use std::time::{SystemTime, UNIX_EPOCH};


use super::{
    ca_type::{mock_public_key, PublicKey, SCPSignature, Timestamp},
    merkle::MerkleHash,
    table::HTable,
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
#[derive(Clone)]
pub struct InnerDelegateCell {
    // opaque namespace<>
    name_space: String,
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
pub struct InnerValueCell {
    // opaque value<>
    value: String,
    owner_key: PublicKey,
    value_sig: SCPSignature,
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
pub enum Cell {
    Value(ValueCell),
    Delegate(DelegateCell),
}

pub enum CellRef<'a> {
    Value(&'a mut ValueCell),
    Delegate(&'a mut DelegateCell),
}

fn timestamp_now() -> u64 {
    let now = SystemTime::now();
    now.duration_since(UNIX_EPOCH).unwrap().as_secs()
}

impl ValueCell {
    pub fn set_modify_timestamp(&mut self) -> CellOpResult<()> {
        let now = timestamp_now();

        if now <= self.commitment_time {
            return Err(CellOpError::CommitmentNotExpires);
        }

        self.revision_time = now;
        self.commitment_time = now;

        Ok(())
    }

    pub fn is_prefix_of_cell(&self, cell: &Cell) -> bool {
        self.inner_cell.as_ref().is_some_and(|inner| {
            cell.name_space_or_value()
                .is_some_and(|str| str.starts_with(&inner.value))
        })
    }

    pub fn equals_prefix(&self, prefix: &String) -> bool {
        self.inner_cell
            .as_ref()
            .is_some_and(|inner| inner.value.eq(prefix))
    }

    pub fn contains_prefix_from_cell(&self, cell: &Cell) -> bool {
        self.inner_cell.as_ref().is_some_and(|inner| {
            cell.name_space_or_value()
                .is_some_and(|prefix| inner.value.starts_with(prefix))
        })
    }

    pub fn contains_prefix(&self, prefix: &String) -> bool {
        self.inner_cell
            .as_ref()
            .is_some_and(|inner| inner.value.starts_with(prefix))
    }
}

impl DelegateCell {
    pub fn set_modify_timestamp(&mut self) -> CellOpResult<()> {
        let now = timestamp_now();

        if now <= self.commitment_time {
            return Err(CellOpError::CommitmentNotExpires);
        }

        self.revision_time = now;
        self.commitment_time = now;

        Ok(())
    }

    pub fn namespace<'a>(&'a self) -> Option<&'a String> {
        self.inner_cell.as_ref().map(|v| &v.name_space)
    }

    pub fn allowance(&self) -> Option<u32> {
        match &self.inner_cell {
            Some(inner) => Some(inner.allowance),
            None => None,
        }
    }

    pub fn equals_prefix(&self, prefix: &String) -> bool {
        self.inner_cell
            .as_ref()
            .is_some_and(|inner| inner.name_space.eq(prefix))
    }

    pub fn is_prefix_of_cell(&self, cell: &Cell) -> bool {
        self.inner_cell.as_ref().is_some_and(|inner| {
            cell.name_space_or_value()
                .is_some_and(|str| str.starts_with(&inner.name_space))
        })
    }

    pub fn contains_prefix_from_cell(&self, cell: &Cell) -> bool {
        self.inner_cell.as_ref().is_some_and(|inner| {
            cell.name_space_or_value()
                .is_some_and(|prefix| inner.name_space.starts_with(prefix))
        })
    }

    pub fn is_prefix_of(&self, prefix: &String) -> bool {
        self.inner_cell
            .as_ref()
            .is_some_and(|inner| prefix.starts_with(&inner.name_space))
    }

    pub fn contains_prefix(&self, prefix: &String) -> bool {
        self.inner_cell
            .as_ref()
            .is_some_and(|inner| inner.name_space.starts_with(prefix))
    }
}

impl Cell {
    pub fn signature(&self) -> &SCPSignature {
        match &self {
            Cell::Value(cell) => &cell.authority_sig,
            Cell::Delegate(cell) => &cell.authority_sig,
        }
    }

    pub fn commitment_time(&self) -> &Timestamp {
        match self {
            Cell::Value(cell) => &cell.commitment_time,
            Cell::Delegate(cell) => &cell.commitment_time,
        }
    }
    pub fn revision_time(&self) -> &Timestamp {
        match self {
            Cell::Value(cell) => &cell.revision_time,
            Cell::Delegate(cell) => &cell.revision_time,
        }
    }

    pub fn set_commitment_time(&mut self, timestamp: Timestamp) {
        match self {
            Cell::Value(cell) => cell.commitment_time = timestamp,
            Cell::Delegate(cell) => cell.commitment_time = timestamp,
        }
    }

    pub fn set_revision_time(&mut self, timestamp: Timestamp) {
        match self {
            Cell::Value(cell) => cell.revision_time = timestamp,
            Cell::Delegate(cell) => cell.revision_time = timestamp,
        }
    }

    pub fn is_valid(&self) -> CellOpResult<()> {
        // TODO: for now, just check the signature is vali
        if !self.signature().verify() {
            return Err(CellOpError::InvalidSignature);
        }

        Ok(())
    }

    pub fn is_value_cell(&self) -> bool {
        match self {
            Cell::Value(_) => true,
            Cell::Delegate(_) => false,
        }
    }

    pub fn test_new_delegate_cell(name_space: String, allowance: u32) -> Self {
        Cell::Delegate(DelegateCell {
            create_time: timestamp_now(),
            revision_time: timestamp_now(),
            commitment_time: timestamp_now(),
            authority_sig: Default::default(),
            inner_cell: Some(InnerDelegateCell {
                name_space,
                delegate: mock_public_key(),
                delegatoin_sig: Default::default(),
                allowance: allowance,
            }),
            table: None,
        })
    }

    pub fn test_new_value_cell(value: String) -> Self {
        Cell::Value(ValueCell {
            create_time: Default::default(),
            revision_time: Default::default(),
            commitment_time: Default::default(),
            authority_sig: Default::default(),
            inner_cell: Some(InnerValueCell {
                value: value,
                owner_key: mock_public_key(),
                value_sig: Default::default(),
            }),
        })
    }

    fn check_commitment_expires(&self, timestamp: &Timestamp) -> CellOpResult<()> {
        if self.commitment_time() < timestamp {
            Ok(())
        } else {
            CellOpResult::Err(CellOpError::CommitmentNotExpires)
        }
    }

    fn modify(&mut self) -> CellOpResult<&Self> {
        let now = timestamp_now();
        if let Err(err) = self.check_commitment_expires(&now) {
            Err(err)
        } else {
            self.set_revision_time(now);

            todo!()
        }
    }

    pub fn name_space_or_value<'a>(&'a self) -> Option<&'a String> {
        match &self {
            Cell::Value(val) => val.inner_cell.as_ref().map(|c| &c.value),

            // val.inner_cell.map(|c|{&c.value}),
            Cell::Delegate(del) => del.inner_cell.as_ref().map(|c| &c.name_space),
        }
    }

    pub fn allowance(&self) -> Option<u32> {
        match &self {
            Cell::Value(cell) => cell.inner_cell.as_ref().map(|_| 1),
            Cell::Delegate(cell) => cell.inner_cell.as_ref().map(|c| c.allowance),
        }
    }

    pub fn inner_cell_type(&self) -> InnerCellType {
        match self {
            Cell::Value(_) => InnerCellType::Value,
            Cell::Delegate(_) => InnerCellType::Delegate,
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

    fn make_test_value_cell<'a>(commitment_timestamp: Timestamp) -> Cell {
        Cell::Value(ValueCell {
            create_time: 0,
            revision_time: 0,
            commitment_time: commitment_timestamp,
            authority_sig: Default::default(),
            inner_cell: Some(InnerValueCell {
                value: "".into(),
                owner_key: mock_public_key(),
                value_sig: Default::default(),
            }),
        })
    }

    #[test]
    fn error_updating_before_commitment_timestamp_expires() {
        let cell = make_test_value_cell(1);

        assert!(cell
            .check_commitment_expires(&0)
            .is_err_and(|err| { matches!(err, CellOpError::CommitmentNotExpires) }));

        assert!(cell
            .check_commitment_expires(&1)
            .is_err_and(|err| { matches!(err, CellOpError::CommitmentNotExpires) }));

        assert!(cell.check_commitment_expires(&2).is_ok())
    }

    #[test]
    fn invalid_cell() {
        let cell_invalid_sig = Cell::Value(ValueCell {
            create_time: timestamp_now(),
            revision_time: timestamp_now(),
            commitment_time: timestamp_now(),
            authority_sig: SCPSignature::test_gen_fake_signature(),
            inner_cell: Some(InnerValueCell {
                value: "".into(),
                owner_key: mock_public_key(),
                value_sig: Default::default(),
            }),
        });

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
        if let Some(self_val) = self.name_space_or_value() {
            if let Some(other_val) = other.name_space_or_value() {
                return self_val.partial_cmp(other_val);
            }
        }

        None
    }
}

impl<'a> Default for Cell {
    fn default() -> Self {
        Cell::Value(ValueCell {
            create_time: Default::default(),
            revision_time: Default::default(),
            commitment_time: Default::default(),
            authority_sig: Default::default(),
            inner_cell: Some(InnerValueCell {
                value: "".into(),
                owner_key: mock_public_key(),
                value_sig: Default::default(),
            }),
        })
    }
}

impl Cell {
    pub fn to_merkle_hash(&self) -> Option<MerkleHash> {
        todo!()
        // Implement serialization for cell.
    }
}
