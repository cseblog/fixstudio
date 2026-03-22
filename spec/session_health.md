Session Health — Design Proposal
The current implementation collapses each rule into a flat HealthIssue string. For 1M messages this doesn't scale visually — you'd get hundreds of identical rows. The proposal: each rule gets its own rich data struct and a purpose-fit chart type.

Architecture change
Add a typed payload enum alongside the existing HealthIssue:


HealthIssue {
    kind, severity, time, msg_indices,
    technical_desc, business_impact,
    detail: HealthDetail,          // ← NEW: typed payload per rule
}

enum HealthDetail {
    HeartbeatGaps(HeartbeatGapDetail),
    SequenceGaps(SequenceGapDetail),
    Resends(ResendDetail),
    Reconnects(ReconnectDetail),
    RateBursts(RateBurstDetail),
    LateCancels(LateCancelDetail),
    RejectedCancels(RejectedCancelDetail),
    None,
}
Rule-by-rule design
1 — Heartbeat Gap


pub struct HeartbeatGapDetail {
    pub configured_interval_sec: i64,
    pub gaps: Vec<HeartbeatGap>,   // individual gap events
}
pub struct HeartbeatGap {
    pub time: String,
    pub gap_ms: i64,
    pub msg_idx: usize,
}
Multiple gaps are grouped into one issue (not N issues)
Chart: scatter plot — X = time-of-day, Y = gap duration (ms), dashed threshold line
Good for seeing "gaps cluster at market open" vs random
2 — Sequence Gap


pub struct SequenceGapDetail {
    pub total_missing: u64,
    pub gaps: Vec<SequenceGap>,
}
pub struct SequenceGap {
    pub from_seq: u64,
    pub to_seq: u64,
    pub missing: u64,
    pub time: String,
    pub indices: [usize; 2],   // msg before and after gap
}
Chart: bar chart — each bar = one gap, height = number of missing seqnums, X = time
Reveals a single large gap (session reset) vs many small gaps (drops) at a glance
3 — Excessive Resends


pub struct ResendDetail {
    pub count: usize,
    pub rate_per_1000: usize,
    pub instances: Vec<ResendInstance>,
}
pub struct ResendInstance {
    pub time: String,
    pub begin_seq: u64,    // tag 7 (BeginSeqNo)
    pub end_seq: u64,      // tag 16 (EndSeqNo), 0 = infinity
    pub msg_idx: usize,
}
Chart: timeline scatter — each dot = one resend request, X = time
Shows burst clusters vs evenly distributed → helps diagnose cause
4 — Reconnects


pub struct ReconnectDetail {
    pub logons: Vec<LogonEvent>,
}
pub struct LogonEvent {
    pub time: String,
    pub seq_num: u64,       // tag 34 at logon — reset if 1
    pub reset_seq: bool,    // seq_num == 1 indicates reset
    pub msg_idx: usize,
}
Chart: connection state timeline — horizontal bar that goes red during gap between logons, green when up
Each logon marked with a label; grey for seq-reset logons (new session) vs amber for mid-session reconnects
5 — Message Rate Burst


pub struct RateBurstDetail {
    pub threshold: usize,              // e.g. 100/sec
    pub buckets: Vec<RateBucket>,      // all 1-sec windows (not just bursts)
    pub burst_indices: Vec<usize>,     // which bucket indices exceeded threshold
}
pub struct RateBucket {
    pub second_label: String,
    pub count: usize,
}
Computation change for 1M messages: bucket by floor(us / 1_000_000) using a HashMap → O(N) time, O(seconds) space, no sliding window
Chart: area/bar chart of message rate over time, threshold line, exceeded windows highlighted in amber
Most visually informative rule
6 — Late Cancels


pub struct LateCancelDetail {
    pub cases: Vec<LateCancelCase>,
}
pub struct LateCancelCase {
    pub cl_ord_id: String,
    pub fill_time: String,
    pub cancel_time: String,
    pub lag_ms: i64,        // how late the cancel was
    pub msg_idx: usize,
}
Chart: scatter plot — X = time of cancel, Y = lag in ms
Clusters at certain times = systemic problem (e.g. slow OMS at market open)
Also show a small lag histogram (how late: <1ms / 1-10ms / >10ms buckets)
7 — Rejected Cancels


pub struct RejectedCancelDetail {
    pub rejections: Vec<CancelRejection>,
}
pub struct CancelRejection {
    pub time: String,
    pub orig_cl_ord_id: String,
    pub reason_code: String,    // tag 102
    pub reason_text: String,    // human label for code
    pub msg_idx: usize,
}
Tag 102 codes: 0=TooLateToCancel, 1=UnknownOrder, 3=AlreadyPendingCancel

Chart: donut pie — breakdown by rejection reason code
Shows whether problem is "too late" (race condition) vs "unknown order" (OMS sync issue)
Timeline scatter as secondary view
Health tab UI layout
Each rule becomes a collapsible card instead of a flat list row:


┌─ ⚠ Sequence Gap  ─────────────────────────────── 3 gaps · 47 missing ─┐
│  [bar chart: gap size over time]                                         │
│  from 1823→1831 at 11:42, from 2100→2105 at 14:23, ...                 │
└──────────────────────────────────────────────────────────────────────────┘
┌─ ▲ Message Rate Burst ───────────────────────── 2 bursts · peak 143/s ─┐
│  [area chart: msg rate over session]                                     │
└──────────────────────────────────────────────────────────────────────────┘
Summary: 3 files change — session_health.rs (data structs + computation), overview.rs (Health tab renderer), style.rs (card CSS).

Do you want to proceed with this design, or adjust anything before I start coding?