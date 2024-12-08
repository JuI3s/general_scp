use std::{borrow::BorrowMut, cell::RefCell, rc::Rc};



use super::{
    ca_type::{PublicKey, SCPSignature},
    cell::{Cell, DelegateCell, ValueCell},
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
pub enum TableEntry {
    Value(ValueEntry),
    Delegate(DelegateEntry),
}

pub type HValueEntry = Rc<RefCell<ValueEntry>>;
pub struct ValueEntry {
    pub cell: ValueCell,
}

pub type HDelegateEntry = Rc<RefCell<DelegateEntry>>;
pub struct DelegateEntry {
    pub cell: DelegateCell,
}

pub struct TableMeta {
    pub allowance: u32,
    lookup_key: String,
}

pub type HTable = Rc<RefCell<Table>>;
pub struct Table {
    pub allowance: u32,
    pub name_space: String,
    // Need to change this to a map
    pub value_entries: Vec<HValueEntry>,
    pub delegate_entries: Vec<HDelegateEntry>,
    pub merkle_tree: Box<MerkleTree>,
}

pub struct RootEntry<'a> {
    namespace_root_key: PublicKey,
    application_identifier: &'a str,
    listing_sig: SCPSignature,
    allowance: u32,
}

impl TableEntry {
    //    Delegating the whole or part of a namespace requires adding a new
    //    lookup key for the namespace and a matching delegate cell.  Each
    //    delegation must be validated in the context of the other table
    //    entries and the table itself.  For example, the owner of a table
    //    delegated an /8 IPv4 block must not to delegate the same /16 block to
    //    two different tables.
    pub fn delegate(&mut self) {}
}

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
            Ok(_) => match cell {
                Cell::Value(cell) => Ok(self.value_entries.push(ValueEntry::new_handle(cell))),
                Cell::Delegate(cell) => {
                    Ok(self.delegate_entries.push(DelegateEntry::new_handle(cell)))
                }
            },
        }
    }

    pub fn remove_delegation_cell(table: &HTable, key: &String) {
        // TODO: update modification time.
        match Table::find_delegation_cell(table, key) {
            None => {}
            Some(entry) => {
                if !entry
                    .as_ref()
                    .borrow_mut()
                    .cell
                    .set_modify_timestamp()
                    .is_ok()
                {
                    return;
                }
                entry.as_ref().borrow_mut().cell.inner_cell = None
            }
        }
    }

    pub fn remove_value_cell(table: &HTable, key: &String) {
        // TODO: update modification time.
        match Table::find_value_cell(table, key) {
            None => {}
            Some(entry) => {
                if !entry
                    .as_ref()
                    .borrow_mut()
                    .cell
                    .set_modify_timestamp()
                    .is_ok()
                {
                    return;
                }
                entry.as_ref().borrow_mut().cell.inner_cell = None;
            }
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

        if !cell
            .name_space_or_value()
            .is_some_and(|v| v.starts_with(&self.name_space))
        {
            return Err(TableOpError::NamespaceError);
        }

        if self
            .value_entries
            .iter()
            .any(|table_entry| table_entry.borrow().cell.contains_prefix_from_cell(cell))
        {
            return Err(TableOpError::CellAddressIsPrefix);
        }

        if self
            .value_entries
            .iter()
            .any(|table_entry| table_entry.borrow().cell.is_prefix_of_cell(cell))
        {
            return Err(TableOpError::CellAddressContainsPrefix);
        }

        if self
            .delegate_entries
            .iter()
            .any(|table_entry| table_entry.borrow().cell.contains_prefix_from_cell(cell))
        {
            return Err(TableOpError::CellAddressIsPrefix);
        }

        if self
            .delegate_entries
            .iter()
            .any(|table_entry| table_entry.borrow().cell.is_prefix_of_cell(cell))
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
            if e.borrow().cell.inner_cell.is_some() {
                cur += 1;
            }
        });

        self.delegate_entries.iter().for_each(|e| {
            if let Some(val) = e.borrow().cell.allowance() {
                cur += val;
            }
        });

        if cur + allowance > self.allowance {
            Err(TableOpError::NotEnoughAllowence(self.allowance, cur))
        } else {
            Ok(())
        }
    }

    pub fn find_delegation_cell(table: &Rc<RefCell<Self>>, key: &String) -> Option<HDelegateEntry> {
        for entry in &table.borrow().delegate_entries {
            if entry.borrow().cell.equals_prefix(key) {
                return Some(entry.clone());
            }

            if entry.borrow().cell.is_prefix_of(key) {
                if let Some(next_table) = &entry.borrow().cell.table {
                    return Table::find_delegation_cell(&next_table, key);
                }
            }
        }

        None
    }

    pub fn find_value_cell(table: &Rc<RefCell<Self>>, key: &String) -> Option<HValueEntry> {
        if let Some(val) = table
            .borrow()
            .value_entries
            .iter()
            .find(|e| e.borrow().cell.equals_prefix(key))
        {
            return Some(val.clone());
        }

        if let Some(del_entry) = table
            .borrow()
            .delegate_entries
            .iter()
            .find(|e| e.borrow().cell.is_prefix_of(key))
        {
            match &del_entry.borrow().cell.table {
                Some(new_table) => return Self::find_value_cell(new_table, key),
                None => {
                    return None;
                }
            }
        }
        None
    }
}

impl ValueEntry {
    pub fn new_handle(cell: ValueCell) -> HValueEntry {
        Rc::new(RefCell::new(Self { cell: cell }))
    }
}

impl DelegateEntry {
    pub fn new_handle(cell: DelegateCell) -> HDelegateEntry {
        Rc::new(RefCell::new(Self { cell: cell }))
    }
}

impl TableMeta {
    pub fn to_merkle_hash(&self) -> Option<MerkleHash> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_delegation_rule() {
        let mut entries = Table::new(100, "home/".to_owned());
        let home_cell = Cell::test_new_delegate_cell(String::from("home/home"), 1);
        let cell1 = Cell::test_new_value_cell(String::from("home/cell1"));

        assert!(entries
            .check_cell_valid(&Cell::test_new_value_cell(String::from("ho")))
            .is_err_and(|err| { err == TableOpError::NamespaceError }));
        assert!(entries.add_entry(home_cell).is_ok());
        assert!(entries.check_cell_valid(&cell1).is_ok());

        assert!(entries.add_entry(cell1.clone()).is_ok());
        assert!(entries.check_cell_valid(&cell1).is_err_and(|err| {
            err == TableOpError::CellAddressIsPrefix
                || err == TableOpError::CellAddressContainsPrefix
        }));

        let cell2 = Cell::test_new_value_cell(String::from("home/"));
        let cell3 = Cell::test_new_value_cell(String::from("home/1/2"));

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

        let home_cell = Cell::test_new_delegate_cell(String::from("home/"), 1);
        assert!(table.add_entry(home_cell).is_ok());

        assert!(table
            .contains_enough_allowance(1)
            .is_err_and(|err| { err == TableOpError::NotEnoughAllowence(1, 1) }));
    }
}
