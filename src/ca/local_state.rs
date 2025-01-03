use crate::ca::crypto::PrivateKey;
use crate::ca::operation::SetRootOperation;
use crate::ca::root::RootEntry;
use crate::ca::state::{CAState, CAStateOpError};

use super::operation::SCPOperation;
use super::state::CAStateOpResult;

pub struct LocalCAState {
    pub private_key: PrivateKey,
    pub state: CAState,
}

impl LocalCAState {
    pub fn init_from_toml(toml_path: &str) -> Self {
        todo!()
    }

    pub fn init_state_from_pkcs8_pem(private_key_path: &str) -> Self {
        let private_key = PrivateKey::from_pkcs8_pem(private_key_path);
        Self {
            private_key,
            state: Default::default(),
        }
    }

    pub fn create_name_space(&self, name_space: &str) -> CAStateOpResult<SCPOperation> {
        if self.state.root_listing.0.contains_key(name_space) {
            Err(CAStateOpError::AlreadyExists)
        } else {
            let entry = RootEntry::new(&self.private_key, name_space.to_owned());

            Ok(SCPOperation::SetRoot(SetRootOperation {
                entry,
                remove: false,
            }))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ca::crypto::TEST_OPENSSL_PRIVATE_KEY;

    use super::*;

    #[test]
    fn test_create_name_space() {
        let mut local_state = LocalCAState::init_state_from_pkcs8_pem(TEST_OPENSSL_PRIVATE_KEY);

        let operation = local_state.create_name_space("namespace1").unwrap();
        let entry = match &operation {
            SCPOperation::SetRoot(set_root_operation) => set_root_operation.entry.clone(),
            _ => panic!("not reached"),
        };

        assert!(local_state.state.on_scp_operation(operation).is_ok());

        assert_eq!(local_state.state.root_listing.0.len(), 1);
        let added_entry = local_state.state.root_listing.0.get("namespace1").unwrap();

        assert_eq!(
            added_entry.application_identifier,
            entry.application_identifier
        );
        assert_eq!(added_entry.allowance, entry.allowance);
    }
}
