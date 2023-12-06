use super::cell::Cell;

pub type PublicKey = [u8; 64];
type Timestamp = u64;
struct Signature {
    pk: PublicKey,
}

pub enum CellType {
    Value,
    Delegate,
}

pub struct Record<'a> {
    title: &'a str,
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
    fn prefix() {
        let a = Record { title: "/sun/a" };
        let b = Record { title: "/sun/b" };

        assert!(!a.title.starts_with(b.title));
    }
}
