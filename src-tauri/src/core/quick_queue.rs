//! The lazy quick panel's queue (spec 009 T122, T134): the panel is built on its first open, and a
//! show or a clipboard prefill asked for before its page is ready waits here and is replayed once
//! the page reports ready. Pure state, so the ordering rules are tested without windows.

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct QuickQueue {
    ready: bool,
    pending_show: bool,
    pending_prefill: Option<String>,
}

/// What the window layer must do after `page_ready`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replay {
    pub present: bool,
    pub prefill: Option<String>,
}

impl QuickQueue {
    pub const fn new() -> Self {
        Self {
            ready: false,
            pending_show: false,
            pending_prefill: None,
        }
    }

    /// A new panel webview was built: nothing it receives counts until its page is ready.
    pub fn created(&mut self) {
        self.ready = false;
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn show_pending(&self) -> bool {
        self.pending_show
    }

    /// The user asked for the panel. `true`: present it now; `false`: it waits for the page.
    pub fn request_show(&mut self) -> bool {
        if self.ready {
            true
        } else {
            self.pending_show = true;
            false
        }
    }

    /// A clipboard prefill for the panel. `Some`: send it now; `None`: it waits for the page.
    pub fn prefill(&mut self, text: String) -> Option<String> {
        if self.ready {
            Some(text)
        } else {
            self.pending_prefill = Some(text);
            None
        }
    }

    /// The page has its listeners: hand back what waited for it.
    pub fn page_ready(&mut self) -> Replay {
        self.ready = true;
        Replay {
            present: std::mem::take(&mut self.pending_show),
            prefill: self.pending_prefill.take(),
        }
    }

    /// The user closed the panel (Esc, ×, the shortcut again): a queued show or prefill is dropped.
    pub fn hide(&mut self) {
        self.pending_show = false;
        self.pending_prefill = None;
    }

    /// The panel lost focus. Only a panel on screen closes; a panel still loading keeps its queue,
    /// since a blur while it is hidden is not the user leaving it. `true`: hide it.
    pub fn blur(&mut self, visible: bool) -> bool {
        if !visible {
            return false;
        }
        self.hide();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_show_before_ready_waits_and_is_replayed() {
        let mut queue = QuickQueue::new();
        queue.created();
        assert!(!queue.request_show());
        assert!(queue.show_pending());
        let replay = queue.page_ready();
        assert!(replay.present);
        assert!(!queue.show_pending());
        assert!(queue.request_show(), "once ready, a show is immediate");
    }

    #[test]
    fn blur_before_ready_keeps_the_queued_show() {
        // The defect found on macOS VM 1: the hidden, loading panel lost focus and dropped its show.
        let mut queue = QuickQueue::new();
        queue.created();
        queue.request_show();
        assert!(!queue.blur(false));
        assert!(queue.page_ready().present);
    }

    #[test]
    fn blur_on_screen_hides_and_clears() {
        let mut queue = QuickQueue::new();
        queue.page_ready();
        assert!(queue.blur(true));
        assert!(!queue.show_pending());
    }

    #[test]
    fn a_prefill_before_ready_is_kept_until_ready() {
        let mut queue = QuickQueue::new();
        queue.created();
        assert_eq!(queue.prefill("hello".into()), None);
        assert_eq!(queue.prefill("newer".into()), None);
        let replay = queue.page_ready();
        assert_eq!(replay.prefill.as_deref(), Some("newer"));
        assert_eq!(queue.prefill("now".into()).as_deref(), Some("now"));
    }

    #[test]
    fn closing_before_ready_drops_the_queue() {
        let mut queue = QuickQueue::new();
        queue.created();
        queue.request_show();
        queue.prefill("text".into());
        queue.hide();
        assert_eq!(
            queue.page_ready(),
            Replay {
                present: false,
                prefill: None
            }
        );
    }

    #[test]
    fn a_rebuilt_panel_waits_again() {
        let mut queue = QuickQueue::new();
        queue.page_ready();
        queue.created();
        assert!(!queue.request_show());
    }
}
