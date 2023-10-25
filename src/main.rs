use rust_example::{application::work_queue::WorkQueue, overlay::peer::Peer};

fn main() {
    let mut work_queue = WorkQueue::new();
    let mut peer = Peer::new();
    peer.incr_one();
    peer.add_to_queue(&mut work_queue);
    work_queue.execute_task();
}
