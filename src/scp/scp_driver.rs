use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, Weak},
};

use weak_self_derive::WeakSelf;

use crate::{
    application::work_queue::{ClockEvent, HWorkQueue},
    herder::herder::Herder,
    utils::weak_self::WeakSelf, scp::ballot_protocol::SCPPhase,
};

use super::{
    ballot_protocol::{BallotProtocol, BallotProtocolState, SCPBallot, SCPStatement},
    local_node::{LocalNode, HLocalNode},
    nomination_protocol::NominationValue,
    slot::{HSlot, Slot, SlotIndex},
};

pub type HSCPDriver = Arc<Mutex<dyn SCPDriver>>;

// #[derive(WeakSelf)]
pub struct SlotDriver {
    pub slot_index: u64,
    pub local_node: HLocalNode,
    pub timer: HSlotTimer,
}

pub enum ValidationLevel {
    InvalidValue,
    MaybeValidValue,
    FullyValidatedValue,
    VoteToNominate,
}

pub type HSCPEnvelope = Arc<Mutex<NominationValue>>;
pub struct SCPEnvelope {}

pub trait SCPDriver {
    fn validate_value(
        slot_index: u64,
        value: &NominationValue,
        nomination: bool,
    ) -> ValidationLevel;

    // Inform about events happening within the consensus algorithm.

    // ``nominating_value`` is called every time the local instance nominates a new value.
    fn nominating_value(self: &Arc<Self>, value: &NominationValue);
    // `value_externalized` is called at most once per slot when the slot externalize its value.
    fn value_externalized(slot_index: u64, value: &NominationValue);

    // the following methods are used for monitoring of the SCP subsystem most implementation don't really need to do anything with these.

    fn emit_envelope(envelope: &SCPEnvelope);

    fn sign_envelope(envelope: &SCPEnvelope);
}

pub type HSlotTimer = Arc<Mutex<SlotTimer>>;
pub struct SlotTimer {
    work_queue: HWorkQueue,
}

impl SlotTimer {
    pub fn add_task(&mut self, callback: ClockEvent) {
        self.work_queue.lock().unwrap().add_task(callback);
    }

}

impl SlotDriver {
    fn get_prepare_candidates(hint: &SCPStatement) -> BTreeSet<SCPBallot> {
        todo!()
    }
}

// pub trait WeakSelf {
//     fn get_weak_self(&mut self) -> Weak<Mutex<&mut Self>>;
// }

impl SCPDriver for SlotDriver {
    fn nominating_value(self: &Arc<Self>, value: &NominationValue) {}

    fn validate_value(
        slot_index: u64,
        value: &NominationValue,
        nomination: bool,
    ) -> ValidationLevel {
        ValidationLevel::MaybeValidValue
    }

    fn emit_envelope(envelope: &SCPEnvelope) {}

    fn value_externalized(slot_index: u64, value: &NominationValue) {
        todo!()
    }

    fn sign_envelope(envelope: &SCPEnvelope) {
        todo!()
    }


}

impl BallotProtocol for SlotDriver {
    fn externalize(&mut self) {
        todo!()
    }

    fn recv_ballot_envelope(&mut self) {
        todo!()
    }

    fn attempt_accept_prepared(
        &mut self,
        state: &mut BallotProtocolState,
        hint: &SCPStatement,
    ) -> bool {

        if state.phase != SCPPhase::PhasePrepare && state.phase != SCPPhase::PhaseConfirm {
            return false;
        }

        let candidates = SlotDriver::get_prepare_candidates(hint);

        for candidate in candidates {
            
        }
        todo!()
    }

    fn set_accept_prepared(state: &mut BallotProtocolState, prepared: &SCPBallot) -> bool {
        todo!()
    }

    fn attempt_confirm_prepared(state: &mut BallotProtocolState, hint: &SCPStatement) {
        todo!()
    }

    fn set_confirm_prepared(
        state: &mut BallotProtocolState,
        newC: &SCPBallot,
        newH: &SCPBallot,
    ) -> bool {
        todo!()
    }

    fn attempt_accept_commit(state: &mut BallotProtocolState, hint: &SCPStatement) -> bool {
        todo!()
    }

    fn set_accept_commit(state: &mut BallotProtocolState, c: &SCPBallot, h: &SCPBallot) -> bool {
        todo!()
    }

    fn attempt_confirm_commit(state: &mut BallotProtocolState, hint: &SCPStatement) -> bool {
        todo!()
    }

    fn set_confirm_commit(
        state: &mut BallotProtocolState,
        acceptCommitLow: &SCPBallot,
        acceptCommitHigh: &SCPBallot,
    ) -> bool {
        todo!()
    }

    fn attemptBump() -> bool {
        todo!()
    }
}
