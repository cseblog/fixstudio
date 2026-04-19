# Build & Run

---

## Prerequisites

| Tool | Version | Required for | Install |
|---|---|---|---|
| Rust (stable) | ≥ 1.75 | Everything | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Xcode CLI tools | Any | macOS linker | `xcode-select --install` |
| MSVC toolchain | Any | Windows build | Install via Visual Studio Installer |
| Cargo (bundled with Rust) | Any | Build system | Included with `rustup` |

Nightly Rust is **not required**. All SIMD intrinsics use stable `std::arch` APIs.

---

## Quick Start

```bash
# Clone and enter the project
git clone <repo-url>
cd fixstudio

# Development build (license checks bypassed, Pro features unlocked)
cargo run --features dev

# Production build
cargo build --release
# Binary: target/release/AiFIXParser
```

---

## Development

### Run in dev mode (recommended for iterating)

```bash
cargo run --features dev
```

`--features dev` enables the `dev` feature flag which:
- Bypasses all Whop license checks
- Returns a synthetic Pro license immediately
- Unlocks all Pro features (Health, Fill Quality, Lifecycle, Overview, Validator)

**Do not ship a binary built with `--features dev`.**

### Check for errors without running

```bash
cargo check
cargo check --features dev
```

### Run tests (33 tests across parser, validator, session analysis)

```bash
cargo test
```

Tests live in:
- `src/parser.rs` — parse_all, parse_all_simd_bytes, SIMD paths, normalize
- `src/validator.rs` — required tags, enums, checksum, body length
- `src/session_health.rs` — gap detection, reconnect detection, rate bursts

---

## Benchmarks

### Step 1: Generate test fixtures

The benchmark data files are **not checked in** (too large for git). Generate them first:

```bash
cargo run --release --bin gen_fix
```

This produces:
- `fixtures/fix_test_1m.log` — 1,000,000 FIX 4.4 messages, ~195 MB, SOH-delimited
- `fixtures/fix_health_test_100k.log` — 100,000 messages with deliberate health issues

**Run time:** ~5–10 seconds.

### Step 2: Run benchmarks

```bash
cargo bench
```

For native CPU tuning on the current machine (enables AVX2 on x86, improves NEON scheduling on ARM):

```bash
RUSTFLAGS="-C target-cpu=native" cargo bench
```

### Benchmark groups

| Group | Measures |
|---|---|
| `100k_messages` | 100k message parse — pipe path, SOH path, str path |
| `1m_messages` | Full 1M message parse (the headline benchmark) |
| `single_execution_report` | Per-message microbenchmark |

Results are in `target/criterion/report/index.html` (open in a browser).

**Current results (M1 Max, opt-level 3, thin LTO):**

| Benchmark | Time |
|---|---|
| Single ExecutionReport (SIMD bytes) | ~254 ns |
| 100k messages (SIMD bytes, SOH) | ~7.9 ms |
| 1M messages (SIMD bytes) | **~85 ms** |

---

## Release Build

```bash
cargo build --release
```

Binary output: `target/release/AiFIXParser`

Release profile settings (from `Cargo.toml`):
- `opt-level = 3` — maximum LLVM optimization
- `lto = true` — link-time optimization across all crates
- `strip = true` — debug symbols removed (smaller binary)
- `codegen-units = 1` — single codegen unit (better inlining)
- `panic = "abort"` — no stack unwinding

### macOS

```bash
cargo build --release --target aarch64-apple-darwin    # Apple Silicon
cargo build --release --target x86_64-apple-darwin     # Intel Mac
```

For a universal binary:
```bash
lipo -create -output AiFIXParser \
  target/aarch64-apple-darwin/release/AiFIXParser \
  target/x86_64-apple-darwin/release/AiFIXParser
```

### Windows

Requires the MSVC toolchain (not MinGW):
```bash
rustup target add x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
```

The `build.rs` embeds `assets/icon.ico` and a DPI-awareness manifest into the Windows binary via `winres`.

### Linux

```bash
cargo build --release --target x86_64-unknown-linux-gnu
```

Note: Dioxus desktop on Linux uses WebKitGTK. Install required system packages:
```bash
# Ubuntu/Debian
sudo apt-get install libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev \
  librsvg2-dev patchelf

# Fedora
sudo dnf install gtk3-devel webkit2gtk4.1-devel
```

---

## Test Data Generator

```bash
cargo run --release --bin gen_fix
```

Produces realistic FIX 4.4 data:
- Mixed message types: ~60% ExecutionReport, ~20% NewOrderSingle, ~20% session
- Multiple sender/target pairs
- FX trading scenario (currency pairs, realistic prices/sizes)
- `fix_health_test_100k.log` includes: sequence gaps, reconnects, rate bursts, rejected cancels

---

## Environment & Platform Notes

| Platform | SIMD path | Notes |
|---|---|---|
| macOS (Apple Silicon) | NEON (aarch64) | Always available; no runtime check needed |
| macOS (Intel) | AVX2 or scalar | Detected at runtime via `is_x86_feature_detected!` |
| Windows (x86_64) | AVX2 or scalar | Same runtime detection |
| Linux (x86_64) | AVX2 or scalar | Same runtime detection |
| Other | Scalar | Byte-by-byte fallback |

### mimalloc initialization

mimalloc is set as the global allocator in `src/main.rs`:
```rust
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
```

No configuration needed. It automatically uses per-thread arenas.

### Rayon thread pool prewarm

On startup, `main.rs` prewarms the Rayon thread pool to avoid first-parse latency:
```rust
rayon::ThreadPoolBuilder::new()
    .build_global()
    .ok();
```

This spawns all worker threads immediately. Without it, the first file load triggers thread creation, adding ~50ms.

---

## Common Issues

| Symptom | Cause | Fix |
|---|---|---|
| `fixtures/fix_test_1m.log: No such file` | Fixture not generated | Run `cargo run --release --bin gen_fix` |
| Charts not rendering | No internet (ECharts loads from CDN) | Ensure internet access or bundle ECharts locally |
| Pro features locked in dev | Missing `--features dev` flag | `cargo run --features dev` |
| `dioxus-desktop` link error on Linux | Missing WebKitGTK headers | Install GTK3 + WebKit2GTK packages (see Linux section above) |
| Slow first benchmark run | Rayon threads not prewarm in bench context | Use `cargo bench` (Criterion warms up runs; first measurement may be slower) |
