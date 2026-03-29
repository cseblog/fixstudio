# AI FIX Parser — Architecture

**Version:** 2.0.0
**Stack:** Rust 2021 edition, Dioxus 0.7.2 (desktop), Rayon, NEON/AVX2 SIMD
**Platform targets:** macOS (aarch64/x86_64), Windows (x86_64), Linux (x86_64)

---

## What It Does

AI FIX Parser is a native desktop application for loading, parsing, and analyzing FIX protocol
log files. It targets files with millions of messages — the headline goal is **<100ms parse time
for 1M messages** on a developer laptop.

Users can:

- Load `.fix` / `.log` / `.txt` files or entire folders
- Inspect any message's fields in Table, Raw, or JSON view
- Filter the timeline by time range, sender/target, message type, ClOrdID, or any field value
- Validate messages against FIX 4.x rules (missing tags, bad enums, checksum errors)
- Analyze session health: sequence gaps, heartbeat gaps, reconnects, rate spikes
- Score fill quality per counterparty and symbol
- Export filtered messages to CSV

---

## Subsystem Map

```
┌───────────────────────────────────────────────────────────────────────┐
│                            Desktop App (Dioxus)                        │
│                                                                         │
│  ┌──────────────┐  ┌─────────────────────────────────────────────────┐ │
│  │ Timeline     │  │ Right Panel (one of)                             │ │
│  │ (filter +    │  │  ┌─────────┐  ┌──────────┐  ┌────────────────┐  │ │
│  │  scroll)     │  │  │ Detail  │  │Validator │  │  Lifecycle /   │  │ │
│  │              │  │  │ Table / │  │ report   │  │  Overview /    │  │ │
│  │  click row   │  │  │ Raw /   │  │ per-msg  │  │  Premium panel │  │ │
│  │     │        │  │  │ JSON    │  │ debugger │  │ (Pro only)     │  │ │
│  └──────────────┘  │  └─────────┘  └──────────┘  └────────────────┘  │ │
│         │          └─────────────────────────────────────────────────┘ │
│         │                                                               │
│  app.rs signals: Signal<Vec<FixMessage>>, Signal<Option<usize>>, ...   │
└──────────────────────────────┬────────────────────────────────────────┘
                               │  Vec<FixMessage>
                ┌──────────────┴──────────────────────┐
                │  Analysis Layer (Rust, parallel)     │
                │                                      │
                │  ┌─────────────┐  ┌───────────────┐ │
                │  │  Validator  │  │ Session Health │ │
                │  │  (6 pass    │  │ (7 rules,      │ │
                │  │   types,    │  │  sequence/     │ │
                │  │   parallel) │  │  heartbeat/    │ │
                │  └─────────────┘  │  reconnects)   │ │
                │  ┌─────────────┐  └───────────────┘ │
                │  │  Session    │  ┌───────────────┐ │
                │  │  Summary    │  │ Fill Quality  │ │
                │  │  (stats,    │  │ (scorecard    │ │
                │  │  latency)   │  │  per cp/sym)  │ │
                │  └─────────────┘  └───────────────┘ │
                └──────────────┬──────────────────────┘
                               │  &[u8]
                ┌──────────────┴──────────────────────┐
                │  Parser  (src/parser.rs)             │
                │                                      │
                │  boundary scan ──► par_windows(2)    │
                │  ┌──────────┐  ┌──────────────────┐  │
                │  │ memmem   │  │ simd_parse_avx2  │  │
                │  │ "8=FIX"  │  │ simd_parse_neon  │  │
                │  │ parallel │  │ simd_parse_scalar│  │
                │  │ ≥2 MB    │  │ apply_token()    │  │
                │  └──────────┘  └──────────────────┘  │
                └──────────────┬──────────────────────┘
                               │  &[u8] (zero-copy)
                ┌──────────────┴──────────────────────┐
                │  Loader  (src/loader.rs)             │
                │                                      │
                │  rfd::AsyncFileDialog                │
                │  memmap2::Mmap  (or read fallback)   │
                │  SOH/pipe detection (first 4 KB)     │
                └─────────────────────────────────────┘
```

---

## Technologies

| Subsystem | Key crate(s) | Notes |
|---|---|---|
| UI | `dioxus 0.7.2`, `dioxus-desktop 0.7.2` | Reactive Rust UI, renders native WebView |
| Windowing | `tao 0.34` | Cross-platform window abstraction under dioxus-desktop |
| Charts | ECharts 5.5.1 (CDN, JS) | Lifecycle flow, overview stats, health charts |
| File dialogs | `rfd 0.15` | Async native file/folder picker |
| Parsing SIMD | `std::arch::{x86_64, aarch64}` | AVX2 (x86) and NEON (ARM) intrinsics, both `unsafe` |
| SIMD search | `memchr 2` | Vectorized `memmem` for "8=FIX" boundary scan |
| Parallelism | `rayon 1.10` | `par_windows`, `par_iter`, `into_par_iter` |
| Allocator | `mimalloc 0.1` | Thread-local arena allocator, replaces system malloc |
| File I/O | `memmap2 0.9` | Zero-copy memory-mapped file access |
| Strings | `compact_str 0.8` | Inline string storage ≤23 bytes — no heap for short FIX values |
| Serialization | `serde 1`, `serde_json 1` | License JSON, export, Whop API response |
| HTTP | `reqwest 0.12` (rustls) | License validation against Whop API |
| Async | `tokio 1` (rt, sync) | Dioxus async `spawn()` for file dialogs and license checks |
| Export | stdlib `Write` | CSV export (no external crate) |
| Build (Win) | `winres 0.1` | Embed `.ico` and DPI manifest in Windows binary |

---

## Data Flow: File Load → Parse → Display

```
User picks file
       │
       ▼
rfd::AsyncFileDialog (async, tokio spawn)
       │
       ▼
std::fs::File::open + memmap2::Mmap::map    ← zero-copy preferred
  (fallback: file.read() into Vec<u8>)
       │
       ▼
First 4 KB scanned for 0x01 (SOH) to set delimiter badge
       │
       ▼
parse_all_simd_bytes(&[u8])                 ← HOT PATH
  │
  ├─ message_start_offsets(&[u8]) → Vec<u32>
  │   small files (<2MB):  serial memmem for "8=FIX"
  │   large files (≥2MB):  parallel chunk scan
  │                         each thread scans own range + 4-byte overlap,
  │                         keeps only markers in [own_start, own_end)
  │                         flatten in index order → already sorted
  │
  ├─ offsets.push(input.len())     ← sentinel
  │
  └─ offsets.par_windows(2)
       .map(|w| parse_single_simd(&input[w[0]..w[1]]))
       .collect::<Vec<FixMessage>>()

parse_single_simd(&[u8]) → FixMessage
  │  arena = raw.to_vec()       (one memcpy, ~190 bytes avg)
  │  fill_message(raw, &mut msg)
  │    ↓ dispatch
  │    simd_parse_avx2  (x86_64 + AVX2)   32 bytes/iter
  │    simd_parse_neon  (aarch64)          16 bytes/iter
  │    simd_parse_scalar (fallback)         1 byte/iter
  │
  │  apply_token(raw, start, end, msg) called per field
  │    tag_to_u16(bytes)      → u16 (branch tree, no alloc)
  │    value_start = start + eq_index + 1  (arena offset)
  │    fields.push(FixField { tag, value_len, value_start })
  │    match tag { 35 => msg_type_label, 49 => sender, 52 => time, ... }
  │
  └─ → FixMessage { arena: Vec<u8>, fields: Vec<FixField>, time, sender, ... }

Vec<FixMessage>
  │
  ├─ (Pro) validate_batch()     → Vec<ValidationReport>  (par_iter)
  ├─ (Pro) run_health_checks()  → SessionHealthReport     (sequential, stateful)
  ├─ (Pro) build_session_summary() → SessionSummary
  └─ (Pro) build_scorecard()    → FillQualityScorecard
  │
  └─ app.rs: messages.set(msgs)     ← reactive Signal update
             timeline re-renders (filtered, paginated)
             right panel shows selected message detail
```

---

## Threading Model

| Phase | Threading | Crate |
|---|---|---|
| File dialog | `tokio::spawn` (single async task) | `tokio`, `rfd` |
| Boundary scan (≥2 MB) | `rayon::into_par_iter` across N chunks | `rayon` |
| Message parse | `rayon::par_windows(2)` | `rayon` |
| Validation | `rayon::par_iter` | `rayon` |
| Session health | Single thread (stateful, sequential) | — |
| Session summary | Single thread | — |
| Fill quality | Single thread | — |
| Old Vec<FixMessage> drop | `std::thread::spawn` (background) | `std` |
| UI rendering | Dioxus main thread | `dioxus-desktop` |

Rayon's thread pool is pre-warmed at startup (via `ThreadPoolBuilder.build_global()`) to avoid
first-parse latency. The allocator (mimalloc) maintains per-thread arenas to minimize cross-thread
contention on `Vec::with_capacity` calls in the hot parse loop.

---

## License & Pro Features

```
On startup:
  load_license() → reads ~/.config/aifixparser/license.json
  if Some(StoredLicense) → is_pro = true

On "Activate" button:
  validate_license(key) → POST to api.whop.com/v2/memberships
  success → save_license() → write JSON to config dir
           → is_pro = true, premium_panel shown

dev feature flag (`cargo build --features dev`):
  bypasses all license checks (always returns Some(dev))
```

Pro-gated subsystems: Session Health, Session Summary, Fill Quality, Validator panel,
Lifecycle view, Overview charts, CSV Export.

---

## Binary Layout

```
Cargo.toml
  ├── [[bin]] AiFIXParser   (src/main.rs)    — desktop app
  ├── [[bin]] gen_fix       (src/bin/gen_fix.rs) — test data generator
  └── [lib]   aifixparser   (src/lib.rs)     — public library facade

src/lib.rs exposes:
  pub mod parser, validator, model, dictionary, export, sample, simd
```

`gen_fix` produces:
- `fixtures/fix_test_1m.log` — 1M SOH-delimited FIX 4.4 messages (~195 MB)
- `fixtures/fix_health_test_100k.log` — 100k messages (~19.8 MB)

Run: `cargo run --release --bin gen_fix`

---

## Build & Run

```bash
# Run in development (license bypassed)
cargo run --features dev

# Release build
cargo build --release

# Run benchmarks
cargo bench
# → target/criterion/report/index.html

# Run tests
cargo test

# Generate test fixtures (needed before benchmarks)
cargo run --release --bin gen_fix
```

**Build prerequisites:** Rust stable (≥1.75), nightly not required.
On macOS, Xcode CLI tools needed for linker. On Windows, MSVC toolchain needed for `winres`.

---

## Known Gaps & TODOs

| Item | Status |
|---|---|
| AI chat with logs (`spec/ai_chat_with_your_logs.md`) | Proposed, not implemented |
| Order flow pattern detection (`spec/order_flow_patterns.md`) | Proposed, not implemented |
| Dark mode toggle | Mentioned in code_style notes, no UI control |
| JSON / Parquet export | Not implemented (CSV only) |
| Web deployment | Desktop-only (Dioxus desktop, not web) |
| `spec/fill_quality_score.md` | Stub — feature is implemented, spec is empty |
| `spec/fix_parse.md` | Stub — feature is implemented, spec is empty |

`docs/parser_data_flow.md` references `parse_all_simd(&str)` — **this function was removed**
when `parse_single` was unified to delegate to `parse_single_simd`. See `docs/parser.md`.
