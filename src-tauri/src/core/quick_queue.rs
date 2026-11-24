//! The lazy quick panel's queue (spec 009 T122, T134): the panel is built on its first open, and a
//! show or a clipboard prefill asked for before its page is ready waits here and is replayed once
//! the page reports ready. Pure state, so the ordering rules are tested without windows.

/// A screen capture reaching the panel (`quick-screen`, contracts/screen-result-panel.md).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct QuickScreenEvent {
    /// `reading`, `text`, `no_text`, `read_failed` or `reset`.
    pub phase: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_lang: Option<String>,
    /// The number the page sends back once the reading state is painted (`AckGate`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
}

impl QuickScreenEvent {
    /// A new capture: the panel resets to "Reading the text…". The source is always `auto`: text
    /// read from the screen is in an unknown language, whatever the user's default source is
    /// (FR-007; the removed `translate_screen_text` did the same). `seq` numbers the show.
    pub fn reading(target_lang: String, seq: u64) -> Self {
        Self {
            phase: "reading",
            text: None,
            source_lang: Some("auto".into()),
            target_lang: Some(target_lang),
            seq: Some(seq),
        }
    }

    pub fn text(text: String) -> Self {
        Self {
            phase: "text",
            text: Some(text),
            source_lang: None,
            target_lang: None,
            seq: None,
        }
    }

    /// The panel was opened normally after a screen capture: back to a fresh typed session.
    pub fn reset() -> Self {
        Self::bare("reset")
    }

    pub fn no_text() -> Self {
        Self::bare("no_text")
    }

    pub fn read_failed() -> Self {
        Self::bare("read_failed")
    }

    fn bare(phase: &'static str) -> Self {
        Self {
            phase,
            text: None,
            source_lang: None,
            target_lang: None,
            seq: None,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct QuickQueue {
    ready: bool,
    pending_show: bool,
    pending_prefill: Option<String>,
    /// At most a `reading` event and the capture's last event, in that order.
    pending_screen: Vec<QuickScreenEvent>,
}

/// What the window layer must do after `page_ready`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replay {
    pub present: bool,
    pub prefill: Option<String>,
    /// Sent before the show, so the panel never paints the previous capture or typed text.
    pub screen: Vec<QuickScreenEvent>,
}

impl QuickQueue {
    pub const fn new() -> Self {
        Self {
            ready: false,
            pending_show: false,
            pending_prefill: None,
            pending_screen: Vec::new(),
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

    /// A screen capture event. `Some`: send it now; `None`: it waits for the page (a newer event
    /// replaces an older waiting one). A `reading` event replaces typed content, so a waiting
    /// clipboard prefill is dropped.
    pub fn screen_event(&mut self, event: QuickScreenEvent) -> Option<QuickScreenEvent> {
        if event.phase == "reading" {
            self.pending_prefill = None;
        }
        if self.ready {
            Some(event)
        } else {
            // A capture's reading event is kept with its last event, so a page that loads late
            // still resets (automatic source, reading state) before it gets the text. A new
            // `reading` or `reset` starts over; a later last event replaces an earlier one.
            if matches!(event.phase, "reading" | "reset") {
                self.pending_screen.clear();
            } else {
                self.pending_screen
                    .retain(|waiting| matches!(waiting.phase, "reading" | "reset"));
            }
            self.pending_screen.push(event);
            None
        }
    }

    /// The page has its listeners: hand back what waited for it.
    pub fn page_ready(&mut self) -> Replay {
        self.ready = true;
        Replay {
            present: std::mem::take(&mut self.pending_show),
            prefill: self.pending_prefill.take(),
            screen: std::mem::take(&mut self.pending_screen),
        }
    }

    /// The user closed the panel (Esc, ×, the shortcut again): a queued show or prefill is dropped.
    pub fn hide(&mut self) {
        self.pending_show = false;
        self.pending_prefill = None;
        self.pending_screen.clear();
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

/// What to do with a capture's last event (text, no text, read failed) once reading ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingEnd {
    /// Send it to the panel.
    Deliver,
    /// The user closed the panel while it was reading: nothing more reaches the page, so no
    /// recognised text is translated after the user dismissed it (FR-006, FR-023).
    Drop,
    /// The panel lost the focus while reading and still has not got it back (a system prompt such
    /// as macOS's private-window-picker question, or a click elsewhere): send the event and give the
    /// panel the focus again, so a later click outside closes it as usual (code review: it used to
    /// stay on screen without focus and could not be dismissed by clicking away).
    DeliverAndFocus,
}

/// The reading phase of one capture in the panel, from its `reading` event to its last event. A
/// blur is not acted on while reading (a system prompt may take the focus for a moment), but it is
/// remembered, and a close during reading abandons the capture.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReadingTracker {
    reading: bool,
    blurred: bool,
    abandoned: bool,
}

impl ReadingTracker {
    pub const fn new() -> Self {
        Self {
            reading: false,
            blurred: false,
            abandoned: false,
        }
    }

    /// A new capture started reading.
    pub fn start(&mut self) {
        *self = Self {
            reading: true,
            ..Self::new()
        };
    }

    pub fn is_reading(&self) -> bool {
        self.reading
    }

    /// The panel lost the focus. `true`: ignore it for now (reading), it is remembered.
    pub fn blur(&mut self) -> bool {
        if self.reading {
            self.blurred = true;
        }
        self.reading
    }

    /// The panel was closed. During reading the capture is abandoned.
    pub fn hide(&mut self) {
        if self.reading {
            self.reading = false;
            self.abandoned = true;
        }
    }

    /// Reading ended; `panel_focused` is whether the panel has the focus now.
    pub fn finish(&mut self, panel_focused: bool) -> ReadingEnd {
        let was = *self;
        *self = Self::new();
        if was.abandoned {
            ReadingEnd::Drop
        } else if was.reading && was.blurred && !panel_focused {
            ReadingEnd::DeliverAndFocus
        } else {
            ReadingEnd::Deliver
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reading_always_asks_for_automatic_source_detection() {
        // Code review: the panel translated screen text from the user's default source language.
        let event = QuickScreenEvent::reading("vi".into(), 7);
        assert_eq!(event.source_lang.as_deref(), Some("auto"));
        assert_eq!(event.target_lang.as_deref(), Some("vi"));
        assert_eq!(event.seq, Some(7));
    }

    #[test]
    fn a_close_during_reading_drops_the_capture() {
        let mut tracker = ReadingTracker::new();
        tracker.start();
        tracker.hide();
        assert_eq!(tracker.finish(false), ReadingEnd::Drop);
        assert!(!tracker.is_reading());
    }

    #[test]
    fn a_blur_during_reading_is_kept_and_gives_the_focus_back_when_reading_ends() {
        let mut tracker = ReadingTracker::new();
        tracker.start();
        assert!(tracker.blur(), "ignored while reading");
        assert_eq!(tracker.finish(false), ReadingEnd::DeliverAndFocus);
    }

    #[test]
    fn a_blur_during_reading_is_forgotten_when_the_focus_came_back() {
        let mut tracker = ReadingTracker::new();
        tracker.start();
        tracker.blur();
        assert_eq!(tracker.finish(true), ReadingEnd::Deliver);
    }

    #[test]
    fn without_reading_blur_and_close_are_not_held() {
        let mut tracker = ReadingTracker::new();
        assert!(!tracker.blur());
        tracker.hide();
        assert_eq!(tracker.finish(false), ReadingEnd::Deliver);
    }

    #[test]
    fn a_new_capture_clears_the_previous_one() {
        let mut tracker = ReadingTracker::new();
        tracker.start();
        tracker.hide();
        tracker.start();
        assert_eq!(tracker.finish(true), ReadingEnd::Deliver);
    }

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
                prefill: None,
                screen: Vec::new()
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

    #[test]
    fn a_screen_event_before_ready_waits_and_comes_before_the_show() {
        let mut queue = QuickQueue::new();
        queue.created();
        queue.request_show();
        let event = QuickScreenEvent::reading("vi".into(), 1);
        assert_eq!(queue.screen_event(event.clone()), None);
        let replay = queue.page_ready();
        assert_eq!(replay.screen, vec![event]);
        assert!(
            replay.present,
            "the show follows the event in the same replay"
        );
        assert_eq!(
            queue.screen_event(QuickScreenEvent::no_text()),
            Some(QuickScreenEvent::no_text()),
            "once ready, an event is immediate"
        );
    }

    #[test]
    fn a_waiting_reading_event_is_kept_before_the_text_of_its_capture() {
        // A page that loads after recognition still gets the reset (automatic source) first.
        let mut queue = QuickQueue::new();
        queue.created();
        let reading = QuickScreenEvent::reading("vi".into(), 1);
        queue.screen_event(reading.clone());
        queue.screen_event(QuickScreenEvent::no_text());
        queue.screen_event(QuickScreenEvent::text("hello".into()));
        assert_eq!(
            queue.page_ready().screen,
            vec![reading, QuickScreenEvent::text("hello".into())]
        );
    }

    #[test]
    fn a_new_capture_replaces_everything_still_waiting() {
        let mut queue = QuickQueue::new();
        queue.created();
        queue.screen_event(QuickScreenEvent::reading("vi".into(), 1));
        queue.screen_event(QuickScreenEvent::text("old".into()));
        let second = QuickScreenEvent::reading("vi".into(), 2);
        queue.screen_event(second.clone());
        assert_eq!(queue.page_ready().screen, vec![second]);
    }

    #[test]
    fn a_capture_drops_a_waiting_clipboard_prefill() {
        let mut queue = QuickQueue::new();
        queue.created();
        queue.prefill("typed".into());
        queue.screen_event(QuickScreenEvent::reading("vi".into(), 1));
        let replay = queue.page_ready();
        assert_eq!(replay.prefill, None);
        assert!(!replay.screen.is_empty());
    }
}
