use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    marker::PhantomData,
    rc::Rc,
    sync::Arc,
};

use crate::{
    application::work_queue::WorkScheduler, herder::{self, herder::HerderDriver}, overlay_impl::in_memory_global::InMemoryGlobalState, scp::{
        ballot_protocol::BallotProtocolState,
        envelope::{SCPEnvelope, SCPEnvelopeController},
        local_node::{self, LocalNodeInfo},
        nomination_protocol::{NominationProtocol, NominationProtocolState, NominationValue},
        scp_driver::{SCPDriver, SlotDriver},
        scp_driver_builder::SlotDriverBuilder,
        slot::{self, Slot, SlotIndex},
        statement::{SCPStatement, SCPStatementNominate},
    }
};

use super::{
    conn::{PeerConn, PeerConnBuilder},
    message::{HelloEnvelope, MessageController, SCPMessage},
    peer::{PeerID, SCPPeerState},
};

pub struct PeerNode<N, H, C, CB>
where
    N: NominationValue,
    H: HerderDriver<N>,
    C: PeerConn<N>,
    CB: PeerConnBuilder<N, C>,
{
    pub peer_idx: PeerID,
    pub message_controller: Rc<RefCell<MessageController<N>>>,
    pub peer_conns: BTreeMap<PeerID, C>,
    pub slots: BTreeMap<SlotIndex, Arc<SlotDriver<N, H>>>,
    nomination_protocol_states: BTreeMap<SlotIndex, NominationProtocolState<N>>,
    ballot_protocol_states: BTreeMap<SlotIndex, BallotProtocolState<N>>,

    conn_builder: CB,
    scp_envelope_controller: SCPEnvelopeController<N>,
    herder: Rc<RefCell<H>>,

    work_scheduler: Rc<RefCell<WorkScheduler>>,
    local_node_info: Rc<RefCell<LocalNodeInfo<N>>>,
}

impl<N, H, C, CB> Debug for PeerNode<N, H, C, CB>
where
    N: NominationValue,
    H: HerderDriver<N>,
    C: PeerConn<N>,
    CB: PeerConnBuilder<N, C>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PeerNode")
            .field("peer_idx", &self.peer_idx)
            .finish()
    }
}

impl<N, H, C, CB> PeerNode<N, H, C, CB>
where
    N: NominationValue,
    H: HerderDriver<N> + 'static,
    C: PeerConn<N> + Debug,
    CB: PeerConnBuilder<N, C>,
{
    pub fn new(
        peer_idx: PeerID,
        herder: H,
        conn_builder: CB,
        global_state: &Rc<RefCell<InMemoryGlobalState<N>>>,
        local_node_info: LocalNodeInfo<N>,
        work_scheduler: Rc<RefCell<WorkScheduler>>,
    ) -> Self {
        let msg_queue = MessageController::new();
        global_state
            .borrow_mut()
            .peer_msg_queues
            .insert(peer_idx.to_string(), msg_queue.clone());

        Self {
            peer_idx,
            message_controller: msg_queue,
            herder: Rc::new(RefCell::new(herder)),
            conn_builder,
            peer_conns: BTreeMap::new(),
            scp_envelope_controller: SCPEnvelopeController::new(),
            slots: Default::default(),
            work_scheduler,
            local_node_info: Rc::new(RefCell::new(local_node_info)),
            nomination_protocol_states: Default::default(),
            ballot_protocol_states: Default::default(),
        }
    }

    pub fn send_message(&mut self, peer_id: &PeerID, msg: &SCPMessage<N>) {
        let mut peer_conn = self.peer_conns.get_mut(peer_id);
        if peer_conn.is_none() {
            peer_conn = match msg {
                SCPMessage::SCP(_) => None,
                SCPMessage::Hello(_) => Some(self.add_connection(peer_id)),
            };
        }

        if let Some(peer_conn) = peer_conn {
            peer_conn.send_message(msg);
        }
    }

    pub fn send_broadcast_message(&mut self, msg: &SCPMessage<N>) {
        for peer_conn in self.peer_conns.values_mut() {
            peer_conn.send_message(&msg);
        }
    }

    pub fn slot_nominate(&mut self, slot_idx: SlotIndex) {
        let slot = self.get_or_create_slot_and_states(slot_idx);

        let env_id = slot.nominate(
            self.nomination_protocol_states.get_mut(&slot_idx).unwrap(),
            self.ballot_protocol_states.get_mut(&slot_idx).unwrap(),
            Arc::new(N::default()),
            &Default::default(),
            &mut self.scp_envelope_controller,
        );

        if let Some(env_id) = env_id {
            let scp_env = self
                .scp_envelope_controller
                .get_envelope(&env_id)
                .unwrap()
                .clone();

            let scp_msg = SCPMessage::SCP(scp_env);

            self.send_broadcast_message(&scp_msg);
        }
    }

    pub fn send_hello(&mut self, peer_id: &PeerID) {
        let hello_env = HelloEnvelope {
            id: self.peer_idx.clone(),
        };
        let msg = SCPMessage::Hello(hello_env);
        self.send_message(peer_id, &msg);
    }

    pub fn add_connection(&mut self, peer_id: &PeerID) -> &mut C {
        self.peer_conns
            .entry(peer_id.to_string())
            .or_insert(self.conn_builder.build(peer_id))
    }

    pub fn process_one_message(&mut self) -> bool {
        let msg_option = self.message_controller.borrow_mut().pop();

        match msg_option {
            Some(msg) => {
                match msg {
                    SCPMessage::SCP(scp_env) => self.on_scp_env(scp_env),
                    SCPMessage::Hello(hello_env) => self.on_hello_env(hello_env),
                }
                true
            }
            None => false,
        }
    }

    fn on_hello_env(&mut self, hello_env: HelloEnvelope) {
        let peer_id = &hello_env.id;
        if !self.peer_conns.contains_key(peer_id) {
            self.add_connection(peer_id);
            self.peer_conns
                .get_mut(peer_id)
                .unwrap()
                .set_state(super::peer::SCPPeerConnState::Connected);

            self.send_hello(&hello_env.id);
        } else {
            self.peer_conns
                .get_mut(peer_id)
                .unwrap()
                .set_state(super::peer::SCPPeerConnState::Connected);
        }
    }

    fn build_slot(&self, slot_idx: SlotIndex) -> Arc<SlotDriver<N, H>> {
        SlotDriverBuilder::<N, H>::new()
            .slot_index(slot_idx)
            .herder_driver(self.herder.clone())
            .timer(self.work_scheduler.clone())
            .local_node(self.local_node_info.clone())
            .nomination_protocol_state(NominationProtocolState::new(self.peer_idx.clone()))
            .build_handle()
            .unwrap()
    }

    fn get_or_create_slot_and_states(&mut self, slot_idx: SlotIndex) -> Arc<SlotDriver<N, H>> {
        if !self.slots.contains_key(&slot_idx) {
            self.slots
                .insert(slot_idx.clone(), self.build_slot(slot_idx));

            self.nomination_protocol_states.insert(
                slot_idx.clone(),
                NominationProtocolState::new(self.peer_idx.clone()),
            );

            self.ballot_protocol_states
                .insert(slot_idx.clone(), Default::default());
        }

        self.slots.get(&slot_idx).unwrap().clone()
    }

    fn on_scp_env(&mut self, scp_env: SCPEnvelope<N>) {
        let slot_idx: u64 = scp_env.slot_index.clone();
        let env_id = self.scp_envelope_controller.add_envelope(scp_env);

        let slot = self.get_or_create_slot_and_states(slot_idx);
        slot.recv_scp_envelvope(
            self.nomination_protocol_states.get_mut(&slot_idx).unwrap(),
            self.ballot_protocol_states.get_mut(&slot_idx).unwrap(),
            &env_id,
            &mut self.scp_envelope_controller,
        );
    }

    pub fn process_all_messages(&mut self) -> usize {
        let mut msg_processed = 0;
        while self.process_one_message() {
            msg_processed += 1;
        }
        msg_processed
    }
}
