use crate::application::quorum::QuorumSet;
use crate::herder::herder::HerderDriver;
use crate::scp::statement::SCPStatement;
use std::collections::BTreeSet;
use std::sync::Arc;

use super::local_state::LocalCAState;
use super::operation::{CAOperation, SCPCAOperation};

pub struct CAStateDriver {}

impl HerderDriver<SCPCAOperation> for CAStateDriver {
    fn new() -> Self {
        Self {}
    }

    fn combine_candidates(
        &self,
        candidates: &BTreeSet<Arc<SCPCAOperation>>,
    ) -> Option<SCPCAOperation> {
        // TODO: need to filter out conflicting operations
        Some(SCPCAOperation(Vec::from_iter(
            candidates
                .iter()
                .map(|val| val.0.iter())
                .flatten()
                .map(|val| val.to_owned()),
        )))
    }

    fn extract_valid_value(&self, value: &SCPCAOperation) -> Option<SCPCAOperation> {
        None
    }
}
