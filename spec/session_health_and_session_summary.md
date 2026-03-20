# Overview Session Report — Specification

**Feature name:** Overview Session Report  
**Combines:** Smart Session Summary (Feature 8), Fill Quality & Counterparty Scorecard (Feature 4), Session Health AI Diagnostics (Feature 5)  
**Tier:** Pro  
**AI involvement:** Optional (for natural-language summaries and diagnostics)

---

## 1. Context for Implementation

### 1.1 Existing Codebase

| File | Purpose |
|------|---------|
| `src/model.rs` | `FixMessage` (fields, time, sender, target, msg_type_raw, cl_ord_id, symbol, side, order_qty, etc.), `FixField` (tag: u16, value: CompactString) |
| `src/parser.rs` | `parse_all()`, `parse_all_simd_bytes()` → `Vec<FixMessage>` |
| `src/components/lifecycle.rs` | `tag_val(msg, tag)` for field lookup, `parse_fix_time_us()`, latency helpers, order lifecycle grouping |
| `src/dictionary.rs` | `msg_type_label()`, `value_description()`, tag/msgtype mappings |

### 1.2 FIX Tag Reference (for this spec)

| Tag | Name | Usage |
|-----|------|-------|
| 8 | BeginString | FIX version (FIX.4.4, FIXT.1.1) |
| 34 | MsgSeqNum | Sequence number |
| 35 | MsgType | Message type (0=Heartbeat, A=Logon, D=NewOrderSingle, 8=ExecutionReport, F=OrderCancelRequest, 9=OrderCancelReject, etc.) |
| 49 | SenderCompID | Sender |
| 56 | TargetCompID | Target |
| 52 | SendingTime | Timestamp (YYYYMMDD-HH:MM:SS or YYYYMMDD-HH:MM:SS.sss) |
| 108 | HeartBtInt | Heartbeat interval (seconds) |
| 6 | AvgPx | Average price |
| 11 | ClOrdID | Client order ID |
| 14 | CumQty | Cumulative quantity |
| 17 | ExecID | Execution ID |
| 31 | LastPx | Last fill price |
| 32 | LastQty | Last fill quantity |
| 37 | OrderID | Order ID |
| 38 | OrderQty | Order quantity |
| 39 | OrdStatus | Order status (0=New, 1=PartiallyFilled, 2=Filled, 4=Canceled, 8=Rejected) |
| 40 | OrdType | Order type (1=Market, 2=Limit) |
| 41 | OrigClOrdID | Original ClOrdID (for cancel/replace) |
| 44 | Price | Limit price |
| 55 | Symbol | Security symbol |
| 54 | Side | Side (1=Buy, 2=Sell) |
| 150 | ExecType | Execution type (0=New, 4=Canceled, F=Trade/Fill, 8=Rejected) |
| 151 | LeavesQty | Leaves quantity |

### 1.3 Helper: Get Field Value

```rust
// Already exists in lifecycle.rs
fn tag_val(msg: &FixMessage, tag: u16) -> &str {
    msg.fields.iter().find(|f| f.tag == tag).map(|f| f.value.as_str()).unwrap_or("")
}
```

---

## 2. Session Summary (One-Page Executive Report)

### 2.1 Input

- `messages: &[FixMessage]` — output of `parse_all()` or `parse_all_simd_bytes()`

### 2.2 Output Structure

```rust
pub struct SessionSummary {
    pub session_label: String,      // e.g. "BANZAI → EXEC (FIX 4.4)"
    pub begin_string: String,       // Tag 8
    pub sender: String,             // Tag 49 (primary sender)
    pub target: String,             // Tag 56 (primary target)
    pub start_time: String,         // First SendingTime (52)
    pub end_time: String,           // Last SendingTime (52)
    pub duration_str: String,       // e.g. "9h 30m"
    pub order_stats: OrderStats,
    pub latency_stats: LatencyStats,
    pub top_symbols: Vec<(String, u64)>,
    pub notable_events: Vec<NotableEvent>,
}

pub struct OrderStats {
    pub total: u64,
    pub filled: u64,
    pub cancelled: u64,
    pub rejected: u64,
    pub fill_pct: f64,
    pub cancel_pct: f64,
    pub reject_pct: f64,
}

pub struct LatencyStats {
    pub avg_ack_ms: f64,
    pub avg_fill_ms: f64,
    pub worst_spike_ms: f64,
    pub worst_spike_time: Option<String>,
    pub worst_spike_count: u64,
}

pub struct NotableEvent {
    pub severity: EventSeverity,    // Warning, Info, Resolved
    pub time: String,
    pub description: String,
}

pub enum EventSeverity {
    Warning,  // ⚠
    Info,     // ℹ
    Resolved, // ✓
}
```

### 2.3 Computation Algorithm

1. **Session identification:** Group messages by `(tag_val(8), tag_val(49), tag_val(56))`. For bi-directional logs, treat each direction as one logical session or merge into a single "conversation."
2. **Time range:** Min/max of `tag_val(52)` across messages.
3. **Order stats:** Count messages with `tag_val(35) == "D"` (NewOrderSingle) as total orders. For each ClOrdID (11), trace ExecutionReports: if `tag_val(150) == "F"` or `"2"` and `tag_val(39) == "2"` → filled; if `tag_val(39) == "4"` → cancelled; if `tag_val(150) == "8"` or `tag_val(39) == "8"` → rejected.
4. **Latency:** Ack = time from NewOrderSingle (35=D) to first ExecutionReport (35=8). Fill = time from NewOrderSingle to last ExecutionReport with ExecType=F and OrdStatus=2. Compute mean, and find worst spike (e.g. max RTT in a rolling window).
5. **Top symbols:** Count ClOrdID per symbol (55); sort descending; take top 5–10.
6. **Notable events:** Populate from Session Health detections (Section 4).

### 2.4 Display Format (Text)

```
SESSION SUMMARY — 2024-01-02
───────────────────────────────────────────────
Session:        BANZAI → EXEC  (FIX 4.4)
Duration:       08:00:00 — 17:30:22 (9h 30m)

Orders:         1,247 total
  Filled:       1,089  (87.3%)
  Cancelled:    89     (7.1%)
  Rejected:     69     (5.5%)

Avg ack latency:      2.3ms
Avg fill latency:     48ms
Worst latency spike:  210ms at 14:23:18 (47 messages affected)

Top symbols:    MSFT (340), AAPL (287), SPY (201)

Notable events:
  ⚠ 14:23:18  Latency spike — 47 orders delayed >100ms
  ⚠ 11:42:05  Sequence gap (MsgSeqNum 1,823 → 1,831)
  ✓ 09:31:04  3 rejects (stale price, resolved by 09:32)
```

### 2.5 Implementation Location

- **New module:** `src/session_summary.rs` — pure computation, no UI
- **UI:** New component or panel in `src/components/` that renders `SessionSummary`

---

## 3. Fill Quality & Counterparty Scorecard

### 3.1 Input

- `messages: &[FixMessage]`
- Grouping dimensions: counterparty (49/56), symbol (55), optional time bucket

### 3.2 Metrics (Exact Formulas)

| Metric | Formula |
|--------|---------|
| Fill rate | `SUM(CumQty at final fill ER) / SUM(OrderQty from NOS)` per group |
| Slippage (bps) | For limit orders: `(AvgPx - Price) / Price * 10000` for buys; `(Price - AvgPx) / Price * 10000` for sells. Average per fill. |
| Partial fill rate | `COUNT(orders with >1 fill ER) / COUNT(orders)` |
| Reject rate | `COUNT(rejected orders) / COUNT(orders)` |
| Ack latency (ms) | `SendingTime(first ER) - SendingTime(NOS)` in milliseconds |
| Fill latency (ms) | `SendingTime(final fill ER) - SendingTime(NOS)` |
| Cancel success rate | `COUNT(ER with OrdStatus=4) / COUNT(OrderCancelRequest)` |

### 3.3 Data Structure

```rust
pub struct FillQualityScorecard {
    pub rows: Vec<ScorecardRow>,
}

pub struct ScorecardRow {
    pub counterparty: String,   // e.g. "EXEC" or "BANZAI"
    pub symbol: Option<String>, // None = aggregate across symbols
    pub time_bucket: Option<String>, // e.g. "09:00-10:00"
    pub fill_rate: f64,
    pub slippage_bps: f64,
    pub partial_fill_rate: f64,
    pub reject_rate: f64,
    pub avg_ack_ms: f64,
    pub avg_fill_ms: f64,
    pub cancel_success_rate: f64,
    pub order_count: u64,
}
```

### 3.4 Display

- **Table:** Sortable by any column
- **Charts:** Bar chart per counterparty, per symbol (use Plotters or similar)
- **Tree / drill-down:** Click counterparty → show per-symbol breakdown; click symbol → show per-time-bucket

### 3.5 Edge Cases

| Case | Handling |
|------|----------|
| Multiple fills per order | Use last ER with ExecType=F for fill metrics; CumQty and AvgPx from that ER |
| AvgPx (6) vs LastPx (31) | Prefer AvgPx when present; else compute from LastPx/LastQty weighted average |
| Time bucket | Default 1-hour; make configurable (30 min, 1 hr, 4 hr) |
| Orders without symbol | Group under "Unknown" or exclude |

### 3.6 Implementation Location

- **New module:** `src/fill_quality.rs` — computation
- **UI:** `src/components/scorecard.rs` — table + chart + tree navigation

---

## 4. Session Health (Rule-Based Diagnostics)

### 4.1 Input

- `messages: &[FixMessage]`
- Optional: HeartBtInt (108) from first Logon

### 4.2 Detection Rules (No AI Required)

| Pattern | Detection Logic | FIX Tags |
|---------|-----------------|----------|
| Heartbeat gap | Expected heartbeat every `HeartBtInt` sec. If gap > `HeartBtInt * 1.5` between consecutive 35=0, flag. | 35, 52, 108 |
| Sequence gap | MsgSeqNum (34) jumps by >1 without a ResendRequest (35=2) in between | 34, 35 |
| Excessive resends | Count of 35=2 > threshold (e.g. 5 per 1000 messages) | 35 |
| Reconnects | Multiple Logon (35=A) with same SenderCompID/TargetCompID | 35, 49, 56 |
| Message rate burst | >N messages/second (e.g. 100) in a 1-second window | 52 |
| Late cancel | OrderCancelRequest (35=F) SendingTime > final fill ER SendingTime for same ClOrdID | 35, 41, 52 |
| Rejected cancel | OrderCancelReject (35=9) present | 35, 9 |

### 4.3 Output Structure

```rust
pub struct SessionHealthReport {
    pub issues: Vec<HealthIssue>,
}

pub struct HealthIssue {
    pub kind: HealthIssueKind,
    pub severity: IssueSeverity,  // Critical, Warning, Info
    pub time: String,
    pub msg_indices: Vec<usize>,
    pub technical_desc: String,
    pub business_impact: String,  // 2–3 sentences, can be AI-generated later
}

pub enum HealthIssueKind {
    HeartbeatGap,
    SequenceGap,
    ExcessiveResends,
    Reconnect,
    MessageRateBurst,
    LateCancel,
    RejectedCancel,
}
```

### 4.4 Example Business Impact (Template)

> "Three heartbeat gaps occurred at 10:15, 11:42, and 14:23. All three correlate with 'Connection timeout' reject messages 2–3 seconds later, and each was followed by a Logon (Resend). This pattern is consistent with intermittent TCP keepalive failure between your gateway and the execution venue. Check MTU settings or firewall idle-timeout configuration."

### 4.5 Implementation Location

- **New module:** `src/session_health.rs` — rule-based detection
- **UI:** Integrate into Session Summary "Notable events" or a separate "Health" tab

---

## 5. AI Enhancements (Optional, Future)

### 5.1 Smart Session Summary (GPT)

- **Input:** `SessionSummary` + `SessionHealthReport` + `FillQualityScorecard` (as structured data or text)
- **Output:** 1–2 paragraph natural-language executive summary
- **Considerations:** Stream tokens for responsiveness; privacy (what data is sent to API)

### 5.2 Fill Quality Insights (GPT)

- **Input:** Aggregated scorecard
- **Output:** Example: "EXEC outperforms BANZAI on every metric for orders above 5,000 shares. BANZAI's reject rate climbs from 4% to 23% for large orders, suggesting a size-based risk limit. Consider capping BANZAI order size at 4,000 shares or adjusting your smart order router."
- **Considerations:** User opt-in; data anonymization

### 5.3 Health Diagnostics Explanation (GPT)

- **Input:** `HealthIssue` list
- **Output:** Business-impact paragraph per issue (see 4.4 example)
- **Considerations:** Can be done client-side with templates first; GPT for richer explanations

---

## 6. Implementation Order

| Phase | Task | Files |
|-------|------|-------|
| 1 | `SessionSummary` struct + computation | `src/session_summary.rs` |
| 2 | Session Summary UI panel | `src/components/session_summary.rs` |
| 3 | `SessionHealthReport` + detection rules | `src/session_health.rs` |
| 4 | Integrate health issues into summary "Notable events" | `session_summary.rs`, UI |
| 5 | `FillQualityScorecard` struct + computation | `src/fill_quality.rs` |
| 6 | Scorecard table + chart UI | `src/components/scorecard.rs` |
| 7 | Tree/drill-down for counterparty → symbol | `scorecard.rs` |
| 8 | Export to CSV/PDF (reuse existing export if any) | `src/export.rs` |
| 9 | AI summary (optional) | New module + API integration |

---

## 7. UI Layout Suggestion

```
┌─────────────────────────────────────────────────────────────────┐
│  Overview Session Report                                        │
├─────────────────────────────────────────────────────────────────┤
│  [Session Summary]  [Fill Quality]  [Health]  [Export ▼]        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  SESSION SUMMARY — 2024-01-02                                    │
│  ─────────────────────────────────────────────                  │
│  Session: BANZAI → EXEC (FIX 4.4)                                │
│  Duration: 08:00:00 — 17:30:22 (9h 30m)                         │
│  ...                                                             │
│                                                                  │
│  [Notable events list]                                           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 8. Research Questions (Deferred)

- Stream GPT output token-by-token for perceived speed
- Privacy: which metrics require opt-in before sending to API
- Scheduled summaries (e.g. EOD when log file detected)
- Diagnostics: separate panel vs inline timeline annotations
- Baseline "normal" (e.g. first 5 min) vs deviation detection
- Configurable alert thresholds per user/venue
