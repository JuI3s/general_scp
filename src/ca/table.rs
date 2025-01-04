use std::{borrow::BorrowMut, cell::RefCell, collections::HashMap, rc::Rc};

use serde::{Deserialize, Serialize};

use super::{
    cell::Cell,
    merkle::{MerkleHash, MerkleTree},
};

pub type TableOpResult<T> = std::result::Result<T, TableOpError>;

#[derive(PartialEq, Debug)]
pub enum TableOpError {
    NamespaceError,
    CellAddressIsPrefix,
    CellAddressContainsPrefix,
    // NotEnoughAllowence(allowance_capacity, allowance_filled)
    NotEnoughAllowence(u32, u32),
    EmptyCell,
}

/// https://datatracker.ietf.org/doc/html/draft-watson-dinrg-delmap-01
///
/// Tables

/// Every cell is stored in a table, which groups all the mappings created by a single authority public key for a specific namespace. Individual cells are referenced by an application-specific label in a lookup table. _The combination of a lookup key and a referenced cell value forms a mapping_.

///     struct tableentry {
///         opaque lookup_key<>;
///         cell c;
///     }

/// Delegating the whole or part of a namespace requires adding a new lookup key for the namespace and a matching delegate cell.  Each delegation must be validated in the context of the other table entries and the table itself.  For example, the owner of a table delegated an /8 IPv4 block must not to delegate the same /16 block to two different tables.

pub struct TableMeta {
    pub allowance: u32,
    lookup_key: String,
}

impl TableMeta {
    pub fn to_merkle_hash(&self) -> Option<MerkleHash> {
        todo!()
    }
}

#[derive(Eq, PartialEq, Debug, Hash, Clone, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TableId(pub String);

impl TableId {
    pub fn root() -> Self {
        Self("".to_string())
    }

}

pub struct TableCollection(pub HashMap<TableId, Table>);

pub type HTable = Rc<RefCell<Table>>;
pub struct Table {
    pub allowance: u32,
    pub name_space: String,
    // Need to change this to a map
    pub value_entries: Vec<Cell>,
    pub delegate_entries: Vec<Cell>,
    pub merkle_tree: Box<MerkleTree>,
}

//    Delegating the whole or part of a namespace requires adding a new
//    lookup key for the namespace and a matching delegate cell.  Each
//    delegation must be validated in the context of the other table
//    entries and the table itself.  For example, the owner of a table
//    delegated an /8 IPv4 block must not to delegate the same /16 block to
//    two different tables.

impl Default for Table {
    fn default() -> Self {
        Self {
            allowance: Table::DEFAULT_ALLOWANCE,
            merkle_tree: Default::default(),
            name_space: "".to_string(),
            value_entries: Default::default(),
            delegate_entries: Default::default(),
        }
    }
}

impl Table {
    const DEFAULT_ALLOWANCE: u32 = 100;

    pub fn new(allowance: u32, namespace: String) -> Self {
        Self {
            allowance,
            value_entries: Default::default(),
            delegate_entries: Default::default(),
            merkle_tree: Default::default(),
            name_space: namespace,
        }
    }

    pub fn add_entry(&mut self, cell: Cell) -> TableOpResult<()> {
        match self.check_cell_valid(&cell) {
            Err(err) => Err(err),
            Ok(_) => match &cell.inner {
                super::cell::CellData::Value(_) => Ok(self.value_entries.push(cell)),
                super::cell::CellData::Delegate(_) => Ok(self.delegate_entries.push(cell)),
            },
        }
    }

    pub fn check_cell_valid(&self, cell: &Cell) -> TableOpResult<()> {
        // This function can be used to inductively check that after each insertion of a
        // new cell, the table remains valid based on the following rule.

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

        if !cell.contains_prefix(&self.name_space) {
            return Err(TableOpError::NamespaceError);
        }

        if self
            .value_entries
            .iter()
            .any(|table_entry| table_entry.contains_prefix(&cell.name_space_or_value()))
        {
            return Err(TableOpError::CellAddressIsPrefix);
        }

        if self
            .value_entries
            .iter()
            .any(|table_entry| table_entry.contains_prefix_in_cell(cell))
        {
            return Err(TableOpError::CellAddressContainsPrefix);
        }

        if self
            .delegate_entries
            .iter()
            .any(|table_entry| table_entry.contains_prefix_in_cell(cell))
        {
            return Err(TableOpError::CellAddressIsPrefix);
        }

        if self
            .delegate_entries
            .iter()
            .any(|table_entry| cell.contains_prefix_in_cell(table_entry))
        {
            return Err(TableOpError::CellAddressContainsPrefix);
        }

        Ok(())
    }

    pub fn contains_enough_allowance(&self, allowance: u32) -> TableOpResult<()> {
        if self.allowance == 0 {
            return Ok(());
        }

        let mut cur: u32 = 0;
        self.value_entries.iter().for_each(|e| {
            cur += 1;
        });

        self.delegate_entries.iter().for_each(|e| {
            cur += e.allowance();
        });

        if cur + allowance > self.allowance {
            Err(TableOpError::NotEnoughAllowence(self.allowance, cur))
        } else {
            Ok(())
        }
    }
}

pub fn find_delegation_cell<'a>(
    table_maps: &'a TableCollection,
    table_id: &TableId,
    key: &String,
) -> Option<&'a Cell> {
    let table = table_maps.0.get(table_id)?;

    for entry in &table.delegate_entries {
        match &entry.inner {
            super::cell::CellData::Value(_) => {}
            super::cell::CellData::Delegate(inner_delegate_cell) => {
                if entry.name_space_or_value() == key {
                    return Some(entry);
                }

                if let Some(new_table_id) = &inner_delegate_cell.table {
                    return find_delegation_cell(table_maps, new_table_id, key);
                }
            }
        }
    }

    None
}

pub fn find_value_cell<'a>(
    table_maps: &'a TableCollection,
    table_id: &TableId,
    key: &String,
) -> Option<&'a Cell> {
    let table = table_maps.0.get(table_id)?;

    if let Some(val) = table
        .value_entries
        .iter()
        .find(|e| e.name_space_or_value() == key)
    {
        return Some(val);
    }

    if let Some(del_entry) = table
        .delegate_entries
        .iter()
        .find(|e| key.starts_with(e.name_space_or_value()))
    {
        match &del_entry.inner {
            super::cell::CellData::Value(_) => panic!("This should not happen"),
            super::cell::CellData::Delegate(inner_delegate_cell) => {
                if let Some(new_table_id) = &inner_delegate_cell.table {
                    return find_value_cell(table_maps, new_table_id, key);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ca::cell::{test_make_new_delegate_cell, test_make_new_value_cell};

    #[test]
    fn prefix_delegation_rule() {
        let mut entries = Table::new(100, "home/".to_owned());
        let home_cell = test_make_new_delegate_cell(String::from("home/home"), 1);
        let cell1 = test_make_new_value_cell(String::from("home/cell1"), 0);

        assert!(entries
            .check_cell_valid(&test_make_new_value_cell(String::from("ho"), 1))
            .is_err_and(|err| { err == TableOpError::NamespaceError }));
        assert!(entries.add_entry(home_cell).is_ok());
        assert!(entries.check_cell_valid(&cell1).is_ok());

        assert!(entries.add_entry(cell1.clone()).is_ok());
        assert!(entries.check_cell_valid(&cell1).is_err_and(|err| {
            err == TableOpError::CellAddressIsPrefix
                || err == TableOpError::CellAddressContainsPrefix
        }));

        let cell2 = test_make_new_value_cell(String::from("home/"), 1);
        let cell3 = test_make_new_value_cell(String::from("home/1/2"), 2);

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
        let mut table = Table::new(1, "".to_string());
        assert!(table.contains_enough_allowance(1).is_ok());

        let home_cell = test_make_new_delegate_cell(String::from("home/"), 1);
        assert!(table.add_entry(home_cell).is_ok());

        assert!(table
            .contains_enough_allowance(1)
            .is_err_and(|err| { err == TableOpError::NotEnoughAllowence(1, 1) }));
    }
}
