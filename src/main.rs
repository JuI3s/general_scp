use rust_example::{
    application::{application::Application, config::Config, work_queue::WorkQueue},
    overlay::peer::Peer,
};

fn main() {
    let cfg = Config::new_config();
    let mut app = Application::new(&cfg);

    app.start();

    // let mut work_queue = WorkQueue::new();
    // let mut peer = Peer::new();
    // peer.incr_one();
    // peer.add_to_queue(&mut work_queue);
    // work_queue.execute_task();
}
