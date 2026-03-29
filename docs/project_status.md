# Project Status & Lineage

---

## What This Is

**AI FIX Parser** (package name `AiFIXParser`, lib name `aifixparser`) is a commercial desktop application for inspecting, analyzing, and validating FIX protocol log files. It targets financial market participants (traders, brokers, exchange engineers, QA engineers) who routinely work with FIX logs containing millions of messages.

The app ships a free tier (parse + inspect) and a Pro tier (session health, fill quality, validator, lifecycle, overview charts) gated by a Whop subscription.

---

## Evolution

### Phase 1 — Initial Parser (v1.x)

The project started as a straightforward FIX parser with a basic Dioxus UI. Key characteristics of the early implementation:
- `String` fields for all tag values — 15M heap allocations for 1M messages
- Single-threaded scalar parser
- Pipe-only delimiter support
- No SIMD

### Phase 2 — Basic Optimizations

Incremental improvements before the performance sprint:
- `CompactString` replaced `String` — short values (≤23 bytes) stay on stack
- `memchr`/`memmem` for delimiter search — SIMD substring scan for "8=FIX"
- Rayon `par_iter` — multiple cores doing useful work
- `normalize_delimiters` — single-pass SOH/^A/\x01 → pipe conversion
- `opt-level = 3`, `panic = "abort"` — LLVM vectorization
- Baseline: ~263ms for 1M messages

### Phase 3 — ARM NEON SIMD

Discovered the AVX2 path was silently inactive on Apple M1 (aarch64). Added `simd_parse_neon` with a weight-sum movemask emulation:
- Result: ~140ms → ~80ms (43% speedup)
- Lesson: Always verify which code path executes on target hardware

### Phase 4 — mimalloc + Serial memmem

Added `mimalloc` as global allocator to eliminate multi-thread malloc contention. Separated boundary scan (`message_start_offsets`) from per-message parse to avoid double-scanning:
- Boundary scan: serial memmem → `Vec<u32>` offsets (4 bytes vs 16-byte fat pointers)
- Parse: `par_windows(2)` over offset array
- Result: ~80ms → ~113ms initial (regression from double-alloc), then optimized to ~79ms

### Phase 5 — `FixField.tag: u16`

Changed `FixField.tag` from `u32` to `u16`. This alone gave the largest single speedup:
- Smaller struct → better L1 cache utilization when scanning fields
- Result: ~113ms → ~79ms (largest single win)

### Phase 6 — Startup Prewarm

Added Rayon thread pool prewarm and a dummy parse on startup to warm CPU caches and TLS arenas. Eliminates first-parse latency spike visible in interactive use.

### Phase 7 — Data-Oriented Design (DOD Arena)

Replaced `FixField { tag: u16, value: CompactString }` (32 bytes) with `FixField { tag: u16, value_len: u16, value_start: u32 }` (8 bytes). Added `arena: Vec<u8>` to `FixMessage` holding all field value bytes.

Initial implementation caused a regression (104ms → 111ms) due to per-field `extend_from_slice` calls. Fixed by pre-copying raw bytes in one `raw.to_vec()` and computing value offsets as pure arithmetic (`start + eq_index + 1`).

- Result: 104ms → 101ms (after fix)

### Phase 8 — Parallel Boundary Scan

Parallelized the serial `memmem` boundary scan using ownership regions: each Rayon worker scans its chunk plus a 4-byte overlap and keeps only markers within `[own_start, own_end)`. Threshold: files ≥2 MB use the parallel path.

- Result: 101ms → **85ms** (eliminated 19ms of serial work)

### Current State: v2.0.6

The parser achieves ~85ms for 1M SOH-delimited FIX 4.4 messages on Apple M1 Max. The full optimization journey is documented in [spec/blog-parse-1m-fix-under-100ms.md](../spec/blog-parse-1m-fix-under-100ms.md).

---

## Current Feature Status

| Feature | Status | Notes |
|---|---|---|
| Parse FIX logs (pipe + SOH) | ✅ Complete | AVX2 / NEON / scalar, all paths tested |
| Timeline view + filtering | ✅ Complete | 6 filter columns, infinite scroll |
| Message detail (Table/Raw/JSON) | ✅ Complete | |
| FIX Validator | ✅ Complete | 6 pass types, parallel |
| Session Health (7 rules) | ✅ Complete | Sequential; requires Pro |
| Session Summary | ✅ Complete | Stats, latency, top symbols |
| Fill Quality Scorecard | ✅ Complete | Per-cp, per-symbol, size buckets |
| Lifecycle view | ✅ Complete (UI) | ECharts flow graph; requires Pro |
| Overview charts | ✅ Complete (UI) | ECharts multi-chart; requires Pro |
| CSV Export | ✅ Basic | Per-message, no column configuration |
| License system (Whop) | ✅ Complete | Whop API, local JSON cache |
| Folder load (multi-file) | ✅ Complete | DFS, merges all files |
| Benchmark suite | ✅ Complete | Criterion, HTML reports |
| Test data generator | ✅ Complete | 1M realistic FIX 4.4 messages |
| Excel/XLSX export | ❌ Not started | Specified in PRO_FEATURES_SPEC.md |
| AI chat with logs | ❌ Not started | Stub spec only |
| Order flow pattern detection | ❌ Not started | Stub spec only |
| Dark mode toggle | ❌ Missing | Mentioned in code_style; no UI control |
| RTT / Latency percentiles | ❌ Not started | Specified in PRO_FEATURES_SPEC.md |
| Web deployment | ❌ Not planned | Desktop-only (Dioxus desktop) |

---

## Branch & Release Cadence

Recent commits suggest rapid iteration:

```
0be871d  Optimization part 5
1074e18  Clean up
f51cc04  Update
d1556ff  Update demo
aef6e8f  Update workflow
```

Releases tracked via PR merges from `feature/v2.0.x` branches into `main`. Current working branch: `feature/v2.0.6`.

---

## Known Intentional Constraints

- **Desktop-only:** The Dioxus desktop target was chosen for low-friction distribution (no server infra, no WASM complexity). A web version is possible but not planned.
- **FIX 4.x focus:** The dictionary and validator rules are built for FIX 4.4. FIXT.1.1 / FIX 5.0 are partially supported (parsed, but not validated to spec).
- **No persistence:** Messages are not saved between sessions. Every launch starts fresh.
- **No multi-file sort:** Folder loading merges files in DFS order; messages from different files are not sorted by timestamp.
