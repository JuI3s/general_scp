use super::{
    ca_type::{PublicKey, Signature},
    cell::Cell,
};

pub struct TableEntry<'a> {
    // opaque lookup_key<>
    lookup_key: &'a str,
    cell: Cell<'a>,
}

pub struct Table<'a> {
    table_entries: Vec<TableEntry<'a>>,
}

pub struct RootEntry<'a> {
    namespace_root_key: PublicKey,
    application_identifier: &'a str,
    listing_sig: Signature,
    allowance: u32,
}

pub struct RootListing<'a> {
    root_entries: Vec<RootEntry<'a>>,
}
