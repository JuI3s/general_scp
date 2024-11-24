use std::marker::PhantomData;

use crate::scp::nomination_protocol::NominationValue;

pub struct TCPPeer<N>
where
    N: NominationValue,
{
    phantom: PhantomData<N>, // snip
}
