use rust_example::{application::{work_queue::WorkQueue, config::Config, application::Application}, overlay::peer::Peer};

fn main() {
    let cfg = Config::new_config();
    let app = Application::new(&cfg);

    // let mut work_queue = WorkQueue::new();
    // let mut peer = Peer::new();
    // peer.incr_one();
    // peer.add_to_queue(&mut work_queue);
    // work_queue.execute_task();
}
