use std::time::UNIX_EPOCH;

use crate::scp::envelope::SCPEnvelopeID;

pub fn pretty_print_scp_env_id(env: &SCPEnvelopeID) -> u128 {
    env.duration_since(UNIX_EPOCH).unwrap().as_micros() % 1000000000
}
