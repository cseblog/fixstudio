# Technologies

Per-subsystem breakdown of the technology stack. See [architecture.md](architecture.md) for how subsystems interact.

---

## Parser (`src/parser.rs`, `src/simd.rs`)

**Purpose:** Convert raw FIX log bytes into `Vec<FixMessage>` in under 100ms for 1M messages.

**Stack:**
- Rust 2021, `std::arch::{x86_64, aarch64}` (SIMD intrinsics)
- `memchr 2` — SIMD `memmem` for "8=FIX" boundary scan (~2 GiB/s throughput)
- `rayon 1.10` — data-parallel `par_windows(2)` over message offset array
- `mimalloc 0.1` — replaces system allocator; eliminates global mutex contention across Rayon threads
- `compact_str 0.8` — inline string storage (≤23 bytes on stack); avoids heap allocation for short FIX values (sender, target, msg type)

**Why this stack:**
FIX logs are mechanically regular (pure ASCII, fixed-length tags, delimiter-separated). SIMD vectorization is straightforward and gives 16–32 bytes per instruction. The main bottleneck was allocator contention at 1M messages × 2 allocations each — mimalloc's thread-local arenas eliminate that.

**Notable design constraint:**
ARM NEON has no `movemask_epi8` equivalent, requiring a weight-sum emulation to build the bitmask from comparison results. See [parser.md](parser.md#simd_parse_neon--aarch64-path) for the implementation.

---

## Data Model (`src/model.rs`)

**Purpose:** Memory-efficient representation of parsed FIX messages shared across all subsystems.

**Stack:**
- Rust structs; no external serialization crate in the hot path
- `compact_str 0.8` for pre-extracted hot fields
- Arena + offset pattern (Data-Oriented Design): `Vec<u8>` arena + `FixField { tag: u16, value_len: u16, value_start: u32 }`

**Why this design:**
The arena design reduces `FixField` from 32 bytes (with `CompactString`) to 8 bytes — a 4× reduction. For 1M messages × 20 fields, that's 160 MB vs 640 MB of field-array data. See [data_model.md](data_model.md#arena-design-data-oriented) for the full breakdown.

---

## Validator (`src/validator.rs`)

**Purpose:** Check parsed messages against FIX 4.x specification rules.

**Stack:**
- Pure Rust; no external crate
- `rayon` for `validate_batch` (parallel validation)
- Static lookup tables (compiled into binary from `spec/fix44.xml` via `dictionary.rs`)

**Why this design:**
Validation is embarrassingly parallel — each message is independent. Rayon `par_iter` scales linearly with cores. The static tables avoid a runtime XML parse step.

---

## Session Analysis (`src/session_health.rs`, `src/session_summary.rs`, `src/fill_quality.rs`)

**Purpose:** Compute higher-level diagnostics over the full message set.

**Stack:**
- Pure Rust; single-threaded (each module has stateful sequential logic)
- Indexed via `Vec<usize>` (`msg_indices`) to avoid copying messages into sub-slices

**Why single-threaded:**
Session health rules (sequence gaps, reconnect detection) depend on message ordering and inter-message state. Parallelizing would require sorting/merging results, adding complexity without clear benefit at typical file sizes (<1M messages).

---

## UI (`src/app.rs`, `src/components/`)

**Purpose:** Native desktop application shell, layout, and reactive rendering.

**Stack:**
- `dioxus 0.7.2` + `dioxus-desktop 0.7.2` — reactive Rust UI framework that renders via an embedded WebView
- `tao 0.34` — cross-platform window abstraction (event loop, window creation) used by dioxus-desktop
- `tokio 1` (rt, sync features only) — async runtime for file dialog and license HTTP calls
- `rfd 0.15` — native async file/folder picker dialogs (macOS: NSOpenPanel; Windows: IFileOpenDialog)

**Why Dioxus:**
Dioxus provides a React-like component model in Rust without a separate JS build step. The desktop target embeds a WebView for rendering, which gives OS-native look-and-feel while keeping all logic in Rust.

**Trade-off:**
The WebView approach means some CSS/layout quirks, and ECharts charts require CDN JavaScript loaded at runtime (requires internet on first use or offline bundle).

---

## Charts (`src/components/lifecycle.rs`, `overview.rs`, `premium_panel.rs`)

**Purpose:** Interactive data visualization for order flow, message rates, and session health.

**Stack:**
- [ECharts 5.5.1](https://echarts.apache.org/) — loaded from CDN (`cdn.jsdelivr.net`)
- Dioxus `eval()` JS interop to initialize and update chart instances
- No Rust charting crate

**Why ECharts:**
ECharts is feature-complete for the required chart types (flow graphs, scatter, bar, histogram, heatmap) and has a stable API. A Rust-native charting crate would require rebuilding chart logic that ECharts already provides.

**Risk:**
CDN dependency — if the network is unavailable, charts will not render. An offline bundle (bundled JS file) would be the fix. This is documented in [issues.md](issues.md).

---

## File I/O & Loader (`src/loader.rs`)

**Purpose:** Load FIX log files from disk with minimal memory allocation.

**Stack:**
- `memmap2 0.9` — memory-mapped file access (OS page cache → zero-copy)
- `rfd 0.15` — async file picker
- `tokio::spawn` for async execution

**Why mmap:**
For a 200 MB FIX log, reading the entire file into a `Vec<u8>` allocates 200 MB. With mmap, the OS maps the file into virtual address space; pages are loaded on demand. The parser receives a `&[u8]` slice that looks like memory but is backed by the page cache. No upfront copy.

**Fallback:**
If mmap fails (e.g., some network-mounted filesystems, or OS permission issues), the loader falls back to `file.read() → Vec<u8>`.

---

## License (`src/license.rs`)

**Purpose:** Gate Pro features behind a subscription check.

**Stack:**
- `reqwest 0.12` with `rustls-tls` (no native TLS dependency) — HTTP client for Whop API
- `serde 1` + `serde_json 1` — serialize/deserialize license JSON and API responses
- `tokio::spawn` for async validation

**Why rustls:**
`rustls` is a pure-Rust TLS implementation that avoids linking OpenSSL or system TLS libraries. This simplifies cross-platform distribution (Windows, macOS, Linux) without dealing with platform-specific TLS setup.

**External dependency:**
License validation requires a network call to `api.whop.com`. Once activated, the key is cached locally and no further network calls are made per session.

---

## Export (`src/export.rs`)

**Purpose:** Write filtered messages to CSV.

**Stack:**
- Rust stdlib `std::io::Write` — streaming CSV write
- No external CSV crate

**Why no external crate:**
The CSV format needed is simple (comma-delimited, double-quote escape). A purpose-built `csv` crate would add a dependency without meaningful benefit.

**Missing:** Excel/XLSX export is specified in [PRO_FEATURES_SPEC.md](PRO_FEATURES_SPEC.md) but not yet implemented. Would require `rust_xlsxwriter` or similar.

---

## Dictionary (`src/dictionary.rs`)

**Purpose:** Map FIX tag numbers and enum values to human-readable labels.

**Stack:**
- Static Rust `match` expressions compiled from `spec/fix44.xml`
- No runtime XML parsing

**Why static:**
Tag/value lookup happens in the hot parse loop (`apply_token`) and in every UI render. A HashMap would add hash overhead per lookup. Static `match` compiles to an efficient jump table.

---

## Test Data Generator (`src/bin/gen_fix.rs`)

**Purpose:** Generate realistic synthetic FIX log files for benchmarking and testing.

**Stack:**
- XorShift64 PRNG (no external rand crate)
- Pure Rust; writes SOH-delimited FIX 4.4 directly to file

**Output:**
- `fixtures/fix_test_1m.log` — 1M messages, ~195 MB (mixed ExecReports, NOS, session)
- `fixtures/fix_health_test_100k.log` — 100k messages with intentional health issues

---

## Build (`Cargo.toml`, `build.rs`)

**Stack:**
- Cargo (Rust build system)
- `winres 0.1` (Windows build-dependency only) — embeds `.ico` and DPI-awareness manifest into Windows PE
- `criterion 0.5` — statistical benchmarking with HTML reports

**Profile settings:**
```
[profile.release]
opt-level = 3       # Maximum LLVM optimization (vectorization, unrolling)
lto = true          # Link-time optimization across crates
strip = true        # Remove debug symbols from release binary
codegen-units = 1   # Single codegen unit enables global optimization
panic = "abort"     # No unwinding stack; saves ~5–10% binary size
```
