## Feature 7: FIX Message Validator & Debugger

**Philosophy:** Robustness, simplest, cleanest

---

## What it does

Two modes in one new "Validate" tab (alongside Timeline / Lifecycle / Overview):

1. **Single-message debugger** — user pastes one raw FIX message, sees every field
   color-coded (ok / warn / error), checksum and BodyLength verified, exact fix shown.

2. **Batch validation** — validate all currently-loaded messages in parallel, show
   a summary table (message index, type, error count, first error).

---

## Research answers

### Q1: Build a Rust dictionary or use QuickFIX XML / FIX Orchestra?

**Decision: Embed compiled Rust tables in `dictionary.rs`. No external files.**

Reasons:
- App is a single binary (.dmg / .exe). No XML files to bundle or parse at startup.
- QuickFIX XML (~500 KB per version) needs an XML parser dependency and runtime
  deserialization — adds ~5ms startup latency for zero benefit at runtime.
- FIX Orchestra (JSON/XML from fixtrading.org) is comprehensive but designed for
  machine-readable contract specs, not a lookup dictionary. Overkill.
- The existing `dictionary.rs` already has ~280 tags as `match` arms — same pattern,
  zero extra dependencies.
- We only need: required-tag lists per message type, and enum-valid-value sets per tag.
  A `&'static [u16]` per MsgType is ~100 bytes of data — total dictionary < 8 KB.

Reference sources used to build the tables:
- FIX Protocol specification: fixtrading.org/standards
- QuickFIX/J dictionaries (Apache 2.0): FIX42.xml, FIX44.xml, FIX50SP2.xml
- FIX 4.4 spec (most relevant for eFX): tag 453 NoPartyIDs, tag 131 QuoteReqID, etc.

### Q2: Custom tags (≥ 5000)?

**Phase 1:** Report as `Warning::CustomTag { tag }` — "Proprietary/custom tag, not in
standard FIX spec." Do not error. Many eFX venues (Bloomberg FXGO, 360T, BidFX,
Flextrade) send custom tags in production.

**Phase 2 (future):** Add a "Load custom schema" button — user provides a JSON file:
```json
{ "5001": { "name": "InternalOrderRef", "type": "String" },
  "9999": { "name": "VenueTimestamp",   "type": "UTCTimestamp" } }
```
Stored in app config directory (`~/.config/aifixparser/custom_tags.json`).

### Q3: Performance — validate 1M messages in parallel?

**Architecture:** Validation is separated from parsing. Two tiers:

- **Field-level validation** (required tags, enum values, duplicate tags): operates on
  `Vec<FixField>` already stored in `FixMessage`. Stateless per message → trivially
  parallel with Rayon. ~50ns per message → 1M messages < 100ms on M-series.

- **Byte-level validation** (checksum tag 10, BodyLength tag 9): requires raw bytes.
  Only done in single-message debugger mode where user pastes raw text. Not done during
  batch (parser discards raw bytes after parsing — this is correct; recomputing
  checksum over 1M messages from re-encoded fields would be ~3× slower than parsing).

**Rule**: Batch validates fields only. Single-message view validates everything.

---

## Validation rules (prioritized for eFX)

### Tier 1: Always checked (both modes)

**1. Required tags present**

Header tags required on every message: `8` (BeginString), `9` (BodyLength), `35`
(MsgType), `34` (MsgSeqNum), `49` (SenderCompID), `52` (SendingTime), `56` (TargetCompID).
After `35` is known, check MsgType-specific required body tags.

Required body tags per MsgType (FIX 4.2 + 4.4):

| MsgType | Name                       | Required body tags                              |
|---------|----------------------------|-------------------------------------------------|
| `0`     | Heartbeat                  | *(none extra)*                                  |
| `A`     | Logon                      | 98 (EncryptMethod), 108 (HeartBtInt)            |
| `5`     | Logout                     | *(none extra)*                                  |
| `1`     | TestRequest                | 112 (TestReqID)                                 |
| `2`     | ResendRequest              | 7 (BeginSeqNo), 16 (EndSeqNo)                   |
| `3`     | Reject                     | 45 (RefSeqNum)                                  |
| `D`     | NewOrderSingle             | 11, 21, 38, 40, 49, 52, 54, 55, 56, 60         |
| `F`     | OrderCancelRequest         | 11, 41, 49, 52, 54, 55, 56, 60                  |
| `G`     | OrderCancelReplaceRequest  | 11, 21, 38, 40, 41, 49, 52, 54, 55, 56, 60     |
| `H`     | OrderStatusRequest         | 11, 49, 54, 55, 56                              |
| `8`     | ExecutionReport            | 6, 14, 17, 37, 39, 49, 52, 54, 55, 56, 150, 151|
| `9`     | OrderCancelReject          | 11, 37, 39, 41, 49, 52, 56, 102, 434           |
| `R`     | QuoteRequest               | 49, 52, 56, 131                                 |
| `S`     | Quote                      | 49, 52, 55, 56, 117                             |
| `Z`     | QuoteCancel                | 49, 52, 55, 56, 117                             |
| `AA`    | QuoteAcknowledgement       | 49, 52, 56, 117                                 |
| `V`     | MarketDataRequest          | 49, 52, 56, 262, 263, 264, 267, 268             |
| `W`     | MarketDataSnapshot         | 49, 52, 55, 56, 268                             |
| `X`     | MarketDataIncremental      | 49, 52, 56, 268                                 |
| `j`     | BusinessMessageReject      | 49, 52, 56, 372, 380                            |

**2. Enum value validity**

Tags with a closed set of valid values (cross-check `dictionary.rs`):

| Tag  | Name         | Valid values (FIX 4.4)                                   |
|------|--------------|----------------------------------------------------------|
| 35   | MsgType      | all msg type codes (already in `msg_type_label`)         |
| 39   | OrdStatus    | 0–9, A–E                                                 |
| 40   | OrdType      | 1–9, A–P (FX uses 1=Market, 2=Limit, D=PreviouslyQuoted)|
| 54   | Side         | 1–9                                                      |
| 59   | TimeInForce  | 0–9                                                      |
| 98   | EncryptMethod| 0–6                                                      |
| 150  | ExecType     | 0–9, A–I                                                 |
| 21   | HandlInst    | 1–3                                                      |
| 63   | SettlType    | 0–B                                                      |
| 277  | TradeCondition| A–Z (partial)                                           |

**3. Duplicate tags**

A tag appearing more than once in a message (outside a repeating group) is an error.
Exception: tags 58 (Text) can appear in header and body legitimately in some engines.

**4. BodyLength (tag 9) — single-message mode only**

BodyLength = number of bytes from the first byte after `9=<value>|` up to and
including the last `|` before `10=`. Recount from raw input, compare to tag 9 value.

**5. Checksum (tag 10) — single-message mode only**

Sum all bytes from start of message up to (but not including) the `10=` delimiter,
modulo 256. Tag 10 value must equal this as a zero-padded 3-digit string.
Show: `Checksum: found=047 expected=047 ✓` or `found=047 expected=123 ✗`.

### Tier 2: Conditional rules

**6. Conditional required tags**

| Condition                          | Then required                     |
|------------------------------------|-----------------------------------|
| `40=2` (Limit order)               | tag 44 (Price)                    |
| `40=3` or `40=4` (Stop/StopLimit)  | tag 99 (StopPx)                   |
| `150=F` (Trade/Fill)               | tags 31 (LastPx), 32 (LastQty)    |
| `150=G` or `150=H` (Correct/Cancel)| tag 19 (ExecRefID)                |
| `59=6` (GTD)                       | tag 432 (ExpireDate)              |
| `35=D` and `40=D` (Previously Quoted NOS in eFX RFQ) | tag 117 (QuoteID) |

**7. Consistency rules**

- `LeavesQty(151) + CumQty(14)` should equal `OrderQty(38)` when `ExecType=F`.
- `OrdStatus(39)=2` (Filled) → `LeavesQty(151)` must be `0`.
- `MsgSeqNum(34)` should be numeric and > 0.
- `SendingTime(52)` should be parseable as UTCTimestamp (YYYYMMDD-HH:MM:SS[.sss]).

**8. Repeating group delimiter order**

Group delimiter tag must be the first tag in each group instance. Known groups:

| Counter tag | Delimiter tag | Group name         |
|-------------|---------------|--------------------|
| 453         | 448           | NoPartyIDs         |
| 268         | 269           | NoMDEntries        |
| 267         | 269           | NoMDEntryTypes     |
| 78          | 79            | NoAllocs           |
| 382         | 375           | NoMiscFees         |
| 146         | 55            | NoRelatedSym (QuoteRequest) |

**9. FIX version consistency**

Tags introduced after FIX 4.2 should not appear in a `8=FIX.4.2` message:

| Tag(s)        | Introduced in |
|---------------|---------------|
| 453, 448, 452 | FIX 4.4       |
| 263, 264, 267 | FIX 4.2       |
| 150           | FIX 4.2       |
| 571–574       | FIX 4.4       |
| 702–728       | FIX 4.4       |

---

## Architecture

### New file: `src/validator.rs`

Pure validation logic. No UI dependencies. Operates on `&FixMessage` for field-level
rules and `&[u8]` for byte-level rules.

```rust
pub enum Severity { Error, Warning, Info }

pub struct Issue {
    pub severity: Severity,
    pub tag:      Option<u16>,   // which tag caused it (None = structural)
    pub code:     &'static str,  // machine-readable code: "MISSING_TAG", "INVALID_ENUM", etc.
    pub message:  String,        // human-readable, actionable
    pub fix_hint: Option<String>,// what the value should be
}

pub struct ValidationReport {
    pub issues:             Vec<Issue>,
    pub checksum_ok:        Option<bool>,   // None if not checked
    pub checksum_expected:  Option<String>,
    pub body_length_ok:     Option<bool>,
    pub body_length_delta:  Option<i64>,    // found - expected
}

/// Field-level validation only — works on parsed FixMessage.
/// Used for batch validation of 1M messages.
pub fn validate_fields(msg: &FixMessage) -> ValidationReport { ... }

/// Full validation including checksum and BodyLength.
/// Used only in the single-message debugger.
pub fn validate_raw(raw: &[u8]) -> (FixMessage, ValidationReport) { ... }

/// Batch validate a slice in parallel (Rayon).
/// Returns one report per message.
pub fn validate_batch(msgs: &[FixMessage]) -> Vec<ValidationReport> {
    msgs.par_iter().map(validate_fields).collect()
}
```

### New file: `src/components/validator.rs`

Dioxus component for the Validate tab. Two sub-views:

**Single message debugger layout:**
```
┌─────────────────────────────────────────────────────┐
│  Raw FIX input  [textarea]              [Validate]  │
├─────────────────────────────────────────────────────┤
│  Field table (tag | name | value | status)          │
│  8   BeginString  FIX.4.4   ✓                       │
│  9   BodyLength   178       ✓  (counted: 178)       │
│  35  MsgType      D         ✓  NewOrderSingle        │
│  54  Side         3         ✗  INVALID: expected 1/2 │
│  ...                                                 │
├─────────────────────────────────────────────────────┤
│  Checksum:  10=047  expected=047  ✓                  │
│  BodyLength: 9=178  counted=178   ✓                  │
│  Issues: 1 error, 0 warnings                         │
└─────────────────────────────────────────────────────┘
```

**Batch summary layout (when messages are loaded):**
```
  [Validate All N messages]    Errors: 12  Warnings: 34
  ┌──────┬──────────┬───────┬─────────────────────────┐
  │  #   │ MsgType  │ Issues│ First error              │
  ├──────┼──────────┼───────┼─────────────────────────┤
  │  47  │ NOS (D)  │  1 ✗  │ Missing tag 60 (TransactTime) │
  │  103 │ ER (8)   │  2 ✗  │ Missing tag 151 (LeavesQty)   │
  └──────┴──────────┴───────┴─────────────────────────┘
```

Clicking a row in batch view populates the single-message debugger with that message.

### Integration points

- `app.rs`: add `ViewMode::Validator` variant, add "Validate" tab button.
- `lib.rs`: add `pub mod validator;`
- No new crate dependencies needed — uses only existing `FixMessage`, `FixField`,
  `dictionary.rs` functions. No XML parser, no serde for the validation logic.

---

## Implementation plan (ordered by value)

### Step 1 — `src/validator.rs` core (no UI)
- Define `Issue`, `Severity`, `ValidationReport` structs.
- Implement `validate_fields()`: header required tags, MsgType-specific required tags,
  enum value checks for tags 35/39/40/54/59/98/150/21.
- Implement `validate_raw()`: header + field validation + checksum + BodyLength.
- Unit tests: valid NOS message (zero issues), missing ClOrdID, bad Side enum,
  bad checksum, bad BodyLength.

### Step 2 — Single-message UI (`src/components/validator.rs`)
- Textarea input + Validate button.
- Field table with per-field color coding: amber = warn, red = error, green = ok.
- Checksum and BodyLength summary row.
- Issue list below the table.

### Step 3 — Batch validation UI
- "Validate All" button in the Validate tab.
- Summary table showing only messages with issues.
- Click-to-drill: clicking a row populates single-message view with raw text of
  that message (need to reconstruct raw from fields — pipe-delimited).

### Step 4 — Conditional rules and consistency checks
- Conditional required tags (Price when Limit, etc.)
- ExecReport LeavesQty+CumQty=OrderQty consistency.
- Repeating group delimiter order.

### Step 5 — FIX version consistency
- Check `8=FIX.4.x` BeginString and flag tags introduced after that version.

### Step 6 (future) — Custom tag schema upload
- Load JSON file, merge into in-memory tag dictionary.
- Suppress `CustomTag` warnings for known custom tags.

---

## Key eFX-specific notes

**Why eFX is stricter than equity FIX:**
- RFQ workflows use QuoteRequest (R) → Quote (S) → NOS (D, OrdType=D) → ER (8).
  QuoteID (117) must thread through all messages. Missing it breaks the audit trail.
- SettlType (63) is critical in FX — "Regular" vs "Cash" vs "T+n" changes the
  settlement and PnL date. Wrong value is a legal/operational risk.
- Currency (15) must be present on NOS/ER for FX — the symbol EURUSD doesn't uniquely
  identify the notional currency (EUR vs USD leg).
- FX venues (Bloomberg FXGO, 360T, Refinitiv FXall, BidFX, Flextrade) all use FIX 4.4
  but add custom tags (5000+). Our `Warning::CustomTag` handles this gracefully.
- Session-level tags (34 MsgSeqNum) are critical for reconciliation on ECNs.

**Enums to add to dictionary.rs for validation:**

`tag 63` SettlType: `0`=Regular, `1`=Cash, `2`=Next Day, `3`=T+2, `4`=T+3, `5`=T+4,
`6`=Future, `7`=When Issued, `8`=Sellers Option, `9`=T+5, `B`=Broken Date.

`tag 39` OrdStatus: `0`–`9`, `A`–`E` (already in dictionary.rs).

`tag 150` ExecType: `0`–`9`, `A`–`I` (already in dictionary.rs).

---

## What we do NOT do (scope boundary)

- We do not re-implement the FIX engine (session state, sequence gap detection) —
  that is the Session Analysis / Overview panel's job.
- We do not parse FIX Orchestra XML at runtime.
- We do not validate FIX 5.0 FIXT transport vs. application version split in Phase 1.
  (The tag structure is identical to 4.4 for eFX messages; difference is BeginString.)
- We do not auto-fix messages — we show what is wrong and what the correct value is,
  but the user decides whether to change it.



Rule code tags: Every issue now shows a labeled badge before the message text:

Error Rule 1 — MISSING_HEADER_TAG
Error Rule 2 — MISSING_REQUIRED_TAG
Error Rule 3 — INVALID_ENUM
Error Rule 4 — CONDITIONAL_TAG_MISSING
Error Rule 5 — CONSISTENCY_FILL_QTY
Error Rule 6 — CONSISTENCY_FILLED_LEAVES
Error Rule 7 — INVALID_SEQNUM
Warning Rule 8 — DUPLICATE_TAG
Warning Rule 9 — CUSTOM_TAG
Warning Rule 10 — EXTENDED_TAG
Warning Rule 11 — VERSION_VIOLATION
Error Rule 12 — MISSING_CHECKSUM
Error Rule 13 — CHECKSUM_MISMATCH
Error Rule 14 — BODY_LENGTH_MISMATCH