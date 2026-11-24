//! A one-shot acknowledgement from a page, numbered per show (spec 004 FR-021, FR-022): the backend
//! arms it, emits the state the page must render together with the number, and waits (bounded) for
//! the page to answer with that number before it shows the window. An answer that arrives late from
//! an earlier show carries an older number and is ignored, so it cannot release a newer show before
//! that show's reset state is painted.

use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Debug, Default)]
pub struct AckGate {
    seq: u64,
    waiting: Option<Sender<()>>,
}

impl AckGate {
    pub const fn new() -> Self {
        Self {
            seq: 0,
            waiting: None,
        }
    }

    /// Starts a new wait. Returns the number the page must send back, and the receiver to wait on.
    pub fn arm(&mut self) -> (u64, Receiver<()>) {
        let (sender, receiver) = channel();
        self.seq += 1;
        self.waiting = Some(sender);
        (self.seq, receiver)
    }

    /// The page answered for `seq`. Only the current wait is released; a stale answer is dropped.
    /// Returns whether the answer released the wait.
    pub fn fire(&mut self, seq: u64) -> bool {
        if seq != self.seq {
            return false;
        }
        match self.waiting.take() {
            Some(sender) => sender.send(()).is_ok(),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn the_matching_answer_releases_the_wait() {
        let mut gate = AckGate::new();
        let (seq, receiver) = gate.arm();
        assert!(gate.fire(seq));
        assert!(receiver.recv_timeout(Duration::from_millis(10)).is_ok());
    }

    #[test]
    fn a_late_answer_from_an_earlier_show_does_not_release_the_next_one() {
        // Show #1 timed out; show #2 is armed; then #1's answer arrives (code review, FR-022).
        let mut gate = AckGate::new();
        let (first, _timed_out) = gate.arm();
        let (second, receiver) = gate.arm();
        assert!(!gate.fire(first));
        assert!(receiver.recv_timeout(Duration::from_millis(10)).is_err());
        assert!(gate.fire(second));
        assert!(receiver.recv_timeout(Duration::from_millis(10)).is_ok());
    }

    #[test]
    fn a_second_answer_for_the_same_show_is_ignored() {
        let mut gate = AckGate::new();
        let (seq, _receiver) = gate.arm();
        assert!(gate.fire(seq));
        assert!(!gate.fire(seq));
    }
}
