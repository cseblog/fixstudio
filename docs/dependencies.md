# Dependencies

All dependencies extracted from `Cargo.toml`. Versions are as pinned in the manifest.

---

## Runtime Dependencies

| Crate | Version | Purpose | Notes |
|---|---|---|---|
| `compact_str` | 0.8 | Inline string storage (≤23 bytes on stack) | Used for hot-path fields in `FixMessage` and `FixField` |
| `tokio` | 1 | Async runtime | Only `rt` and `sync` features enabled; used for `spawn()` in loader and license |
| `serde` | 1 | Serialization framework | `derive` feature; used for license JSON and API response |
| `serde_json` | 1 | JSON encode/decode | License file and Whop API response |
| `mimalloc` | 0.1 | High-performance allocator | `default-features = false` (disables secure mode for speed); replaces system malloc |
| `memmap2` | 0.9 | Zero-copy memory-mapped file I/O | Hot path: file bytes as `&[u8]` backed by OS page cache |
| `memchr` | 2 | SIMD substring/byte search | `memmem::find_iter("8=FIX")` for message boundary scan; ~2 GiB/s |
| `dioxus` | 0.7.2 | Reactive desktop UI framework | `desktop` feature; renders via embedded WebView |
| `dioxus-desktop` | 0.7.2 | Desktop window integration for Dioxus | Wraps `tao` and WebView |
| `ico` | 0.3 | `.ico` file encoding | Used in `build.rs` for Windows icon embedding |
| `rayon` | 1.10 | Data parallelism (thread pool) | `par_windows`, `par_iter`, `into_par_iter` across message slices |
| `rfd` | 0.15 | Native async file/folder picker dialogs | macOS: NSOpenPanel; Windows: IFileOpenDialog |
| `tao` | 0.34 | Cross-platform window abstraction | Event loop and window management; pulled in by dioxus-desktop |
| `open` | 5 | Open URLs in the default browser | Used for "Buy Pro" / changelog links |
| `reqwest` | 0.12 | HTTP client | `default-features = false`, `json` + `rustls-tls` features; used for Whop license API |

---

## Dev Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `criterion` | 0.5 | Statistical benchmark framework with HTML reports |

Run benchmarks with:
```bash
cargo bench
# HTML report: target/criterion/report/index.html
```

---

## Build Dependencies (Windows only)

| Crate | Version | Purpose |
|---|---|---|
| `winres` | 0.1 | Embed `.ico` and DPI-awareness manifest into Windows PE binary |

Only linked on `cfg(windows)` — does not affect macOS or Linux builds.

---

## Feature Flags

| Feature | Effect |
|---|---|
| `dev` | Bypasses all license checks (always returns a valid Pro license). **Never enable in release builds.** |

Usage:
```bash
cargo run --features dev    # development: Pro features unlocked, no Whop API call
cargo build --release       # production: license checks active
```

---

## Dependency Graph Notes

### What `dioxus 0.7.2` brings in transitively
- `tao 0.34` — windowing
- `wry` — WebView abstraction (macOS: WKWebView; Windows: WebView2; Linux: WebKitGTK)
- `tokio` — async runtime (overlaps with explicit dep)

### `reqwest` with `rustls-tls`
Avoids linking OpenSSL or platform TLS. The binary is fully self-contained on all platforms.

### `mimalloc` with `default-features = false`
The default mimalloc build enables secure mode (randomized allocation patterns, guard pages). Disabling it removes the security overhead, which is appropriate for a desktop app that doesn't expose allocation to untrusted code.

---

## Outdated / Risk Assessment

| Crate | Version | Risk | Notes |
|---|---|---|---|
| `dioxus` | 0.7.2 | Medium | Dioxus is actively developed; breaking changes between minors are common. Pin tightly. |
| `tao` | 0.34 | Low | Stable; driven by dioxus-desktop version |
| `reqwest` | 0.12 | Low | 0.12 series is stable and maintained |
| `rfd` | 0.15 | Low | Matches dioxus-desktop's expected dialog API |
| `compact_str` | 0.8 | Low | Stable API; 0.8 is current |
| `rayon` | 1.10 | Low | Very stable; 1.x has been stable for years |
| `memchr` | 2 | Low | Extremely stable; widely used |
| `mimalloc` | 0.1 | Low | Version tracks the upstream mimalloc C library |

**No conflicting versions detected.** All dependencies resolve to a single version tree (verified by `cargo check`).

**Missing dependency for planned features:**
- Excel export (`PRO_FEATURES_SPEC.md §2`) requires `rust_xlsxwriter` or `xlsxwriter-rs` — not yet added.
- AI chat feature (`spec/ai_chat_with_your_logs.md`) would require an Anthropic/OpenAI SDK — not yet added.
