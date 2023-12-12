use std::collections::BTreeSet;

use crate::ca::table;

use super::{
    ca_type::{PublicKey, SCPSignature},
    cell::{Cell, InnerCellType},
    merkle::MerkleTree,
};

type TableOpResult<T> = std::result::Result<T, TableOpError>;

#[derive(PartialEq, Debug)]
pub enum TableOpError {
    NamespaceError,
    CellAddressIsPrefix,
    // NotEnoughAllowence(allowance_capacity, allowance_filled)
    NotEnoughAllowence(u32, u32),
    EmptyCell,
}

// Every cell is stored in a table, which groups all the mappings
// created by a single authority public key for a specific namespace.
// Individual cells are referenced by an application-specific label in a
// lookup table. _The combination of a lookup key and a referenced cell
// value forms a mapping_.

//     struct tableentry {
//         opaque lookup_key<>;
//         cell c;
//     }

// Delegating the whole or part of a namespace requires adding a new
// lookup key for the namespace and a matching delegate cell.  Each
// delegation must be validated in the context of the other table
// entries and the table itself.  For example, the owner of a table
// delegated an /8 IPv4 block must not to delegate the same /16 block to
// two different tables.
pub struct TableEntry<'a> {
    // opaque lookup_key<>
    lookup_key: &'a str,
    cell: &'a Cell<'a>,
}

pub struct Table<'a> {
    allowance: u32,
    table_entries: BTreeSet<TableEntry<'a>>,
    merkle_tree: MerkleTree,
}

pub struct RootEntry<'a> {
    namespace_root_key: PublicKey,
    application_identifier: &'a str,
    listing_sig: SCPSignature,
    allowance: u32,
}

impl<'a> Eq for TableEntry<'a> {}

impl<'a> PartialEq for TableEntry<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.lookup_key == other.lookup_key
    }
}

impl<'a> PartialOrd for TableEntry<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.lookup_key.partial_cmp(other.lookup_key)
    }
}

impl<'a> Ord for TableEntry<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.lookup_key.cmp(other.lookup_key)
    }
}

impl<'a> TableEntry<'a> {
    //    Delegating the whole or part of a namespace requires adding a new
    //    lookup key for the namespace and a matching delegate cell.  Each
    //    delegation must be validated in the context of the other table
    //    entries and the table itself.  For example, the owner of a table
    //    delegated an /8 IPv4 block must not to delegate the same /16 block to
    //    two different tables.
    pub fn delegate(&mut self) {}

    pub fn check_cell_valid(&self, cell: &Cell) -> TableOpResult<()> {
        if let Some(val) = cell.name_space_or_value() {
            if !val.starts_with(self.lookup_key) {
                Err(TableOpError::NamespaceError)
            } else {
                Ok(())
            }
        } else {
            Err(TableOpError::EmptyCell)
        }
    }
}

impl<'a> Default for Table<'a> {
    fn default() -> Self {
        Self {
            allowance: Table::DEFAULT_ALLOWANCE,
            table_entries: Default::default(),
            merkle_tree: Default::default(),
        }
    }
}

impl<'a> Table<'a> {
    const DEFAULT_ALLOWANCE: u32 = 100;

    pub fn new(allowance: u32) -> Self {
        Self {
            allowance,
            table_entries: Default::default(),
            merkle_tree: Default::default(),
        }
    }

    pub fn add_entry(&mut self, cell: &'a Cell<'a>) -> TableOpResult<()> {
        if let Some(val) = cell.name_space_or_value() {
            self.table_entries.insert(TableEntry {
                lookup_key: val,
                cell: cell,
            });
            Ok(())
        } else {
            Err(TableOpError::EmptyCell)
        }
    }

    pub fn remove_entry(&mut self, prompt: &str) {
        self.table_entries.retain(|entry| {
            !entry
                .cell
                .name_space_or_value()
                .is_some_and(|val| val == prompt)
        });
    }

    //    2.3.  Prefix-based Delegation Correctness

    //    To generalize correctness, each table must conform with a prefix-
    //    based rule: for every cell with value or delegation subset "c" in a
    //    table controlling namespace "n", "n" must
    //
    //    (1) be a prefix of "c" and
    //    (2) there cannot exist another cell with value or delegation subset
    //    "c2" such that "c" is a prefix of "c2".

    //    While there exist many more hierarchical naming schemes, many can be
    //    simply represented in a prefix scheme.  For example, suffix-based
    //    delegations, including domain name hierarchies, can use reversed keys
    //    internally and perform a swap in the application layer before
    //    displaying any results to clients.  Likewise, 'flat' delegation
    //    schemes where there is no explicit restriction can use an empty
    //    prefix.
    pub fn check_cell_valid(&self, cell: &Cell) -> TableOpResult<()> {
        if !self.table_entries.iter().any(|table_entry| {
            table_entry.cell.inner_cell_type() == InnerCellType::Delegate
                && table_entry.cell.is_prefix_of(cell)
        }) {
            return Err(TableOpError::NamespaceError);
        }

        // TODO: do not iterate over the whole thing for performance optimization.
        if self
            .table_entries
            .iter()
            .any(|table_entry| cell.is_prefix_of(&table_entry.cell))
        {
            return Err(TableOpError::CellAddressIsPrefix);
        }

        Ok(())
    }

    pub fn contains_enough_allowance(&self, allowance: u32) -> TableOpResult<()> {
        if self.allowance == 0 {
            return Ok(());
        }

        if let Some(Some(current_allowance)) = self
            .table_entries
            .iter()
            .map(|entry| entry.cell.allowance())
            .reduce(|acc, e| {
                if let Some(_acc) = acc {
                    if let Some(_e) = e {
                        return Some(_acc + _e);
                    }
                }
                None
            })
        {
            if current_allowance + allowance > self.allowance {
                return Err(TableOpError::NotEnoughAllowence(
                    self.allowance,
                    current_allowance,
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_delegation_rule() {
        let mut entries: Table<'_> = Default::default();
        let home_cell = Cell::new_delegate_cell("home/", 1);
        let cell1 = Cell::new_value_cell("home/1");

        assert!(entries
            .check_cell_valid(&cell1)
            .is_err_and(|err| { err == TableOpError::NamespaceError }));
        assert!(entries.add_entry(&home_cell).is_ok());
        assert!(entries.check_cell_valid(&cell1).is_ok());

        assert!(entries.add_entry(&cell1).is_ok());
        assert!(entries
            .check_cell_valid(&cell1)
            .is_err_and(|err| { err == TableOpError::CellAddressIsPrefix }));

        let cell2 = Cell::new_value_cell("home/");
        let cell3 = Cell::new_value_cell("home/1/2");

        assert!(entries
            .check_cell_valid(&cell2)
            .is_err_and(|err| { err == TableOpError::CellAddressIsPrefix }));
        assert!(entries
            .check_cell_valid(&cell1)
            .is_err_and(|err| { err == TableOpError::CellAddressIsPrefix }));
        assert!(entries.check_cell_valid(&cell3).is_ok());
    }

    #[test]
    fn allowance() {
        let mut table = Table::new(1);
        assert!(table.contains_enough_allowance(1).is_ok());

        let home_cell = Cell::new_delegate_cell("home/", 1);
        assert!(table.add_entry(&home_cell).is_ok());

        assert!(table
            .contains_enough_allowance(1)
            .is_err_and(|err| { err == TableOpError::NotEnoughAllowence(1, 1) }));
    }
}
