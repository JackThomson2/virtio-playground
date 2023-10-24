pub trait PollableQueue {
    fn wait_for_event(&self);

    fn submit_event(&self);
}
