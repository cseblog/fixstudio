# FIX Studio — GUI Design

> Three-section desktop app: **Parser** → **Analysis** → **Build**
> Each section is a top-level tab. Analysis and Build have sub-views.

---

## Overall App Shell

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  ⚡ FIX Studio                                                    [_] [□] [×]  │
├─────────────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐   ┌───────────────┐   ┌──────────────┐                       │
│  │  📋  Parser  │   │  🔬  Analysis │   │  🔧  Build   │                       │
│  └──────────────┘   └───────────────┘   └──────────────┘                       │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## 📋 Parser Tab

The existing core feature — load, filter, and inspect FIX messages.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  ⚡  [📋 Parser]  [🔬 Analysis]  [🔧 Build]                        [_] [□] [×] │
├──────────┬──────────────────────────────────────────┬──────────────────────────┤
│          │                                          │                          │
│  Spec    │  ┌─────────────────────────────────────┐ │  Tag   Value   Desc      │
│ [FIX44▼] │  │  📂  Drop FIX log here or click     │ │  ──────────────────────  │
│          │  └─────────────────────────────────────┘ │  8     FIX.4.4 BeginStr  │
│ ☑ Skip   │                                          │  9     103     BodyLen   │
│   HB     │  Time            From    To    Type      │  35    D       MsgType   │
│          │  ──────────────────────────────────────  │  49    BANZAI  Sender    │
│ [Load    │  09:30:00.187    BANZAI  EXEC  NewOrder  │  56    EXEC    Target    │
│  Sample] │  09:30:00.189 ▶  EXEC    BANZAI ER:New   │  11    1234    ClOrdID   │
│          │  09:30:00.237    EXEC    BANZAI ER:Fill   │  21    1       HandlInst │
│ 1,247    │  09:30:01.012    BANZAI  EXEC  Cancel    │  38    10000   OrderQty  │
│ messages │  09:30:01.014    EXEC    BANZAI ER:Cxl    │  40    2       OrdType   │
│          │  09:31:04.187    EXEC    BANZAI Reject    │  44    150.50  Price     │
│ 87.3%    │  09:31:04.200    BANZAI  EXEC  NewOrder  │  54    1       Side(Buy) │
│ fill rate│  ...                                     │  55    MSFT    Symbol    │
│          │                                          │  59    0       TIF       │
│ p95: 48ms│  Filter: [time ▼][sender][target][type ] │  60    20240102 TransactT│
│          │                                          │  10    062     Checksum  │
└──────────┴──────────────────────────────────────────┴──────────────────────────┘
```

### Parser Layout Rules

- **Left sidebar**: spec selector, heartbeat toggle, quick stats (message count, fill rate, p95 latency)
- **Center timeline**: chronological table, newest first, with column filters
- **Right detail panel**: full tag breakdown for the selected message, with human-readable descriptions
- Selecting a row in the timeline highlights it and populates the detail panel

---

## 🔬 Analysis Tab

Four sub-tabs. Requires a FIX log to be loaded in the Parser tab first.

```
├─────────────────────────────────────────────────────────────────────────────────┤
│  [Session Summary]  [Trade Lifecycle]  [Fill Scorecard]  [AI Query]            │
```

---

### Analysis · Session Summary

High-level dashboard. First thing a user sees when switching to Analysis.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  ⚡  [📋 Parser]  [🔬 Analysis]  [🔧 Build]                        [_] [□] [×] │
├─────────────────────────────────────────────────────────────────────────────────┤
│  [Session Summary]  [Trade Lifecycle]  [Fill Scorecard]  [AI Query]            │
├─────────────────────────────────────────────────────────────────────────────────┤
│  SESSION — 2024-01-02  ·  BANZAI → EXEC  ·  FIX 4.4  ·  08:00:00 – 17:30:22  │
│  ─────────────────────────────────────────────────────────────────────────────  │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐  ┌───────────────┐ │
│  │   1,247        │  │   87.3 %       │  │   2.3 ms       │  │   5.5 %       │ │
│  │   Total Orders │  │   Fill Rate    │  │   Avg Ack      │  │   Reject Rate │ │
│  └────────────────┘  └────────────────┘  └────────────────┘  └───────────────┘ │
│                                                                                 │
│  ┌────────────────────────────────────┐  ┌──────────────────────────────────┐  │
│  │  ⚠ Notable Events                 │  │  Top Symbols                     │  │
│  │  ─────────────────────────────    │  │  ─────────────────────────────   │  │
│  │  ⚠ 14:23:18  Latency spike        │  │  MSFT  ████████████████  340     │  │
│  │             47 orders > 100ms     │  │  AAPL  █████████████     287     │  │
│  │  ⚠ 11:42:05  SeqNum gap           │  │  SPY   ████████          201     │  │
│  │             1823 → 1831           │  │  ORCL  ██████            143     │  │
│  │  ✓ 09:31:04  3 rejects resolved   │  │  AMZN  ████              89      │  │
│  └────────────────────────────────────┘  └──────────────────────────────────┘  │
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │  🤖 AI Insight                                                          │    │
│  │  ────────────────────────────────────────────────────────────────────   │    │
│  │  Afternoon session (13:00–17:30) shows 2.3× higher reject rate than    │    │
│  │  morning. All rejects carry OrdRejReason=2 — likely intraday notional  │    │
│  │  limit exceeded. Consider monitoring cumulative exposure per session.   │    │
│  │                                          [✨ Generate Full Summary]     │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

### Analysis · Trade Lifecycle

Reconstructs the full RFQ → Quote → NewOrder → ER → Cancel chain per order,
linked by `ClOrdID` (11), `OrigClOrdID` (41), `QuoteID` (117).

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  [Session Summary]  [Trade Lifecycle]  [Fill Scorecard]  [AI Query]            │
├─────────────────────────────────────────────────────────────────────────────────┤
│  Search: [ClOrdID or Symbol    ]  [All Symbols ▼]  [All Sides ▼]  [Status ▼]  │
│  ─────────────────────────────────────────────────────────────────────────────  │
│  ClOrdID      Symbol  Side  Qty     Status      Fill%   Ack     Fill    Slip   │
│  1352157882   MSFT    Buy   10,000  ✅ FILLED   100%    2.1ms   228ms   +2bps  │
│  1352157895   ORCL    Sell  10,000  ✅ FILLED   100%    1.8ms   180ms   -1bps  │
│  1352157912   SPY     Buy   10,000  ❌ CANCELED  0%     3.2ms   —       —      │
│  ─────────────────────────────────────────────────────────────────────────────  │
│                                                                                 │
│  ▼ MSFT Buy 10,000 @ 150.40  ·  ClOrdID: 1352157882577                        │
│  ─────────────────────────────────────────────────────────────────────────────  │
│                                                                                 │
│   [RFQ: R]──14ms──[Quote: S]──230ms──[NewOrder: D]──2.1ms──[ER: New  ]        │
│                                              │                                  │
│                                              ├──────48ms──[ER: Partial]         │
│                                              │              500 @ 150.38        │
│                                              │                                  │
│                                              └─────180ms──[ER: Fill   ]         │
│                                                            9500 @ 150.40        │
│                                                                 │               │
│                                                          [CancelReq: F]──3ms──▶ │
│                                                          (arrived after fill)   │
│                                                                                 │
│  ┌──────────────────────────────────────────────────────────────────────────┐   │
│  │  🤖  "MSFT buy for 10,000 acknowledged in 2.1ms — excellent. First       │   │
│  │  partial at 150.38 is 2bps below limit — favorable. Cancel arrived       │   │
│  │  3ms after final fill and had no effect."                  [Explain ✨]  │   │
│  └──────────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

### Analysis · Fill Scorecard

Per-counterparty and per-symbol performance metrics. Sortable table + bar charts.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  [Session Summary]  [Trade Lifecycle]  [Fill Scorecard]  [AI Query]            │
├─────────────────────────────────────────────────────────────────────────────────┤
│  Group by: [Counterparty ▼]   Symbol: [All ▼]   Period: [Full Session ▼]       │
│  ─────────────────────────────────────────────────────────────────────────────  │
│  Counterparty  FillRate  Slippage  PartialFill  RejectRate  AckLat   FillLat   │
│  ──────────────────────────────────────────────────────────────────────────     │
│  EXEC          94.2%     -1.2bps   12%          2.1%        1.9ms    45ms   ✅  │
│  BANZAI        78.4%     +3.8bps   34%          9.7%        4.1ms    98ms   ⚠  │
│                                                                                 │
│  ─── EXEC ─────────────────────────────────────────────────────────────────    │
│  Fill Rate   ████████████████████████████████████████░░░░  94.2%              │
│  Slippage    ████░░░░░░░░░░░░░░░░░░  -1.2 bps (favorable)                     │
│  Ack Latency ████████░░░░░░░░░░░░░░  1.9ms  p95: 8ms                          │
│                                                                                 │
│  ─── BANZAI ───────────────────────────────────────────────────────────────    │
│  Fill Rate   ███████████████████████████████░░░░░░░░░░░░░  78.4%              │
│  Slippage    ████████████░░░░░░░░░░  +3.8 bps (unfavorable)                   │
│  Ack Latency ████████████████░░░░░░  4.1ms  p95: 22ms ⚠                       │
│                                                                                 │
│  ┌──────────────────────────────────────────────────────────────────────────┐   │
│  │  🤖  EXEC outperforms BANZAI on every metric. BANZAI reject rate climbs  │   │
│  │  from 4% to 23% for orders >5,000 shares — suggests a size-based risk   │   │
│  │  limit. Consider capping BANZAI at 4,000 shares.       [Export CSV ↓]   │   │
│  └──────────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

### Analysis · AI Query — "Chat with your logs"

Natural language interface backed by structured FIX session data.
Only aggregated metrics leave the machine by default (no raw prices or order IDs).

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  [Session Summary]  [Trade Lifecycle]  [Fill Scorecard]  [AI Query]            │
├─────────────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────────────────────┐  │
│  │  Session context loaded: 1,247 orders · 3 counterparties · FIX 4.4       │  │
│  │  🔒 Only aggregated metrics sent — no raw prices or order IDs            │  │
│  └───────────────────────────────────────────────────────────────────────────┘  │
│                                                                                 │
│  ┌───────────────────────────────────────────────────────────────────────────┐  │
│  │  You  ·  09:31                                                            │  │
│  │  Which counterparty gave me the worst fills on AAPL over 5,000 shares?   │  │
│  └───────────────────────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────────────────────┐  │
│  │  🤖 AI  ·  09:31                                                          │  │
│  │  BANZAI had the worst fills on AAPL orders above 5,000 shares:           │  │
│  │  · Average slippage: +4.2 bps vs EXEC's -0.8 bps                        │  │
│  │  · Partial fill rate: 41% vs EXEC's 9%                                   │  │
│  │  · 3 outright rejects vs 0 for EXEC in the same window                  │  │
│  │  Recommendation: route large AAPL orders exclusively to EXEC.            │  │
│  └───────────────────────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────────────────────┐  │
│  │  You  ·  09:32                                                            │  │
│  │  Find all rejects between 09:30 and 10:00                                │  │
│  └───────────────────────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────────────────────┐  │
│  │  🤖 AI  ·  09:32                                                  ●●●     │  │
│  │  Found 3 rejects in that window...                                        │  │
│  └───────────────────────────────────────────────────────────────────────────┘  │
│                                                                                 │
│  Suggested: [Worst fills on AAPL?] [Sequence gaps?] [Latency spikes?]          │
│  ┌──────────────────────────────────────────────────────────┐ ┌─────────────┐  │
│  │  Ask anything about your session...                      │ │   Send  ↵   │  │
│  └──────────────────────────────────────────────────────────┘ └─────────────┘  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

#### Example queries the AI can answer

```
"Which counterparty gave me the worst fills on AAPL orders over 5,000 shares?"
"Find all rejects between 09:30 and 10:00 and explain why they happened."
"What was my average order-to-fill latency for MSFT today?"
"Show me all orders where the cancel arrived after the fill."
"Were there any sequence number gaps and when did they happen?"
"Which symbol had the most partial fills?"
"Compare my morning vs afternoon reject rate."
```

---

## 🔧 Build Tab

Wizard-style FIX engine code generator. Outputs working Rust, Java, or Python
boilerplate + QuickFIX config. The generated project writes a FIX log to `./log/`
which can be dragged straight into the Parser tab to inspect.

---

### Build · Step 1 — Choose Type

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  ⚡  [📋 Parser]  [🔬 Analysis]  [🔧 Build]                        [_] [□] [×] │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Build a FIX Engine                                                            │
│  ─────────────────────────────────────────────────────────────────────────      │
│                                                                                 │
│  ┌───────────────────────────────┐   ┌───────────────────────────────┐         │
│  │                               │   │                               │         │
│  │   🖥  FIX Client              │   │   🌐  FIX Server              │         │
│  │       (Initiator)             │   │       (Acceptor)              │         │
│  │                               │   │                               │         │
│  │   Connects TO a venue or      │   │   Accepts connections FROM    │         │
│  │   broker. Used by buy-side    │   │   clients. Used by sell-side, │         │
│  │   OMS, algo engines, and      │   │   brokers, and exchange       │         │
│  │   test simulators.            │   │   gateways.                   │         │
│  │                               │   │                               │         │
│  │        [Select →]             │   │        [Select →]             │         │
│  └───────────────────────────────┘   └───────────────────────────────┘         │
│                                                                                 │
│  Language:  ◉ Rust   ○ Java   ○ Python   ○ QuickFIX Config only               │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

### Build · Step 2 — Configure Session

Split-panel: form on the left, live code preview on the right.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  🔧 Build  ›  FIX Client  ›  Configure                                         │
├─────────────────────────────────────┬───────────────────────────────────────────┤
│  CONFIGURATION                      │  PREVIEW                                 │
│  ─────────────────────────────────  │  ─────────────────────────────────────   │
│  Session                            │  # session.cfg                           │
│  FIX Version   [FIX.4.4        ▼]   │  [DEFAULT]                               │
│  SenderCompID  [MY_FIRM          ]   │  ConnectionType=initiator                │
│  TargetCompID  [EXEC_VENUE       ]   │  ReconnectInterval=5                     │
│  HeartbeatInt  [30               ]   │  FileStorePath=store                     │
│  Hostname      [fix.venue.com    ]   │  FileLogPath=log                         │
│  Port          [9876             ]   │                                          │
│                                     │  [SESSION]                               │
│  Messages to Handle                 │  BeginString=FIX.4.4                     │
│  ☑ NewOrderSingle (D)               │  SenderCompID=MY_FIRM                    │
│  ☑ ExecutionReport (8)              │  TargetCompID=EXEC_VENUE                 │
│  ☑ OrderCancelRequest (F)           │  HeartBtInt=30                           │
│  ☑ OrderCancelReject (9)            │  SocketConnectHost=fix.venue.com         │
│  ☑ Logon / Logout (A/5)            │  SocketConnectPort=9876                  │
│  ☑ Heartbeat (0)                    │  StartTime=00:00:00                      │
│  ☐ QuoteRequest (R)                 │  EndTime=00:00:00                        │
│  ☐ Quote (S)                        │  DataDictionary=FIX44.xml                │
│                                     │                                          │
│  Auth                               │  ──────────────────────────────────────  │
│  Username      [                 ]   │  // src/main.rs                         │
│  Password      [                 ]   │  struct MyApp;                          │
│  Reset on logon ☑                   │  impl Application for MyApp {            │
│                                     │    fn on_logon(&self, sid: &SessionID) { │
│  [← Back]                [Next →]   │      println!("Logon: {}", sid);         │
│                                     │    }                                     │
│                                     │    fn from_app(&self, msg: &Message,     │
│                                     │      sid: &SessionID) {                  │
│                                     │      // handle D, 8, F, 9...             │
│                                     │    }                                     │
│                                     │  }                                       │
└─────────────────────────────────────┴───────────────────────────────────────────┘
```

---

### Build · Step 3 — Download & Deploy

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│  🔧 Build  ›  FIX Client  ›  Download                                          │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ✅  Your FIX Client is ready                                                  │
│                                                                                 │
│  ┌──────────────────────────────────────────────────────────────────────────┐   │
│  │  📦  fix-client-MY_FIRM/                                                 │   │
│  │      ├── Cargo.toml                                                      │   │
│  │      ├── session.cfg                                                     │   │
│  │      ├── FIX44.xml                                                       │   │
│  │      └── src/                                                            │   │
│  │          ├── main.rs          ← entry point + session init               │   │
│  │          ├── app.rs           ← Application trait impl                   │   │
│  │          ├── order_sender.rs  ← NewOrderSingle builder                   │   │
│  │          └── handlers.rs     ← ExecutionReport / Cancel handlers         │   │
│  └──────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
│  Quick Start                                                                    │
│  ──────────────────────────────────────────────────────────────────────────     │
│  $ cargo run                    # start the FIX client                         │
│  $ cargo test                   # run message handler tests                    │
│                                                                                 │
│  Test with AI FIX Parser                                                        │
│  ──────────────────────────────────────────────────────────────────────────     │
│  The client writes a FIX log to ./log/ — drag it into the Parser tab to        │
│  inspect your session in real time.                                            │
│                                                                                 │
│         [📥 Download .zip]     [📋 Copy to Clipboard]     [📂 Open in IDE]    │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## Key Design Decisions

| Decision | Rationale |
|---|---|
| 3 top-level tabs | Clear separation: parse → understand → build |
| Analysis sub-tabs | Progressive disclosure — overview first, drill down second |
| Live config preview in Build | Instant feedback loop, no mental compilation needed |
| AI always optional | All offline features work without API key; AI is additive |
| Build → Parser flywheel | User builds an engine, runs it, drags log into Parser, analyzes it — no other tool closes this loop |

---

## Feature → Tab Mapping

| Feature (from premium-features.md) | Tab | Sub-tab |
|---|---|---|
| Trade Lifecycle Reconstructor | Analysis | Trade Lifecycle |
| Natural Language FIX Query | Analysis | AI Query |
| AI Reject Root Cause Analyzer | Analysis | Session Summary (inline) |
| Fill Quality & Counterparty Scorecard | Analysis | Fill Scorecard |
| Session Health AI Diagnostics | Analysis | Session Summary |
| Order Flow Pattern Recognition | Analysis | Trade Lifecycle (overlay) |
| FIX Message Validator & Debugger | Parser | Inline validation panel |
| Smart Session Summary | Analysis | Session Summary |
| Compliance & Risk Flag Engine | Analysis | Session Summary (flags) |
| Multi-Session Comparison | Analysis | New sub-tab (v2) |
| FIX Client / Server Generator | Build | Step 1–3 wizard |

---

## Tier Gating

```
FREE        Parser tab — full access, no limits
PRO         Analysis tab — Session Summary, Trade Lifecycle, Fill Scorecard
            Build tab — Config only (no code generation)
PREMIUM     Analysis tab — AI Query + AI narration on all panels
            Build tab — Full code generation (Rust / Java / Python)
ENTERPRISE  All of the above + REST API + bulk processing + webhook alerts
```
