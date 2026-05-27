use dioxus::prelude::*;

use crate::model::FixMessage;
use crate::types::ViewMode;
use crate::components::lifecycle::LifecycleChain;
use crate::validator::ValidationReport;

/// Per-tab parser session. All state is held in `Signal`s rooted at `ScopeId::ROOT`
/// so the signals survive for the lifetime of the application regardless of which
/// tab is currently rendered.
#[derive(Clone, Copy, PartialEq)]
pub struct Tab {
    pub id:              u64,
    pub label:           Signal<String>,
    pub input:           Signal<String>,
    pub messages:        Signal<Vec<FixMessage>>,
    pub selected_idx:    Signal<Option<usize>>,
    pub skip_heartbeats: Signal<bool>,
    pub skip_common:     Signal<bool>,
    pub parse_stats:     Signal<Option<(usize, u64)>>,
    pub file_name:       Signal<Option<String>>,
    /// Absolute path of the currently-loaded file (None for paste / sample
    /// loads). Used by the Reload button + the optional auto-watch poller
    /// to re-read the same file when it changes on disk.
    pub file_path:       Signal<Option<String>>,
    /// Last-seen mtime (unix-millis) of `file_path`. Used by the auto-watch
    /// poller to detect changes without re-parsing on every tick.
    pub file_mtime_ms:   Signal<u64>,
    /// Per-tab toggle: when true, a 1-second poller reloads `file_path`
    /// whenever its mtime increases. Defaults off — explicit Reload button
    /// is the default flow.
    pub file_auto_watch: Signal<bool>,
    /// Byte offset into `file_path` consumed so far. When auto-watch fires
    /// and the file has grown, the loader reads only `[tail_offset..]` and
    /// APPENDS to `messages` instead of re-parsing the whole file.
    pub file_tail_offset: Signal<u64>,
    /// When true, the Timeline auto-scrolls to the bottom after each
    /// successful tail load. Independent of `file_auto_watch` because the
    /// operator may want background ingest without losing scroll position.
    pub file_follow_tail: Signal<bool>,
    pub loaded_files:    Signal<Vec<String>>,
    pub show_file_list:  Signal<bool>,
    pub view_mode:       Signal<ViewMode>,
    pub loading:         Signal<bool>,

    // ── Per-tab UI state (filter inputs, view toggles) ─────────────────────
    // Kept on the Tab so switching tabs preserves what the user was looking at.
    pub f_time:           Signal<String>,
    pub f_time_op:        Signal<String>,
    pub f_sender:         Signal<String>,
    pub f_target:         Signal<String>,
    pub f_msg:            Signal<String>,
    pub f_clord:          Signal<String>,
    pub f_detail:         Signal<String>,
    pub timeline_filters_open: Signal<bool>,
    pub display_limit:    Signal<usize>,
    pub detail_view:      Signal<u8>,   // 0=Table, 1=Raw, 2=JSON
    pub detail_filter:    Signal<String>,
    pub detail_filter_open: Signal<bool>,

    // ── Per-tab Validator state ────────────────────────────────────────────
    // Cached on the Tab so switching tabs never shows stale batch results
    // from another tab. `validator_batch_signature` is the messages-length
    // snapshot the cached reports were computed against — used by the panel
    // to detect "this cache is for a different message set, recompute".
    pub validator_tab_kind:        Signal<u8>,      // 0=Debugger, 1=Batch
    pub validator_raw_input:       Signal<String>,
    pub validator_filter:          Signal<String>,
    pub validator_report:          Signal<Option<ValidationReport>>,
    pub validator_parsed_fields:   Signal<Vec<(u16, String)>>,
    pub validator_batch_reports:   Signal<Vec<(usize, String, ValidationReport)>>,
    pub validator_batch_total:     Signal<usize>,
    pub validator_batch_validating:Signal<bool>,
    pub validator_batch_signature: Signal<usize>,
    /// Monotonically increasing epoch. Bumped by the host (tab_view / app) when
    /// the user navigates away from the Validator (view change, tab switch).
    /// The in-flight chunked validation task checks this between chunks and
    /// aborts when its snapshot no longer matches.
    pub validator_cancel:          Signal<u64>,

    // ── Per-tab Lifecycle (Latency) state ──────────────────────────────────
    // Mirrors the Validator pattern: chains are computed off-thread via rayon
    // so the UI does not freeze for million-message tabs, and a cancel epoch
    // lets the host abort an in-flight compute when the user navigates away.
    pub lifecycle_chains:    Signal<Vec<LifecycleChain>>,
    pub lifecycle_signature: Signal<usize>,
    pub lifecycle_computing: Signal<bool>,
    pub lifecycle_cancel:    Signal<u64>,
    /// Chain-id filter for the Latency view. Lifted to Tab so a "Jump to
    /// latency chain" action from the Timeline can pre-fill it before
    /// switching view_mode = Lifecycle.
    pub lifecycle_filter_id: Signal<String>,
}

impl Tab {
    /// Abort any in-flight validator job for this tab and force a fresh
    /// recompute the next time the Validator view mounts. Called by app /
    /// tab_view on view-mode change or tab switch.
    pub fn cancel_validator(&self) {
        let mut cancel = self.validator_cancel;
        let next = *cancel.peek() + 1;
        cancel.set(next);
        let mut sig = self.validator_batch_signature;
        sig.set(usize::MAX);
        let mut validating = self.validator_batch_validating;
        validating.set(false);
        let mut reports = self.validator_batch_reports;
        reports.set(Vec::new());
    }

    /// Abort any in-flight lifecycle/chain computation for this tab and force
    /// a fresh recompute the next time the Latency view mounts. Called by the
    /// host on view-mode change or tab switch.
    pub fn cancel_lifecycle(&self) {
        let mut cancel = self.lifecycle_cancel;
        let next = *cancel.peek() + 1;
        cancel.set(next);
        let mut sig = self.lifecycle_signature;
        sig.set(usize::MAX);
        let mut computing = self.lifecycle_computing;
        computing.set(false);
        let mut chains = self.lifecycle_chains;
        chains.set(Vec::new());
    }
}

impl Tab {
    pub fn new(id: u64, label: impl Into<String>) -> Self {
        let s = ScopeId::ROOT;
        Self {
            id,
            label:           Signal::new_in_scope(label.into(),       s),
            input:           Signal::new_in_scope(String::new(),      s),
            messages:        Signal::new_in_scope(Vec::new(),         s),
            selected_idx:    Signal::new_in_scope(None,               s),
            skip_heartbeats: Signal::new_in_scope(true,               s),
            skip_common:     Signal::new_in_scope(false,              s),
            parse_stats:     Signal::new_in_scope(None,               s),
            file_name:       Signal::new_in_scope(None,               s),
            file_path:       Signal::new_in_scope(None,               s),
            file_mtime_ms:   Signal::new_in_scope(0u64,               s),
            file_auto_watch: Signal::new_in_scope(false,              s),
            file_tail_offset:Signal::new_in_scope(0u64,               s),
            file_follow_tail:Signal::new_in_scope(false,              s),
            loaded_files:    Signal::new_in_scope(Vec::new(),         s),
            show_file_list:  Signal::new_in_scope(false,              s),
            view_mode:       Signal::new_in_scope(ViewMode::Timeline, s),
            loading:         Signal::new_in_scope(false,              s),

            f_time:                Signal::new_in_scope(String::new(),       s),
            f_time_op:             Signal::new_in_scope("=".to_string(),     s),
            f_sender:              Signal::new_in_scope(String::new(),       s),
            f_target:              Signal::new_in_scope(String::new(),       s),
            f_msg:                 Signal::new_in_scope(String::new(),       s),
            f_clord:               Signal::new_in_scope(String::new(),       s),
            f_detail:              Signal::new_in_scope(String::new(),       s),
            timeline_filters_open: Signal::new_in_scope(true,                s),
            display_limit:         Signal::new_in_scope(1000usize,           s),
            detail_view:           Signal::new_in_scope(0u8,                 s),
            detail_filter:         Signal::new_in_scope(String::new(),       s),
            detail_filter_open:    Signal::new_in_scope(false,               s),

            validator_tab_kind:        Signal::new_in_scope(1u8,           s), // default = Batch
            validator_raw_input:       Signal::new_in_scope(String::new(), s),
            validator_filter:          Signal::new_in_scope(String::new(), s),
            validator_report:          Signal::new_in_scope(None,          s),
            validator_parsed_fields:   Signal::new_in_scope(Vec::new(),    s),
            validator_batch_reports:   Signal::new_in_scope(Vec::new(),    s),
            validator_batch_total:     Signal::new_in_scope(0usize,        s),
            validator_batch_validating:Signal::new_in_scope(false,         s),
            validator_batch_signature: Signal::new_in_scope(usize::MAX,    s),
            validator_cancel:          Signal::new_in_scope(0u64,          s),

            lifecycle_chains:    Signal::new_in_scope(Vec::new(),    s),
            lifecycle_signature: Signal::new_in_scope(usize::MAX,    s),
            lifecycle_computing: Signal::new_in_scope(false,         s),
            lifecycle_cancel:    Signal::new_in_scope(0u64,          s),
            lifecycle_filter_id: Signal::new_in_scope(String::new(), s),
        }
    }
}

/// Build the diff key for a message. Returns `None` for messages that have no
/// stable identifier (heartbeats, logons, etc.) — they are not coloured in
/// compare mode.
pub fn message_key(m: &FixMessage) -> Option<String> {
    if !m.cl_ord_id.is_empty()    { return Some(format!("c:{}",  m.cl_ord_id)); }
    if !m.quote_id.is_empty()     { return Some(format!("q:{}",  m.quote_id)); }
    if !m.quote_req_id.is_empty() { return Some(format!("qr:{}", m.quote_req_id)); }
    None
}

// ── Pure id-management helpers (testable without Dioxus) ──────────────────────
//
// These mirror the tab-management logic that lives inside `app.rs` closures.
// Extracting the pure shape into functions lets the test suite verify all
// invariants (compare-self illegal, active-promotion on close, etc.) without
// spinning up a Dioxus runtime.

/// Apply a "close tab" intent to a `(ids, active, compare)` triple, returning
/// the post-close state. If the close would leave only one tab, the close is
/// rejected (the caller forbids closing the last tab in the UI too).
///
/// Invariants enforced on the return value:
/// - `compare != Some(active)` (never compare a tab with itself)
/// - `active` is always one of the remaining ids
/// - `compare`, when `Some`, is always one of the remaining ids
#[allow(dead_code)]
pub fn close_tab_ids(
    ids:     &[u64],
    active:  u64,
    compare: Option<u64>,
    closing: u64,
) -> (Vec<u64>, u64, Option<u64>) {
    if ids.len() <= 1 {
        return (ids.to_vec(), active, compare);
    }
    let new_ids: Vec<u64> = ids.iter().copied().filter(|&i| i != closing).collect();
    if new_ids.len() == ids.len() {
        // Nothing actually closed (id not found) — return state unchanged.
        return (new_ids, active, compare);
    }
    let mut new_compare = compare.filter(|&c| c != closing);
    let new_active = if active == closing {
        new_ids[0]
    } else {
        active
    };
    if new_compare == Some(new_active) {
        new_compare = None;
    }
    (new_ids, new_active, new_compare)
}

/// Apply a "switch active tab" intent. The product rule is: clicking any tab
/// (even the current compare partner) clears compare and promotes that tab to
/// active. Use this for every active-switch entry point — tab click, ⌘1-9,
/// ⌘Tab cycle — so the behaviour stays centralised.
#[allow(dead_code)]
pub fn switch_active(
    active:     u64,
    compare:    Option<u64>,
    new_active: u64,
) -> (u64, Option<u64>) {
    if new_active == active {
        return (active, compare);
    }
    (new_active, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_tab_removes_id() {
        let (ids, a, c) = close_tab_ids(&[1, 2, 3], 1, None, 2);
        assert_eq!(ids, vec![1, 3]);
        assert_eq!(a, 1);
        assert_eq!(c, None);
    }

    #[test]
    fn close_active_promotes_first_remaining() {
        let (ids, a, c) = close_tab_ids(&[1, 2, 3], 2, None, 2);
        assert_eq!(ids, vec![1, 3]);
        assert_eq!(a, 1);
        assert_eq!(c, None);
    }

    #[test]
    fn close_compare_clears_compare() {
        let (_ids, _a, c) = close_tab_ids(&[1, 2], 1, Some(2), 2);
        assert_eq!(c, None);
    }

    #[test]
    fn close_active_lands_on_compare_clears_compare() {
        // tabs=[1,2,3], active=3, compare=1; close active=3 → promote first=1;
        // active==compare would be self-compare → compare must be cleared.
        let (ids, a, c) = close_tab_ids(&[1, 2, 3], 3, Some(1), 3);
        assert_eq!(ids, vec![1, 2]);
        assert_eq!(a, 1);
        assert_eq!(c, None);
    }

    #[test]
    fn close_last_tab_rejected() {
        let (ids, a, c) = close_tab_ids(&[1], 1, None, 1);
        assert_eq!(ids, vec![1]);
        assert_eq!(a, 1);
        assert_eq!(c, None);
    }

    #[test]
    fn close_unknown_id_no_op() {
        let (ids, a, c) = close_tab_ids(&[1, 2], 1, Some(2), 99);
        assert_eq!(ids, vec![1, 2]);
        assert_eq!(a, 1);
        assert_eq!(c, Some(2));
    }

    #[test]
    fn switch_to_different_tab_clears_compare() {
        let (a, c) = switch_active(1, Some(2), 3);
        assert_eq!(a, 3);
        assert_eq!(c, None);
    }

    #[test]
    fn switch_to_same_tab_keeps_compare() {
        let (a, c) = switch_active(1, Some(2), 1);
        assert_eq!(a, 1);
        assert_eq!(c, Some(2));
    }

    #[test]
    fn switch_to_compare_partner_clears_compare() {
        // Clicking the B chip in compare mode would otherwise leave us comparing
        // a tab with itself; switch_active forbids that.
        let (a, c) = switch_active(1, Some(2), 2);
        assert_eq!(a, 2);
        assert_eq!(c, None);
    }

    fn dummy_msg(cl_ord: &str, quote: &str, quote_req: &str) -> FixMessage {
        FixMessage {
            arena: Vec::new(),
            fields: Vec::new(),
            time: Default::default(),
            sender: Default::default(),
            target: Default::default(),
            msg_type_raw: Default::default(),
            msg_type_label: "",
            cl_ord_id: cl_ord.into(),
            quote_id: quote.into(),
            quote_req_id: quote_req.into(),
            side: Default::default(),
            order_qty: Default::default(),
            symbol: Default::default(),
            text: Default::default(),
        }
    }

    #[test]
    fn message_key_prefers_cl_ord_id() {
        let m = dummy_msg("ORD1", "Q1", "QR1");
        assert_eq!(message_key(&m).as_deref(), Some("c:ORD1"));
    }

    #[test]
    fn message_key_falls_back_to_quote_id() {
        let m = dummy_msg("", "Q1", "QR1");
        assert_eq!(message_key(&m).as_deref(), Some("q:Q1"));
    }

    #[test]
    fn message_key_falls_back_to_quote_req_id() {
        let m = dummy_msg("", "", "QR1");
        assert_eq!(message_key(&m).as_deref(), Some("qr:QR1"));
    }

    #[test]
    fn message_key_none_for_session_messages() {
        let m = dummy_msg("", "", "");
        assert_eq!(message_key(&m), None);
    }
}
