# Pro Features Specification

Detailed specifications for three premium features: Session/Order Lifecycle View, Export to CSV/Excel, and RTT/Latency Metrics.

---

## 1. Session / Order Lifecycle View

### 1.1 Overview

Group FIX messages by logical entities (sessions and orders) so users can trace conversation flows and order lifecycles instead of scrolling a flat timeline.

### 1.2 Entities

#### 1.2.1 Session

**Definition:** A logical FIX session between two counterparties, identified by BeginString + SenderCompID + TargetCompID.

| Field | Source | Notes |
|-------|--------|-------|
| `session_id` | Derived: `{BeginString}_{SenderCompID}_{TargetCompID}` | Unique per session |
| `begin_string` | Tag 8 | FIX.4.4, FIXT.1.1, etc. |
| `sender` | Tag 49 | SenderCompID |
| `target` | Tag 56 | TargetCompID |
| `first_msg_index` | Parser index | First message in session |
| `last_msg_index` | Parser index | Last message in session |
| `message_count` | Calculated | Count of messages in session |
| `time_range` | Min/max of tag 52 | SendingTime span |
| `message_types` | Set of tag 35 | MsgTypes present in session |

**Session boundaries:** A new session starts when (BeginString, SenderCompID, TargetCompID) differs from the previous message. Logout (35=5) or end-of-file does not create a new session; the next message with different session keys does.

**Bi-directional sessions:** EXEC↔BANZAI produces two logical sessions (EXEC→BANZAI and BANZAI→EXEC) because SenderCompID/TargetCompID swap. The UI may optionally group these as a **conversation pair** for display.

#### 1.2.2 Order

**Definition:** An order lifecycle identified by ClOrdID (11) within a session. Optionally also by OrderID (37) for ExecutionReports.

| Field | Source | Notes |
|-------|--------|-------|
| `order_id` | Derived: `{session_id}_{ClOrdID}` | Unique per order |
| `cl_ord_id` | Tag 11 | Client Order ID |
| `order_id` | Tag 37 | From ExecutionReport (may be empty initially) |
| `orig_cl_ord_id` | Tag 41 | For cancel/replace; links to parent order |
| `symbol` | Tag 55 | Security |
| `side` | Tag 54 | 1=Buy, 2=Sell, etc. |
| `order_qty` | Tag 38 | Quantity |
| `ord_type` | Tag 40 | 1=Market, 2=Limit, etc. |
| `ord_status` | Tag 39 | Latest status from ExecutionReport |
| `exec_type` | Tag 150 | Latest ExecType |
| `session_id` | Derived | Parent session |
| `message_indices` | List[int] | Parser indices of all messages in this order's lifecycle |

**Order lifecycle message sequence (typical):**

1. **NewOrderSingle (35=D)** – Order creation
2. **ExecutionReport (35=8)** – Pending New (OrdStatus=A, ExecType=5)
3. **ExecutionReport** – New (OrdStatus=0, ExecType=0)
4. **ExecutionReport** – Partial Fill (OrdStatus=1, ExecType=F)
5. **ExecutionReport** – Fill (OrdStatus=2, ExecType=F)
6. **OrderCancelRequest (35=F)** – Cancel request (optional)
7. **ExecutionReport** – Canceled (OrdStatus=4, ExecType=4)

**Or:** Reject (35=8 with ExecType=8), OrderCancelReject (35=9), etc.

**Order linking:** OrderCancelRequest and OrderCancelReplaceRequest reference OrigClOrdID (41). The system links these to the parent order for cancel/replace chains.

### 1.3 Session View UI

#### 1.3.1 Session List (Sidebar or Top-Level)

| Column | Description |
|--------|-------------|
| Session | `{Sender} ↔ {Target}` (e.g. EXEC ↔ BANZAI) |
| BeginString | FIX.4.4, FIXT.1.1 |
| Msg Count | Number of messages |
| Time Range | Start time – End time |
| Orders | Number of distinct orders (ClOrdIDs) |

**Actions:** Click session → expand to show messages and orders.

#### 1.3.2 Session Detail View

When a session is selected:

- **Message list:** Same columns as current timeline (Time, Sender, Target, Message, ClOrdID, Detail), filtered to this session.
- **Order summary:** Collapsible list of orders in this session.
- **Filters:** Apply existing column filters within the session.

#### 1.3.3 Order Lifecycle View

When an order is selected:

- **Order header card:**
  - ClOrdID, Symbol, Side, OrderQty, OrdType
  - Latest OrdStatus, ExecType
  - CumQty (14), AvgPx (6), LeavesQty (151) when available
- **Timeline:** Messages that belong to this order, in chronological order:
  - NewOrderSingle
  - Each ExecutionReport (with status badge)
  - OrderCancelRequest / OrderCancelReplaceRequest (if any)
  - OrderCancelReject (if any)
- **Visual flow:** Optional diagram: D → ER(0) → ER(F) → …
- **Parent order link:** If OrigClOrdID is set, show link to parent order (cancel/replace chain).

### 1.4 Data Model Additions

```text
Session {
  id: String
  begin_string: String
  sender: String
  target: String
  message_indices: Vec<usize>
  orders: Vec<Order>  // derived
}

Order {
  id: String
  cl_ord_id: String
  order_id: Option<String>
  orig_cl_ord_id: Option<String>
  symbol: String
  side: String
  order_qty: String
  ord_type: String
  ord_status: String
  exec_type: String
  cum_qty: String
  avg_px: String
  leaves_qty: String
  message_indices: Vec<usize>
  session_id: String
}
```

### 1.5 Computation

- **Sessions:** Single pass over parsed messages; create/update session when (8, 49, 56) changes.
- **Orders:** Single pass per session; group by ClOrdID (11). For ExecutionReports without ClOrdID, use OrderID to match to prior ER with same OrderID.
- **Cancel/Replace chains:** Index orders by ClOrdID; when OrigClOrdID appears, link child to parent.

### 1.6 Edge Cases

| Case | Handling |
|------|----------|
| Messages without ClOrdID | Exclude from order view; remain in session view |
| Duplicate ClOrdID | Same order; append messages |
| Out-of-order messages | Sort by SendingTime (52) within order |
| Gap fill / Resend | Session view includes them; order view may omit if not tied to ClOrdID |
| Multiple sessions in one file | Show all sessions; user selects one |

---

## 2. Export to CSV/Excel

### 2.1 Overview

Export the current timeline (filtered) to CSV or Excel for reporting, audits, and downstream analysis.

### 2.2 Export Scope

- **Source:** Messages in the current timeline view **after filters applied** (including session/order filter if active).
- **Columns:** Configurable (see below).
- **Row:** One row per message, or one row per order (aggregated) in “Order Summary” mode.

### 2.3 CSV Export

#### 2.3.1 Format

- **Encoding:** UTF-8 with BOM (for Excel compatibility).
- **Delimiter:** Comma (`,`).
- **Quote:** Double quote (`"`) for fields containing comma, newline, or quote.
- **Line ending:** CRLF (`\r\n`) for Windows compatibility.
- **Header row:** Optional; default on.

#### 2.3.2 Column Definitions

**Standard columns (always available):**

| Column | Source | Example |
|--------|--------|---------|
| Seq | Row index (1-based) | 1 |
| Time | SendingTime (52) formatted | 2012-11-05 23:24:06 |
| Sender | Tag 49 | EXEC |
| Target | Tag 56 | BANZAI |
| MsgType | Tag 35 | D |
| MsgTypeLabel | Resolved label | NewOrderSingle |
| ClOrdID | Tag 11 | 1352157882577 |
| OrderID | Tag 37 | 1 |
| Symbol | Tag 55 | MSFT |
| Side | Tag 54 (resolved) | BUY |
| OrderQty | Tag 38 | 10000 |
| OrdStatus | Tag 39 (resolved) | Filled |
| ExecType | Tag 150 (resolved) | Fill |
| CumQty | Tag 14 | 10000 |
| AvgPx | Tag 6 | 12.3 |
| Price | Tag 44 | 0 |
| Text | Tag 58 | |
| RawMessage | Full pipe-delimited message | 8=FIX.4.4\|9=61\|... |

**Extended columns (optional, opt-in):**

- All tags present in the message as separate columns: `Tag_8`, `Tag_9`, `Tag_35`, etc.
- Value descriptions: `MsgType_Desc`, `OrdStatus_Desc`, etc.

#### 2.3.3 User Options

| Option | Default | Description |
|--------|---------|-------------|
| Include header | true | First row is column names |
| Include raw message | false | Add RawMessage column |
| Include resolved values | true | Use dictionary labels where available |
| Extended columns | false | Add per-tag columns |
| Date format | YYYY-MM-DD HH:MM:SS | Configurable? |

### 2.4 Excel Export

#### 2.4.1 Format

- **Format:** XLSX (Office Open XML).
- **Library:** `rust_xlsxwriter` or similar.
- **Sheet name:** `FIX Messages` (or sanitized filename).
- **Max rows:** Excel limit 1,048,576; warn or split if exceeded.

#### 2.4.2 Excel-Specific Features

| Feature | Description |
|---------|-------------|
| Header formatting | Bold, freeze first row |
| Column auto-fit | Auto-size columns to content |
| Time column | Excel datetime format for sorting/filtering |
| Multiple sheets | Optional: Sheet1=Messages, Sheet2=Order Summary |
| Filters | Auto-filter on header row |

#### 2.4.3 Order Summary Sheet (Optional)

When “Order Summary” mode is selected:

- One row per order (by ClOrdID).
- Columns: ClOrdID, Symbol, Side, OrderQty, OrdType, OrdStatus, CumQty, AvgPx, LeavesQty, FirstMsgTime, LastMsgTime, MsgCount.

### 2.5 UI Flow

1. User applies filters (optional).
2. User clicks **Export** (or File → Export).
3. Modal:
   - Format: [CSV] [Excel]
   - Columns: [Standard] [Standard + Raw] [Extended]
   - Mode: [Per Message] [Order Summary] (if orders computed)
   - Options: Include header, date format, etc.
4. User clicks **Export**.
5. File save dialog (native `rfd`).
6. Export runs; progress indicator for large datasets.
7. Success message with file path.

### 2.6 Performance

- **Streaming CSV:** Write line-by-line to avoid large memory use.
- **Excel:** Write in chunks; show progress for >10k rows.
- **Large exports:** Consider background task + notification when done.

### 2.7 Edge Cases

| Case | Handling |
|------|----------|
| Empty filtered set | Warn “No messages to export” |
| Special chars in values | Escape per CSV/Excel rules |
| Very long raw message | Truncate or allow wrap; document limit |
| Excel row limit | Split into multiple sheets or warn |

---

## 3. RTT / Latency Metrics

### 3.1 Overview

Measure round-trip time (RTT) and latency between related FIX messages to support operations and SLA monitoring.

### 3.2 Metric Definitions

#### 3.2.1 Round-Trip Time (RTT)

**Definition:** Time from a request sent by A to the response received from B, both within the same logical flow.

| Flow | Request (A→B) | Response (B→A) | RTT =
|------|---------------|-----------------|------
| Logon | Logon (A) | Logon (A) | T(response) - T(request) |
| Heartbeat | Heartbeat (A) | Heartbeat (B) | T(response) - T(request) |
| Test Request | TestRequest (A) | Heartbeat with TestReqID (B) | T(response) - T(request) |
| Order | NewOrderSingle (A) | ExecutionReport Ack (B) | T(first ER) - T(NOS) |
| Cancel | OrderCancelRequest (A) | ExecutionReport Canceled (B) | T(ER) - T(OCR) |

**Matching rules:**
- Same session (Sender/Target swapped for bi-directional).
- Request and response linked by: MsgSeqNum, TestReqID, ClOrdID, or OrderID as applicable.

#### 3.2.2 Latency Percentiles

For a set of RTT samples:

| Metric | Definition |
|--------|------------|
| Min | Minimum RTT (ms) |
| Max | Maximum RTT (ms) |
| Mean | Average RTT (ms) |
| P50 (Median) | 50th percentile |
| P95 | 95th percentile |
| P99 | 99th percentile |
| P99.9 | 99.9th percentile (optional) |
| Count | Number of samples |
| Std Dev | Standard deviation (optional) |

#### 3.2.3 Message-Specific Metrics

| Metric | Description |
|--------|-------------|
| **Logon RTT** | Logon → Logon response |
| **Heartbeat RTT** | Heartbeat → next Heartbeat (from peer) |
| **Order Ack Latency** | NewOrderSingle → first ExecutionReport (Pending New or New) |
| **Fill Latency** | NewOrderSingle → ExecutionReport with ExecType=F (fill) |
| **Cancel Latency** | OrderCancelRequest → ExecutionReport OrdStatus=4 |

### 3.3 Computation

#### 3.3.1 RTT Pairs

1. Sort all messages by SendingTime (52).
2. For each request type:
   - Find request message (e.g. 35=D, Sender=A, Target=B).
   - Find matching response: same session (B→A), within time window (e.g. 60s), matching ClOrdID/OrderID/TestReqID.
   - RTT = response time - request time (in ms).
3. Store pairs: (request_index, response_index, rtt_ms).

#### 3.3.2 Aggregation

- Group RTTs by metric type (Logon, OrderAck, Fill, Cancel, etc.).
- Compute min, max, mean, percentiles.
- Percentiles: sort values, then index = ceil(p/100 * n) - 1.

### 3.4 Data Model

```text
RttPair {
  request_index: usize
  response_index: usize
  flow_type: FlowType  // Logon, Heartbeat, OrderAck, Fill, Cancel, ...
  rtt_ms: f64
  cl_ord_id: Option<String>
  sender: String
  target: String
}

LatencyStats {
  flow_type: FlowType
  count: u64
  min_ms: f64
  max_ms: f64
  mean_ms: f64
  p50_ms: f64
  p95_ms: f64
  p99_ms: f64
}
```

### 3.5 UI

#### 3.5.1 Latency Panel / Tab

- **Summary table:**

| Flow Type | Count | Min | Mean | P50 | P95 | P99 | Max |
|-----------|-------|-----|------|-----|-----|-----|-----|
| Logon | 2 | 45 | 52 | 50 | 62 | 65 | 65 |
| Order Ack | 15 | 12 | 28 | 25 | 48 | 52 | 55 |
| Fill | 5 | 120 | 185 | 180 | 210 | 220 | 225 |
| Cancel | 2 | 35 | 40 | 40 | 42 | 42 | 42 |

- **Filters:** By session, time range, symbol (for order-related metrics).
- **Drill-down:** Click row → show individual RTT pairs in a table with links to source messages.

#### 3.5.2 Per-Message RTT Overlay

- In timeline, show RTT badge when a message is the response of an RTT pair.
- Tooltip: “RTT: 28ms (Order Ack)”.
- Optional: Color-code by threshold (e.g. green &lt;50ms, yellow 50–200ms, red &gt;200ms).

#### 3.5.3 Histogram (Optional)

- Histogram of RTT distribution for selected flow type.
- Bins: e.g. 0–10, 10–25, 25–50, 50–100, 100–200, 200–500, &gt;500 ms.

### 3.6 Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| Max RTT window (sec) | 60 | Ignore responses >60s after request |
| Percentile thresholds | P95, P99 | Which percentiles to compute |
| SLA thresholds | User-defined | e.g. OrderAck < 100ms = green |
| Include Heartbeats | false | Heartbeat RTT often noisy; opt-in |

### 3.7 Edge Cases

| Case | Handling |
|------|----------|
| No matching response | Exclude from RTT; optionally count as “orphan request” |
| Multiple responses | e.g. NOS → Ack, then Fill; use first ER for Ack, Fill ER for Fill |
| Out-of-order timestamps | Use SendingTime; if response time < request time, invalid pair |
| Duplicate ClOrdID | Each NOS is separate; match by position or sequence |
| Gap fill / Resend | Exclude from latency (not real-time flow) |

### 3.8 Time Resolution

- SendingTime (52): `YYYYMMDD-HH:MM:SS.sss` – millisecond precision.
- RTT in ms: parse both timestamps, compute difference.
- If no milliseconds: assume .000; resolution is seconds.

---

## 4. Feature Dependencies

| Feature | Depends On |
|---------|------------|
| Session View | Parser (current) |
| Order Lifecycle | Session detection, ClOrdID/OrderID indexing |
| Export CSV | Current message model, filters |
| Export Excel | Export CSV logic, xlsx crate |
| RTT Metrics | Parsed messages, Session detection, order linking |

---

## 5. Implementation Phases

**Phase 1 – Session / Order Lifecycle**
- Session detection and grouping
- Session list UI
- Order extraction and lifecycle view
- Integration with existing timeline

**Phase 2 – Export**
- CSV export with configurable columns
- Excel export (single sheet)
- Order Summary sheet (optional)

**Phase 3 – RTT Metrics**
- RTT pair detection (Order Ack, Fill, Cancel first)
- Latency stats computation
- Latency panel UI
- Per-message RTT overlay

---

## 6. Success Metrics

| Feature | Metric |
|---------|--------|
| Session View | % of sessions with ≥1 order correctly linked |
| Export | Export completes for files up to 100k messages |
| RTT | >95% of NOS→ER pairs correctly matched |
