# Issues & Technical Debt

Explicitly flagged issues, dead code, incomplete features, and documentation mismatches.
Cross-referenced with source files where applicable.

---

## Critical: Stale Documentation

### `docs/parser_data_flow.md` — superseded and partially incorrect

The file is marked with a `⚠️ STALE` warning at the top, but it is still present and may mislead readers who open it directly from the IDE.

**Issues in `parser_data_flow.md`:**
- References `parse_all_simd(&str)` — **this function does not exist**. It was removed when `parse_single` was unified to delegate to `parse_single_simd`.
- References `FixField { value: CompactString }` — **this field does not exist**. Replaced by arena offset design (`value_start: u32`, `value_len: u16`).
- Describes "parallel chunk alignment" boundary scan — superseded by the ownership-region parallel scan.
- The file's "Key Design Decisions" table references `CompactString` for all fields, which is now wrong.

**Recommendation:** Delete `docs/parser_data_flow.md` entirely. `docs/parser.md` covers everything it described, correctly and in more depth.

---

## Medium: Missing Planned Features

These features are fully specified but not yet implemented. Engineers should not assume they exist.

### Excel / XLSX Export (`PRO_FEATURES_SPEC.md §2`)

Specified with column definitions, formatting rules, and Order Summary sheet. The `src/export.rs` only implements CSV. The `rust_xlsxwriter` crate has not been added.

**Risk:** The PRO_FEATURES_SPEC implies this is a Pro feature. If it appears in marketing, users may expect it.

### RTT / Latency Percentiles (`PRO_FEATURES_SPEC.md §3`)

Full specification exists: RTT pair detection, P50/P95/P99, per-flow-type stats, per-message overlay. No implementation. `LatencyStats` in `session_summary.rs` captures avg/worst but not percentiles.

### AI Chat with Logs (`spec/ai_chat_with_your_logs.md`)

File exists but is empty (0 bytes). No implementation. No crate for LLM integration added.

### Order Flow Pattern Detection (`spec/order_flow_patterns.md`)

File exists but is empty (0 bytes). No implementation.

---

## Medium: ECharts CDN Dependency

Charts (Lifecycle, Overview, Health) load ECharts from `cdn.jsdelivr.net`:
```
https://cdn.jsdelivr.net/npm/echarts@5.5.1/dist/echarts.min.js
```

If the network is unavailable, all chart panels will show a blank area with no error message. The app does not fall back to text-only or cached JS.

**Impact:** The app is marketed as a desktop tool for FIX log analysis. Traders often work in low-connectivity environments (trading floors, VPNs with restricted egress, air-gapped systems). Charts failing silently is a bad user experience.

**Fix:** Bundle `echarts.min.js` as a local asset and serve it from `dioxus_desktop`'s asset system.

---

## Medium: Folder Load — Messages Not Sorted by Time

`loader::pick_and_load_folder()` traverses directories DFS and merges messages in discovery order. If a folder contains logs from multiple sessions or dates, the merged timeline will have messages out of chronological order.

**File:** `src/loader.rs`

**Impact:** Time-range filters in the timeline (`f_time`) may give unexpected results. Session health rules that assume chronological order (sequence gap detection, heartbeat gap detection) may produce false positives.

**Fix:** Sort merged messages by tag 52 (SendingTime) after all files are loaded. Cost: one `sort_by` on up to `Vec<FixMessage>` — acceptable.

---

## Low: `simd.rs` is Unused by the Parser

`src/simd.rs` exports `find_delimiters(&[u8]) -> Vec<usize>` which finds all delimiter positions in a byte slice. It is exposed as a public API in `src/lib.rs` and has its own benchmark (`bench_scanner`).

**It is not called anywhere in the actual parsing pipeline.** The parser's hot path uses `simd_parse_avx2`/`simd_parse_neon` which find delimiters and extract tokens in a single pass — no separate delimiter scan step.

`simd.rs` is useful as an isolated benchmark target to study delimiter scan throughput, but it shouldn't be in `lib.rs`'s public API surface if it's a benchmark artifact.

**Options:**
1. Move to `benches/` or a private module
2. Add a doc comment clarifying it's a benchmark target only
3. Remove it if no external consumer exists

---

## Low: Dead Spec Files

The following spec files are empty (0 bytes) or stubs:

| File | Status |
|---|---|
| `spec/ai_chat_with_your_logs.md` | Empty — feature not implemented |
| `spec/order_flow_patterns.md` | Empty — feature not implemented |
| `spec/fix_parse.md` | Empty — parser is implemented, spec was never written |
| `spec/fill_quality_score.md` | Empty — feature is implemented, spec was never written |

---

## Low: License Key Hardcoded in Source

`src/license.rs` contains:
```rust
const WHOP_API_KEY: &str = "apik_0V9cFRk4ZURA7_C4706365_...";
const WHOP_COMPANY_ID: &str = "biz_3S9JyWeJxWSJS0";
const WHOP_PRODUCT_ID: &str = "prod_ZEXdRPkg2eG7X";
```

These are embedded in the compiled binary. The API key is a server-side read-only membership check key (not a secret that grants write access), but:
- If the key is rotated, a new binary must be distributed to all users
- The key is visible via `strings` on the binary

**Risk level:** Low for this specific use case (read-only membership check), but worth noting for security review.

---

## Low: `parse_single` (String Path) Formerly Had Separate Hot Loop

Prior to the latest optimization, `parse_single(&str)` had its own tokenization loop using `memchr_iter` and `split_once('=')`. It now delegates to `parse_single_simd(raw.as_bytes())`, sharing the NEON/AVX2 hot loop.

The old implementation allocated into `msg.arena` via `extend_from_slice` (the naive approach that caused regression in Phase 7b). The new delegation correctly uses the pre-copy arena trick.

This is not a bug, but engineers reading old tests or benchmarks comparing the str path should be aware the implementations are now unified.

---

## Low: Commit Message Quality

Recent commits have low-information messages:
```
"Optimzation part 5"
"-mclean up"
"update"
"update demo"
```

This makes it difficult to use `git log` / `git blame` to understand why specific changes were made. Not a runtime issue, but an operational/maintenance concern.

---

## Non-Issue: `push_field` and `set_field_value` are `#[allow(dead_code)]`

These two methods on `FixMessage` are only called from test code in `validator.rs`. They are annotated with `#[allow(dead_code)]` to suppress the compiler warning. This is intentional — they're test helpers and should stay.

---

## Non-Issue: `unsafe` Blocks

The parser uses `unsafe` in two controlled places:
1. `std::str::from_utf8_unchecked(val_b)` — FIX protocol is 7-bit ASCII, which is a subset of valid UTF-8. This is sound.
2. `unsafe { simd_parse_avx2(raw, msg) }` and `unsafe { simd_parse_neon(raw, msg) }` — calling `#[target_feature(enable = "...")]` functions requires `unsafe`. Both are guarded by runtime feature detection.

These `unsafe` usages are intentional, documented, and correct.
