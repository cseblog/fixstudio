# Data Model

All persistent data structures passed between the parser, analysis layer, and UI.

---

## FixMessage (`src/model.rs`)

The output of one parsed FIX message.

```rust
pub struct FixMessage {
    // ── Arena ─────────────────────────────────────────────────────────
    pub arena:          Vec<u8>,          // verbatim copy of the raw message bytes

    // ── Fields ────────────────────────────────────────────────────────
    pub fields:         Vec<FixField>,    // one entry per tag=value pair, 8 bytes each

    // ── Pre-extracted hot fields ──────────────────────────────────────
    // Stored as CompactString for O(1) access in the timeline/detail views.
    // CompactString stores values ≤23 bytes inline (no heap); longer values
    // fall back to a heap-allocated small string.
    pub time:           CompactString,    // tag 52, formatted YYYY-MM-DD HH:MM:SS
    pub sender:         CompactString,    // tag 49 SenderCompID
    pub target:         CompactString,    // tag 56 TargetCompID
    pub msg_type_raw:   CompactString,    // tag 35 raw code ("D", "8", "0", ...)
    pub msg_type_label: &'static str,     // human label ("NewOrderSingle", "ExecutionReport", ...)
    pub cl_ord_id:      CompactString,    // tag 11 ClientOrderID
    pub quote_id:       CompactString,    // tag 117
    pub quote_req_id:   CompactString,    // tag 131
    pub side:           CompactString,    // tag 54 label ("BUY" / "SELL" / ...)
    pub order_qty:      CompactString,    // tag 38
    pub symbol:         CompactString,    // tag 55
    pub text:           CompactString,    // tag 58
}
```

**Memory layout note:** With 13 `CompactString` fields (24 bytes each) + 1 `&'static str`
(16 bytes) + 2 `Vec` headers (24 bytes each) = ~384 bytes per struct before the heap data.
For 1M messages the `Vec<FixMessage>` pointer array alone is ~384 MB.

### Arena Design (Data-Oriented)

The `arena` field holds a verbatim copy of the raw message bytes. `FixField` stores
`(value_start, value_len)` byte offsets into this arena rather than owning a string.

**Why:** `CompactString` is 24 bytes (inline up to 23 bytes, or heap pointer otherwise).
`FixField` with an owned value would be 2 (tag) + 22 (padding/CompactString) = 24+ bytes.
With the arena design, `FixField` is exactly 8 bytes — a **4× reduction** in field-iteration
memory bandwidth.

```
Before (CompactString per field):
  FixField = { tag: u16 [2b], _pad [6b], value: CompactString [24b] } = 32 bytes
  1M messages × 20 fields × 32 bytes = 640 MB of field storage

After (arena offsets):
  FixField = { tag: u16 [2b], value_len: u16 [2b], value_start: u32 [4b] } = 8 bytes
  1M messages × 20 fields × 8 bytes  = 160 MB of field storage
  + 1M arena Vecs × ~190 bytes avg   = ~190 MB of arena data
  Total: ~350 MB (vs 640 MB) — 45% less field memory
```

### Helper Methods

```rust
impl FixMessage {
    // Access any field value as &str — zero-copy, borrows from arena
    // field.value_in(&msg.arena)   →   &str lifetime tied to msg.arena

    // Build a FixMessage from individual fields (used in tests/validator)
    pub fn push_field(&mut self, tag: u16, value: &str)

    // Replace the value of a field by appending new bytes to arena
    // (old bytes remain in arena but are unreachable; arena is append-only)
    pub fn set_field_value(&mut self, tag: u16, value: &str)
}
```

---

## FixField (`src/model.rs`)

One `tag=value` pair stored as arena byte offsets.

```rust
pub struct FixField {
    pub tag:         u16,   // FIX tag number (1–9999)
    pub value_len:   u16,   // byte length of value in arena
    pub value_start: u32,   // byte offset in parent FixMessage.arena
}
// Size: 8 bytes total (field order chosen for zero padding)
```

```rust
impl FixField {
    #[inline]
    pub fn value_in<'a>(&self, arena: &'a [u8]) -> &'a str {
        let slice = &arena[self.value_start as usize..][..self.value_len as usize];
        // SAFETY: FIX protocol fields are 7-bit ASCII, which is valid UTF-8.
        unsafe { std::str::from_utf8_unchecked(slice) }
    }
}
```

**Typical usage:**

```rust
// In analysis code:
fn tag_val<'a>(msg: &'a FixMessage, tag: u16) -> &'a str {
    msg.fields.iter()
        .find(|f| f.tag == tag)
        .map(|f| f.value_in(&msg.arena))
        .unwrap_or("")
}

// In UI (RSX macro, inlined due to RSX macro limitations):
span { "{field.value_in(&msg.arena)}" }
```

---

## UI & App Types (`src/types.rs`)

```rust
pub enum UpdateStatus {
    Unchecked,
    Checking,
    UpToDate,
    UpdateAvailable { version: String, url: String },
    Error(String),
}

pub enum ViewMode {
    Timeline,
    Lifecycle,
    Overview,
    Validator,
}
```

`ViewMode` drives which right-panel component is rendered in `app.rs`.

---

## Validation Types (`src/validator.rs`)

### ValidationReport

```rust
pub struct ValidationReport {
    pub issues:               Vec<Issue>,
    pub checksum_ok:          Option<bool>,    // None if validate_fields (no raw bytes)
    pub checksum_found:       Option<String>,  // Actual tag 10 value
    pub checksum_expected:    Option<String>,  // Computed from body bytes
    pub body_length_ok:       Option<bool>,    // None if validate_fields
    pub body_length_found:    Option<u32>,     // Actual tag 9 value
    pub body_length_counted:  Option<u32>,     // Computed byte count
}
```

`checksum_*` and `body_length_*` are only populated when calling `validate_raw(&[u8])` —
the raw bytes are needed to compute expected values. `validate_fields` operates on the
already-parsed `FixMessage` and cannot recover the original byte sequence.

### Issue

```rust
pub struct Issue {
    pub severity:  Severity,         // Error | Warning
    pub tag:       Option<u16>,      // None = structural; Some(n) = specific field
    pub code:      &'static str,     // Machine-readable: "MISSING_HEADER_TAG", etc.
    pub message:   String,           // Human-readable description
    pub fix_hint:  Option<String>,   // Suggested correction
}

pub enum Severity { Error, Warning }
```

**Known issue codes:**

| Code | Meaning |
|---|---|
| `MISSING_HEADER_TAG` | Required header tag absent (8, 9, 35, 34, 49, 52, 56) |
| `MISSING_BODY_TAG` | Required body tag absent for this MsgType |
| `INVALID_ENUM` | Tag value not in allowed enum set |
| `DUPLICATE_TAG` | Same tag appears twice in one message |
| `CONDITIONAL_TAG_MISSING` | Tag Y required when tag X is present |
| `FIX_VERSION_MISMATCH` | Tag introduced after BeginString version |
| `CHECKSUM_ERROR` | Tag 10 value doesn't match computed checksum |
| `BODY_LENGTH_ERROR` | Tag 9 value doesn't match computed length |

---

## Session Health Types (`src/session_health.rs`)

### SessionHealthReport

```rust
pub struct SessionHealthReport {
    pub issues: Vec<HealthIssue>,
}

pub struct HealthIssue {
    pub kind:             HealthIssueKind,
    pub severity:         IssueSeverity,       // Critical | Warning | Info
    pub time:             String,              // ISO 8601 of first occurrence
    pub msg_indices:      Vec<usize>,          // Indices into the messages slice
    pub technical_desc:   String,
    pub business_impact:  String,
    pub detail:           HealthDetail,        // Typed payload per rule
}
```

### 7 Health Rules

```
HealthIssueKind         HealthDetail type            What triggers it
─────────────────────   ──────────────────────────   ──────────────────────────────
SequenceGap             SequenceGapDetail             tag 34 jumps by >1
HeartbeatGap            HeartbeatGapDetail            >interval seconds between tag 0
ExcessiveResends        ResendDetail                  ResendRequest (35=2) rate high
Reconnect               ReconnectDetail               Multiple Logon (35=A) sequences
MessageRateBurst        RateBurstDetail               >100 msgs/sec in 1-second window
LateCancel              LateCancelDetail              OrderCancelReject after partial fill
RejectedCancel          RejectedCancelDetail          OrderCancelReject (35=9) present
```

### SequenceGapDetail (most complex)

```rust
pub struct SequenceGapDetail {
    pub total_missing:  u64,
    pub gaps:           Vec<SequenceGap>,
}

pub struct SequenceGap {
    pub from_seq:  u64,
    pub to_seq:    u64,
    pub missing:   u64,       // to_seq - from_seq - 1
    pub time:      String,    // time of the message after the gap
    pub indices:   [usize; 2], // [last message before gap, first after gap]
}
```

---

## Session Summary Types (`src/session_summary.rs`)

```rust
pub struct SessionSummary {
    pub session_label:    String,        // "{sender} → {target}" or "Multi-session"
    pub begin_string:     String,        // FIX.4.4, FIXT.1.1, ...
    pub sender:           String,
    pub target:           String,
    pub session_count:    usize,         // Distinct (sender, target) pairs
    pub start_time:       String,
    pub end_time:         String,
    pub duration_str:     String,        // "2h 34m" etc.
    pub total_messages:   u64,
    pub order_stats:      OrderStats,
    pub latency_stats:    LatencyStats,
    pub top_symbols:      Vec<(String, u64)>,  // Top 10 by message count
    pub health:           SessionHealthReport,
}

pub struct OrderStats {
    pub total:       u64,
    pub filled:      u64,
    pub cancelled:   u64,
    pub rejected:    u64,
    pub fill_pct:    f64,    // filled / total (%)
    pub cancel_pct:  f64,
    pub reject_pct:  f64,
}

pub struct LatencyStats {
    pub avg_ack_ms:          f64,  // NOS → first ER
    pub avg_fill_ms:         f64,  // NOS → final fill ER
    pub worst_spike_ms:      f64,
    pub worst_spike_time:    Option<String>,
    pub worst_spike_count:   u64,  // How many spikes above threshold
}
```

---

## Fill Quality Types (`src/fill_quality.rs`)

```rust
pub struct FillQualityScorecard {
    pub rows:         Vec<ScorecardRow>,    // Aggregate per counterparty
    pub detail_rows:  Vec<ScorecardRow>,    // Per (counterparty, symbol)
    pub size_rows:    Vec<SizeBucketRow>,   // Per (counterparty, symbol, notional bucket)
}

pub struct ScorecardRow {
    pub counterparty:       String,
    pub symbol:             Option<String>,   // None for aggregate rows
    pub fill_rate:          f64,              // filled_qty / ordered_qty
    pub slippage_bps:       f64,              // (fill_price - limit) / limit × 10_000
    pub partial_fill_rate:  f64,              // partial fills / all fills
    pub reject_rate:        f64,              // rejected_qty / ordered_qty
    pub avg_ack_ms:         f64,              // NOS → first ER latency
    pub avg_fill_ms:        f64,              // NOS → fill ER latency
    pub cancel_success_rate: f64,             // successful cancels / cancel requests
    pub order_count:        u64,
}

pub struct SizeBucketRow {
    pub counterparty:  String,
    pub symbol:        String,
    pub bucket:        SizeBucket,
    pub fill_rate:     f64,
    pub slippage_bps:  f64,
    pub order_count:   u64,
}

pub enum SizeBucket {
    Under1M,    // notional < 1,000,000
    M1To5M,     // 1M ≤ notional < 5M
    M5To10M,
    M10To50M,
    Over50M,
}
```

---

## License Type (`src/license.rs`)

```rust
#[derive(Serialize, Deserialize)]
pub struct StoredLicense {
    pub key:         String,    // Whop license key
    pub activated:   String,    // ISO 8601 activation timestamp
    pub email:       Option<String>,
}
```

Persisted at:
- macOS: `~/Library/Application Support/AiFIXParser/license.json`
- Windows: `%APPDATA%\AiFIXParser\license.json`
- Linux: `~/.config/aifixparser/license.json`

---

## FileLoadResult / FolderLoadResult (`src/loader.rs`)

```rust
pub struct FileLoadResult {
    pub name:       String,
    pub messages:   Vec<FixMessage>,
    pub parse_us:   u64,       // Parse time in microseconds
    pub is_soh:     bool,      // true = SOH-delimited, false = pipe
}

pub struct FolderLoadResult {
    pub folder_name: String,
    pub messages:    Vec<FixMessage>,  // All messages from all files merged
    pub parse_us:    u64,
    pub file_names:  Vec<String>,      // "path/to/file.log (N msgs)", sorted
}
```

Folder loading merges all messages chronologically by discovery order (directory traversal is
DFS; messages from different files are not sorted by time — that is a known limitation).
