# FIX Studio — Premium Feature Roadmap

> Research document for AI-powered premium features.
> Based on the full RFQ → Quote → Order → Execution Report → Cancel lifecycle available in FIX session data.

---

## Context: What data we have

Every FIX session log contains a rich, linked chain of messages:

| MsgType | Tag 35 | Description |
|---|---|---|
| Quote Request | R | Client asks for a price (RFQ) |
| Quote | S | Market maker responds with bid/ask |
| New Order Single | D | Order placed |
| Execution Report | 8 | Order status: New / Partial Fill / Fill / Reject / Canceled |
| Order Cancel Request | F | Cancel request |
| Order Cancel Reject | 9 | Cancel rejected |
| Session Reject | 3 | Protocol-level rejection |
| Heartbeat | 0 | Session keepalive |
| Logon / Logout | A / 5 | Session lifecycle |

Key linking fields across messages:
- `ClOrdID` (11) — links Order → Cancel → ER chain
- `OrigClOrdID` (41) — links cancel to original order
- `QuoteID` (117) — links RFQ → Quote → Order
- `ExecID` (17) — unique per ER
- `OrderID` (37) — venue-assigned order identifier
- `SenderCompID` / `TargetCompID` — session / counterparty identity

---

## Feature 1: Trade Lifecycle Reconstructor

**Category:** Pro tier
**AI involvement:** Optional (pure Rust first, GPT narration as add-on)

### What it does

Automatically groups all related FIX messages into a single visual timeline per order, linking them by `ClOrdID`, `QuoteID`, and `OrigClOrdID`. Shows each state transition with the latency between hops.

```
[RFQ]  →  14ms  →  [Quote]  →  230ms  →  [NewOrder]  →  2.1ms  →  [ER: New]
                                                          →  48ms  →  [ER: Partial 500@150.38]
                                                          →  180ms →  [ER: Fill 9500@150.40]
                                                                    └─ [CancelReq] →  3ms  →  [ER: Canceled] (too late)
```

### GPT narration layer

After reconstructing the chain, send a compact JSON summary (no PII by default) to GPT:

```json
{
  "symbol": "MSFT",
  "side": "Buy",
  "qty": 10000,
  "limit_px": 150.40,
  "rfq_to_quote_ms": 14,
  "order_to_ack_ms": 2.1,
  "fills": [
    { "qty": 500, "px": 150.38, "latency_ms": 48 },
    { "qty": 9500, "px": 150.40, "latency_ms": 228 }
  ],
  "cancel_attempted": true,
  "cancel_succeeded": false
}
```

GPT output example:
> *"This MSFT buy for 10,000 shares was acknowledged in 2.1ms (excellent). The first partial fill of 500 shares at 150.38 was 2 bps below your limit — favorable. The remaining 9,500 shares filled at 150.40 after a 180ms gap, which is unusually long and may reflect a matching engine pause or liquidity drought at that price level. The cancel request arrived after the final fill and had no effect."*

### Research questions
- How to handle messages that span multiple log files / sessions
- How to deal with broker-assigned `OrderID` (37) that differs from client `ClOrdID` (11)
- Handling amends (`35=G` Order Cancel/Replace): how to show price/qty changes in the chain
- Visualisation: should the chain be a horizontal timeline, a tree, or a vertical flow?

---

## Feature 2: Natural Language FIX Query — "Chat with your logs"

**Category:** Premium tier
**AI involvement:** Core (GPT-4o / Claude function calling)

### What it does

After loading a FIX session, the user types plain-English questions and gets answers backed by the actual parsed data.

### Example queries

```
"Which counterparty gave me the worst fills on AAPL orders over 5,000 shares?"
"Find all rejects between 09:30 and 10:00 and explain why they happened."
"What was my average order-to-fill latency for MSFT today?"
"Did I have any potential wash trades?"
"Show me all orders where the cancel arrived after the fill."
"Compare my reject rate this week vs last week."
"Which symbol had the most partial fills?"
"Were there any sequence number gaps and when did they happen?"
```

### Architecture

1. App extracts structured metrics from parsed messages into a compact JSON context (not raw log text)
2. User query + context JSON sent to GPT with a system prompt that describes the FIX schema
3. GPT either answers directly from context or requests a specific filter/aggregation via function call
4. App executes the function (e.g., `filter_by_symbol("AAPL")`, `compute_slippage()`) and returns result
5. GPT composes final answer in plain English

### Privacy model
- Default: only aggregate metrics sent (counts, averages, latencies) — no actual prices or order IDs
- Opt-in: send full message context for richer analysis
- All processing done locally first; only a summary leaves the machine

### Research questions
- Best GPT function-calling schema for FIX data queries
- How to handle queries that require multi-step aggregation
- Whether Claude or GPT-4o gives better domain-specific FIX answers
- Token budget management for large sessions (summarise before sending)
- Caching: don't re-send the same context for follow-up questions

---

## Feature 3: AI Reject Root Cause Analyzer

**Category:** Pro tier
**AI involvement:** Core

### What it does

Whenever an order is rejected (`OrdStatus=8`, tag 39=8) or a session reject (`35=3`) occurs, the app automatically:

1. Collects the full context: the rejected message, the session state at that moment, surrounding messages (±5 messages), any previous similar rejects
2. Sends a compact summary to GPT
3. Returns a plain-English diagnosis with an actionable fix

### Example output

**Input:** `35=8|39=8|58=Invalid price|11=ORD_1234|44=102.15|55=MSFT|52=20240102-09:31:04.187`

**AI output:**
> *"This NewOrderSingle was rejected with 'Invalid price' at 09:31:04.187. Your submitted price (44=102.15) is likely outside the venue's acceptable price band. The prior Heartbeat at 09:31:03.950 shows your market data was 237ms stale at time of submission — this staleness gap is a common cause of price-band rejections. Suggested fix: add a pre-flight staleness check; discard orders if market data age exceeds your venue's price collar window (typically 100–500ms)."*

### Reject pattern library (built-in, no API needed)

Common reject reasons the app can explain locally without GPT:
- `58=Invalid price` → price band / stale quotes
- `58=Unknown symbol` → routing / symbology mismatch
- `58=Duplicate ClOrdID` → order management bug
- `58=Quantity too small/large` → lot-size / notional limit
- `103=2` (OrdRejReason = Broker option) → catch-all, needs context
- `45=N` in Session Reject → invalid sequence number

### Research questions
- Build a reject code taxonomy covering FIX 4.2, 4.4, 5.0 standard reject reasons
- How to detect that multiple rejects share a root cause (e.g., all caused by the same stale price feed)
- Whether to offer "explain this reject" on-demand vs. automatic analysis on load

---

## Feature 4: Fill Quality & Counterparty Scorecard

**Category:** Pro tier
**AI involvement:** Commentary layer (GPT optional)

### What it does

Extracts all Execution Reports with fills (`150=F` or `150=1` or `150=2`) and computes a per-counterparty / per-symbol scorecard:

| Metric | Formula |
|---|---|
| Fill rate | Filled qty / Total ordered qty |
| Slippage (bps) | `(AvgPx - LimitPx) / LimitPx * 10000` (for buys) |
| Partial fill rate | Orders with >1 ER fill / Total orders |
| Reject rate | Rejected orders / Total orders |
| Ack latency | Time: NewOrder → first ER |
| Fill latency | Time: NewOrder → final fill ER |
| Cancel success rate | Successful cancels / Cancel attempts |

### Display

A sortable table + bar chart (per counterparty, per symbol, per time-of-day bucket).

### GPT commentary

Send aggregated scorecard to GPT, receive insights:
> *"EXEC outperforms BANZAI on every metric for orders above 5,000 shares. BANZAI's reject rate climbs from 4% to 23% for large orders, suggesting a size-based risk limit at that venue. Consider capping BANZAI order size at 4,000 shares or adjusting your smart order router logic to prefer EXEC for large fills."*

### Research questions
- How to attribute fills when orders span multiple ERs with different `LastPx` values
- Handling `AvgPx` (6) vs reconstructing from individual `LastPx` (31) / `LastQty` (32) fills
- Time-of-day bucketing: should it be 30-min, 1-hour, or user-defined?
- Export to CSV / PDF for compliance / reporting use

---

## Feature 5: Session Health AI Diagnostics

**Category:** Pro tier
**AI involvement:** Pattern narration

### What it does

Scans the full session log for technical issues and explains them in business terms.

### Detections (rule-based, no API needed)

| Pattern | Detection |
|---|---|
| Heartbeat gaps | Missing `35=0` within `HeartBtInt` (108) × 1.5 |
| Sequence number gaps | Jump in `MsgSeqNum` (34) without `35=2` Resend Request |
| Excessive resend requests | `35=2` count > threshold |
| Reconnects | Multiple `35=A` Logon messages in one session |
| High message rate bursts | >N messages/second → potential rate limit event |
| Late cancel (cancel after fill) | `35=F` timestamp > final fill ER timestamp |
| Rejected cancel | `35=9` Order Cancel Reject |

### GPT narration

For each detected event, generate a 2–3 sentence explanation connecting the technical FIX event to the business impact:
> *"Three heartbeat gaps occurred at 10:15, 11:42, and 14:23. All three correlate with 'Connection timeout' reject messages 2–3 seconds later, and each was followed by a Logon (Resend). This pattern is consistent with intermittent TCP keepalive failure between your gateway and the execution venue. Check MTU settings or firewall idle-timeout configuration."*

### Research questions
- Whether to show diagnostics as a separate panel vs. inline annotations on the timeline
- How to compute baseline "normal" for a session (first 5 minutes?) vs. detecting deviations
- Alert thresholds: should they be configurable per user / per venue?

---

## Feature 6: Order Flow Pattern Recognition

**Category:** Premium tier
**AI involvement:** Classification + narration

### What it does

Detects known algorithmic execution patterns in order flow — either in your own orders or in counterparty flow.

### Patterns to detect

| Pattern | Signal |
|---|---|
| TWAP | Regular time intervals between orders (e.g., every 30s ± 5s), consistent qty |
| VWAP | Qty proportional to volume (requires market data feed or volume field) |
| Iceberg | Repeated fills of identical qty at same price from same side |
| Momentum | Cluster of orders all in same direction after a price move |
| Spoofing signal | Large order cancelled within 100ms, no fill, price moved |
| Sub-second cancel | `35=F` submitted < 200ms after `35=D` on same `ClOrdID` |
| Layering | Multiple orders at different price levels, all cancelled together |

### GPT narration

> *"The order flow from COUNTERPARTY_A between 09:30–11:00 shows 30-second periodic intervals with quantities following a volume-weighted distribution. This is consistent with a VWAP execution algorithm participating at approximately 8% of market volume. Expect continued participation through the afternoon session at similar intervals."*

### Research questions
- What additional fields are needed beyond the current `FixMessage` model (e.g., `OrdType`, `TimeInForce`, `ExecInst`)
- Whether pattern detection should be purely structural (from FIX data alone) or require external price/volume reference data
- Legal / compliance sensitivity: spoofing detection output needs careful wording to avoid false accusations

---

## Feature 7: FIX Message Validator & Debugger

**Category:** Free (with AI explanation as Pro)

### What it does

Validates any pasted FIX message against the spec and provides precise, actionable error messages.

### Validations (rule-based)

- Required tags present for each MsgType (e.g., `35=D` requires 49, 56, 11, 54, 38, 40, 44, 60)
- Valid enum values per tag and version (e.g., `54` Side: 1/2/5/6 for FIX 4.4)
- Checksum (10) verification with correct value shown
- BodyLength (9) verification
- Sequence of tag groups (e.g., repeating groups must have delimiter tag first)
- FIX version consistency (tags introduced in 4.4 shouldn't appear in a 4.2 message)

### AI explanation layer

For non-obvious errors, GPT explains:
> *"Tag 60 (TransactTime) is required for all NewOrderSingle messages in FIX 4.4. Its absence will cause a Session Reject (35=3) from most execution venues. Format required: YYYYMMDD-HH:MM:SS or YYYYMMDD-HH:MM:SS.sss for millisecond precision."*

### Research questions
- Whether to build a full FIX data dictionary in Rust or use an existing open-source spec (FIX Orchestra / QuickFIX XML dictionaries)
- How to handle custom tags (tag >= 5000) — user-defined schema upload?
- Performance: validating 1M messages — should validation run in parallel with parsing?

---

## Feature 8: Smart Session Summary

**Category:** Pro tier
**AI involvement:** Core

### What it does

After loading any FIX log, automatically generates a one-page executive summary of the session using GPT.

### Output format

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

AI insight:
  Your afternoon session (13:00–17:30) shows 2.3× higher reject
  rate than the morning. All afternoon rejects carry OrdRejReason=2
  (Broker option), which at this venue typically indicates you
  exceeded the venue's intraday notional limit. Consider monitoring
  cumulative notional exposure across your session.
```

### Research questions
- How to keep the summary generation fast enough to not feel slow (stream GPT output token-by-token)
- Privacy model: what session metrics are safe to send vs. require opt-in
- Whether to support scheduled summaries (e.g., auto-generate EOD summary when log file is detected)

---

## Feature 9: Compliance & Risk Flag Engine

**Category:** Premium / Enterprise tier
**AI involvement:** Explanation layer

### What it does

Automatically scans order flow for patterns that may trigger compliance concerns, flagging them for review with plain-English explanations.

### Flag types

| Flag | Trigger | Risk |
|---|---|---|
| Potential wash trade | Same symbol, opposite sides, same price ± threshold, < 1s apart | Regulatory / market integrity |
| Sub-second cancel | Cancel sent < 100ms after order, no fill | Spoofing indicator |
| Excessive cancel rate | Cancel rate > 80% on a symbol in a time window | Quote stuffing indicator |
| Large order burst | >50 orders/second on same symbol | Rate limit / exchange rule |
| Self-cross | Two orders from same session that would cross each other | Wash trade risk |
| Late booking | `TransactTime` (60) vs. `SendingTime` (52) gap > threshold | Timestamp manipulation |

### Important: output framing

These are **indicators for review**, not determinations of wrongdoing. GPT output must consistently frame results as *"this pattern warrants review"* rather than *"this is spoofing"*.

### Research questions
- What regulatory thresholds apply (MiFID II, SEC Rule 15c3-5, etc.) — jurisdiction-specific?
- How to avoid false positives on legitimate market-making strategies (high cancel rates are normal for MMs)
- Should flags be exportable as a compliance report (PDF)?
- Does offering this feature create any legal liability?

---

## Feature 10: Multi-Session Comparison

**Category:** Premium tier
**AI involvement:** Insight generation

### What it does

Load two or more FIX session logs side-by-side and compare performance across dimensions.

### Use cases

- **A/B test** two smart order router configurations
- **Before vs. after** a venue configuration change
- **Monday vs. Friday** latency comparison
- **Venue A vs. Venue B** fill quality for the same symbol

### Metrics compared

Fill rate, avg slippage, reject rate, latency distribution (p50/p95/p99), cancel rate, session stability (reconnects, gaps)

### GPT insight

> *"Your Tuesday session with the new routing logic shows 31% lower reject rate and 12% better average slippage vs. Monday's baseline. However, p99 fill latency increased from 45ms to 78ms — the tail latency degradation may offset the slippage improvement for time-sensitive strategies. Recommend further testing during high-volatility opens."*

### Research questions
- UI: how to visualise two timelines simultaneously (split view vs. overlay vs. diff)
- How to align sessions with different absolute timestamps for comparison
- File format: support compressed logs (.gz, .zip) for large multi-day comparisons

---

## Proposed Monetisation Tiers

```
┌─────────────────────────────────────────────────────────┐
│  FREE                                                   │
│  • Parse, view, filter FIX messages                     │
│  • Timeline + detail panel                              │
│  • Basic FIX tag dictionary                             │
│  • Local file loading (no size limit)                   │
├─────────────────────────────────────────────────────────┤
│  PRO  —  $19 / month                                    │
│  • Trade lifecycle reconstruction (Feature 1)           │
│  • Fill quality & counterparty scorecard (Feature 4)    │
│  • Session health diagnostics (Feature 5)               │
│  • Smart session summary (Feature 8)                    │
│  • FIX message validator (Feature 7)                    │
│  • Export: CSV, JSON                                    │
├─────────────────────────────────────────────────────────┤
│  PREMIUM  —  $59 / month                                │
│  • Everything in Pro                                    │
│  • Natural language query — "chat with your logs" (F2)  │
│  • AI reject root cause analyzer (Feature 3)            │
│  • Order flow pattern recognition (Feature 6)           │
│  • Multi-session comparison (Feature 10)                │
│  • Compliance flag engine (Feature 9)                   │
│  • GPT narration on all analytics                       │
├─────────────────────────────────────────────────────────┤
│  ENTERPRISE  —  custom pricing                          │
│  • Everything in Premium                                │
│  • REST API for programmatic access                     │
│  • Bulk log analysis (batch processing)                 │
│  • Team sharing & role-based access                     │
│  • Webhook alerts (latency spikes, reject bursts)       │
│  • Custom tag dictionary upload                         │
│  • On-premises deployment option                        │
└─────────────────────────────────────────────────────────┘
```

---

## Implementation Priority

| Priority | Feature | Effort | Revenue Impact | AI Required? |
|---|---|---|---|---|
| 1 | Trade Lifecycle Reconstruction | Medium | High | No (Rust only) |
| 2 | Fill Quality Scorecard | Low | High | No |
| 3 | Session Health Diagnostics | Low | Medium | No |
| 4 | AI Reject Root Cause Analyzer | Medium | High | Yes (GPT) |
| 5 | Smart Session Summary | Medium | High | Yes (GPT) |
| 6 | FIX Message Validator | Medium | Medium | Partial |
| 7 | Natural Language Query | High | Very High | Yes (GPT) |
| 8 | Counterparty Scorecard | Low | High | No |
| 9 | Order Flow Pattern Recognition | High | Medium | Yes (GPT) |
| 10 | Multi-Session Comparison | High | Medium | Partial |
| 11 | Compliance Flag Engine | Very High | High | Yes (GPT) |

---

## Technical Foundation Needed

Before building any AI features, these Rust data structures are needed:

```rust
/// A single reconstructed order lifecycle.
struct OrderLifecycle {
    cl_ord_id: CompactString,
    orig_cl_ord_ids: Vec<CompactString>,  // amend chain
    quote_id: Option<CompactString>,
    rfq_idx: Option<usize>,              // index into messages[]
    quote_idx: Option<usize>,
    new_order_idx: usize,
    exec_reports: Vec<usize>,            // indices, ordered by time
    cancel_request_idx: Option<usize>,
    cancel_reject_idx: Option<usize>,
    // Computed metrics
    ack_latency_us: Option<u64>,         // NewOrder → first ER
    fill_latency_us: Option<u64>,        // NewOrder → final fill ER
    total_filled_qty: f64,
    avg_fill_px: f64,
    is_fully_filled: bool,
    is_canceled: bool,
    is_rejected: bool,
}

/// Per-counterparty / per-symbol aggregated metrics.
struct FillMetrics {
    counterparty: CompactString,
    symbol: CompactString,
    order_count: usize,
    fill_rate: f64,
    avg_slippage_bps: f64,
    avg_ack_latency_us: u64,
    p95_ack_latency_us: u64,
    reject_rate: f64,
    cancel_success_rate: f64,
}
```

The lifecycle reconstruction runs once after parsing (O(n) with a HashMap on `ClOrdID`) and caches results — all subsequent analytics derive from `Vec<OrderLifecycle>`.

---

## Notes & Open Questions

- **GPT model choice**: GPT-4o for balance of cost/quality; Claude Sonnet 4.6 as alternative (better instruction following for structured tasks). Should be user-configurable — user brings their own API key.
- **Privacy by default**: never send raw FIX field values (prices, quantities, counterparty IDs) without explicit user consent. Send only structural metadata and timing data by default.
- **Offline mode**: all rule-based features (lifecycle reconstruction, validator, scorecard, health diagnostics) must work 100% offline. AI features degrade gracefully to "bring your API key."
- **Token cost estimation**: a 1M-message session summary sent as aggregated metrics should use <2,000 tokens → ~$0.002 per query at current GPT-4o pricing. Feasible with $19/month tier.
- **FIX Orchestra**: the FIX Trading Community publishes machine-readable FIX specs at [https://github.com/FIXTradingCommunity/fix-orchestra](https://github.com/FIXTradingCommunity/fix-orchestra) — could be used to drive the validator and tag dictionary automatically across versions.
