use core::time;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    ca_type::{PublicKey, Signature, Timestamp},
    operation::ReturnValueCell,
};

type CellOptResult<T> = std::result::Result<T, CellOpError>;

pub enum CellOpError {
    CommitmentNotExpires,
    Unknown,
}

pub struct DelegateCell<'a> {
    // opaque namespace<>
    name_space: &'a str,
    delegate: PublicKey,
    // Table authority controls delegations, not delegee
    delegatoin_sig: Signature,
    allowance: u32,
}

pub enum InnerCell<'a> {
    ValueCell(ValueCell<'a>),
    DelegateCell(DelegateCell<'a>),
}

pub struct ValueCell<'a> {
    // opaque value<>
    value: &'a str,
    public_key: PublicKey,
    value_sig: Signature,
}

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
    fn check_commitment_expires(&self, timestamp: Timestamp) -> CellOptResult<()> {
        if self.commitment_time < timestamp {
            Ok(())
        } else {
            CellOptResult::Err(CellOpError::CommitmentNotExpires)
        }
    }

    fn modify(&self) -> CellOptResult<ReturnValueCell> {
        if let Err(err) = self.check_commitment_expires(timestamp_now()) {
            Err(err)
        } else {
            todo!()
        }
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
                public_key: [0; 64],
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
