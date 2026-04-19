# How We Parse 1 Million FIX Messages in Under 100ms

*A story of SIMD, wrong assumptions, a type change that changed everything, Andrew Kelley's Zig compiler talk on data-oriented design, and what happened when we looked at the small-input case.*

---

## Background

[AI FIX Parser](https://aifixparser.com) is a desktop app that lets traders and engineers load FIX protocol log files and inspect, filter, and analyze every message. FIX logs from busy trading sessions can have millions of messages. If parsing takes seconds, the app feels broken. We set ourselves a target: **parse 1 million messages in under 100ms** on a developer's laptop.

This post documents every optimization we tried, what we measured, and — critically — what we learned when our assumptions turned out to be wrong.

---

## The Setup

- **Machine:** Apple M1 Max, 10 cores (8 performance + 2 efficiency)
- **Test file:** 1,000,000 FIX 4.4 messages, SOH-delimited, 195 MB
- **Mix:** ~60% ExecutionReports, ~20% NewOrderSingles, ~20% session messages (Heartbeat, Logon, etc.)
- **Benchmark tool:** Criterion.rs, 10 samples per bench, `profile.bench` opt-level 3, thin LTO

**Baseline: 263ms — 1M messages, single pass, scalar parser.**

Everything below is the story of getting to **87ms** (best single run measured: 85ms — Criterion noise on 10 samples for this workload is ±3–5%) — and the lessons we learned along the way.

---

## Phase 1 — The Obvious Stuff (263ms → ~140ms)

Before the performance sprint, the parser was already through three rounds of basic optimization:

| Change | Effect |
|---|---|
| `String` → `CompactString` for field values | Eliminated ~15M heap allocs for short strings (tags/values fit inline ≤ 23 bytes) |
| `memchr`/`memmem` for delimiter search | SIMD-accelerated scan, ~16–32 bytes/cycle |
| Rayon `par_iter` across messages | All 10 cores doing useful work |
| `normalize_delimiters` single-pass | SOH input: 1× alloc instead of 3× chained `.replace()` |
| `opt-level = 3`, `panic = "abort"` | LLVM vectorization + unrolling, ~10–30% uplift |
| Lazy `value_description` | Removed 1.5M dictionary lookups from the hot parse path |

These got us from ~263ms to a reasonable starting point. But for 1M messages over 195MB, we were still at ~140ms.

---

## Phase 2 — The Assumption That Was Wrong (140ms → ~80ms)

Our parser had a `simd_parse_avx2` function — 32-byte AVX2 vectorized scanning for delimiters. We assumed it was running.

**It wasn't.**

AVX2 is x86. We were running on an **Apple M1 Max (ARM aarch64)**. The `#[cfg(target_arch = "x86_64")]` guard meant we silently fell back to a scalar byte loop for every single delimiter scan. With 15–30 fields per message and 1M messages, that's 15–30 million scalar iterations.

**Lesson #1: Always verify which code path is actually executing on your target hardware.**

### Fix: ARM NEON SIMD

We implemented `simd_parse_neon` — a 128-bit vectorized delimiter scanner using ARM NEON intrinsics. 16 bytes per iteration, comparing against SOH (`0x01`) and pipe (`|`) simultaneously:

```rust
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn simd_parse_neon(raw: &[u8], msg: &mut FixMessage) {
    use std::arch::aarch64::*;
    let soh_vec  = vdupq_n_u8(0x01);
    let pipe_vec = vdupq_n_u8(b'|');
    // Powers-of-two weights to emulate x86 movemask: [1,2,4,8,16,32,64,128]
    let weights_lo = vcreate_u8(0x8040201008040201_u64);
    let weights_hi = vcreate_u8(0x8040201008040201_u64);

    let chunk_count = raw.len() / 16;
    let mut start = 0;

    for chunk_index in 0..chunk_count {
        let chunk = vld1q_u8(raw.as_ptr().add(chunk_index * 16));
        let any = vorrq_u8(vceqq_u8(chunk, soh_vec), vceqq_u8(chunk, pipe_vec));

        // Build a 16-bit bitmask: bit i = 1 means byte i is a delimiter.
        let lo_bits = vaddv_u8(vand_u8(vget_low_u8(any),  weights_lo)) as u16;
        let hi_bits = (vaddv_u8(vand_u8(vget_high_u8(any), weights_hi)) as u16) << 8;
        let mut mask: u16 = lo_bits | hi_bits;

        let base = chunk_index * 16;
        while mask != 0 {
            let end = base + mask.trailing_zeros() as usize;
            apply_token(raw, start, end, msg);
            start = end + 1;
            mask &= mask - 1; // clear lowest set bit
        }
    }
    // scalar tail...
}
```

ARM has no hardware `movemask` instruction (unlike x86 `_mm256_movemask_epi8`). We emulated it: compare 16 bytes against each delimiter, OR the two result masks, AND with a power-of-two weight vector, then sum horizontally with `vaddv_u8` — the output is a 16-bit bitmask of delimiter positions.

**But we got the weights wrong on the first attempt.**

We used `[1,2,3,4,5,6,7,8]` (sequential) instead of `[1,2,4,8,16,32,64,128]` (powers of two). The bitmask positions were garbage. The parser read wrong byte offsets, tag extraction silently produced wrong values, and tests panicked.

**Lesson #2: NEON movemask emulation requires exact powers of two as weights. Sequential weights produce wrong bit positions.**

Correct weights: `vcreate_u8(0x8040201008040201_u64)` — little-endian encoding of `[1,2,4,8,16,32,64,128]`.

We also removed a "quick-rejection" `vmaxvq_u8` check we had added. The logic was: if no byte in the 16-byte chunk is ≥ 0x01, skip the chunk entirely. FIX messages have a delimiter roughly every 10 bytes, so nearly every 16-byte chunk hits at least one — the check fired almost never and cost us an extra instruction plus a branch per chunk.

**Lesson #3: Optimization intuition about "common cases" often needs measurement. Our "quick rejection" almost always ran the slow path.**

---

## Phase 3 — Allocator Contention (~80ms → ~68ms)

With NEON running, 10 Rayon threads were all allocating `Vec<FixField>` simultaneously. The system allocator uses a global mutex for large allocations — under high parallelism, threads queue behind each other.

**Fix: mimalloc**

```toml
# Cargo.toml
mimalloc = { version = "0.1", default-features = false }
```

```rust
// lib.rs
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
```

mimalloc gives each thread its own arena. Allocations from different threads never contend. The impact was immediate and consistent across runs.

---

## Phase 4 — Smarter Parallelism (~68ms → ~110ms → ~50ms on 100k)

The original parallel split looked like this:

```rust
// Phase 1 parallel approach — one slice per message
let slices = message_slices_bytes(input);  // serial O(n) scan
slices.par_iter().map(parse_single_simd).collect()
```

This had a subtle performance problem: the serial boundary scan became a bottleneck before parallelism even started, and with N threads getting N tasks there was no work-stealing to handle load imbalance.

We tried a `parse_chunk` approach — splitting the input into coarse chunks and having each thread both find its own "8=FIX" boundaries AND parse them:

```rust
starts.par_windows(2)
    .flat_map_iter(|w| parse_chunk(&input[w[0]..w[1]]))
    .collect()
```

This caused a different problem: each chunk's parser scanned for boundaries within the chunk, so **every byte was scanned twice** — once to find chunk boundaries, once to find field delimiters during parsing. Performance regressed.

**Lesson #4: If you parallelize boundary detection AND parse each message, you double-scan every byte. Separate the phases.**

**Fix: serial memmem scan → `par_windows(2)` on `Vec<u32>` offsets**

```rust
fn message_start_offsets(input: &[u8]) -> Vec<u32> {
    let capacity = (input.len() / AVG_MSG_BYTES).max(4);
    let mut offsets = Vec::with_capacity(capacity);
    for pos in memmem::find_iter(input, b"8=FIX") {
        offsets.push(pos as u32);  // u32 not usize: 4× smaller vec
    }
    offsets
}

// One serial scan → one parallel parse, no double-scanning.
offsets.push(input.len() as u32);  // sentinel
offsets.par_windows(2)
    .map(|w| parse_single_simd(&input[w[0] as usize..w[1] as usize]))
    .collect()
```

Notice we store `u32` offsets instead of `usize` (8 bytes on 64-bit). FIX log files are < 4 GB, so u32 is sufficient. For 1M messages, `Vec<u32>` is **4× smaller** than `Vec<usize>` — the whole offset list fits in a few hundred KB, well inside L2 cache.

**Lesson #5: Use the smallest integer type that fits your data. 4× smaller working set = fewer cache misses when distributing work across rayon workers.**

---

## Phase 5 — The Single Biggest Win: Tag Type (113ms → 79ms)

*Where we are: ~113ms on 1M after Phase 4. Phase 4's `par_windows` split scaled better on smaller inputs (it produced the 50ms 100k number above) but added per-task overhead that hurt the already-fast 1M case. Phase 5 onward more than recovers it.*

After the above, we profiled and found a surprising hot spot in the field model itself:

```rust
// Before
pub struct FixField {
    pub tag: CompactString,  // "35", "49", "52" — always 1–5 digits
    pub value: CompactString,
}
```

For 1M messages with ~15 fields each, that's **15 million `CompactString::from(tag_str)` calls** — each touching the inline buffer, computing a length, and pattern-matching a string we immediately throw away after identifying the tag number.

More importantly: `FixField` was **48 bytes** — `CompactString` (24) + `CompactString` (24). At 15 fields per message × 1M messages = 15M FixField structs × 48 bytes = **720 MB** of field data written per parse.

**Fix: `FixField.tag` as `u16`**

```rust
pub struct FixField {
    pub tag: u16,           // 2 bytes, zero allocation
    pub value: CompactString,
}
// FixField: 48 bytes → 32 bytes (33% less data written)
```

This required changing:
- `src/model.rs` — field type
- `src/parser.rs` — `apply_token` uses `tag_to_u16(tag_bytes)` directly
- `src/dictionary.rs` — four functions changed from `tag: &str` to `tag: u16`; all string match arms (`"35" =>`) converted to integer arms (`35 =>`)
- `src/components/detail.rs` — seven call sites updated

The integer match compiles to a **jump table** — O(1) dispatch vs O(tags) string comparison. And 15M CompactString constructions for tag strings simply ceased to exist.

**Result: 113ms → 79ms. A 30% drop from a type annotation change.**

**Lesson #6: Data layout is performance. A struct field type that seems cosmetic can dominate your hot path. Profile before assuming anything.**

---

## Phase 6 — Startup Prewarm (79ms parse, but ~250ms first open)

The parser was fast in benchmarks. But the first file load in the actual app felt slow. Profiling revealed three cold-start costs paid exactly once:

1. **Rayon thread spawn:** Rayon creates its thread pool lazily. The first `par_windows` call blocked while macOS spawned 9 threads — 20–80ms.
2. **mimalloc TLS initialization:** Each worker thread initializes its per-thread arena on first allocation.
3. **Instruction cache cold start:** `simd_parse_neon`, `apply_token`, `tag_to_u16` were evicted from i-cache since app launch.

**Fix: prewarm at `main()` startup**

```rust
fn prewarm() {
    // Force Rayon thread pool creation now, not on first file open.
    rayon::ThreadPoolBuilder::new().build_global().ok();
    // Touch the hot code paths with a tiny parse.
    let dummy = b"8=FIX.4.2\x019=5\x0135=D\x0110=000\x01";
    let _ = parser::parse_all_simd_bytes(dummy);
}

fn main() {
    prewarm();  // runs before the window opens (~5ms, invisible to user)
    // ... launch UI
}
```

First real file load improved from ~250ms to ~110ms (still includes cold SSD read) and is indistinguishable from warm loads thereafter.

**Lesson #14: The warm-up gap is real.** Benchmark numbers and first-run app numbers are different problems. Thread pools, allocator arenas, and instruction caches all need one touch before they perform at full speed. A microbenchmark that reuses the same allocator and the same hot code path will never see this cost — a real first file open does.

**Lesson #15: The SSD floor is real too.** 195 MB at ~5 GB/s NVMe = ~39ms of unavoidable I/O on a cold read. No software optimization can parse bytes that haven't arrived. Cold-load time will always be warm-load time + I/O time, and the only lever software has on the I/O side is hinting (`MADV_SEQUENTIAL`, prefetch) which is small compared to the total.

---

## Phase 7 — Data-Oriented Design: The Andrew Kelley Lesson

We watched Andrew Kelley's talk *"A Practical Guide to Applying Data-Oriented Design"* (Handmade Seattle 2021). The core thesis:

> **"Use less memory, go fast. The bottleneck is always cache misses."**

His worked example from the Zig compiler: tokens went from 64 bytes each (storing the full string per token) down to 5 bytes (storing a u32 offset + a tag byte into the source file). The source file is already in memory — why copy it?

We looked at our `FixField` after Phase 5:

```rust
pub struct FixField {
    pub tag: u16,            // 2 bytes
    pub value: CompactString, // 24 bytes (inline for ≤ 23 chars)
}
// Layout: u16 (2) + 6 bytes padding + CompactString (24) = 32 bytes total
```

`CompactString` stores the value inline if it fits in 23 bytes. Most FIX values do — "EXEC", "FIX.4.4", "420.50", "1" — so there were no heap allocations. But we were still copying the value bytes into that inline buffer for every single field, every single message.

For 1M messages × 20 fields each = **20 million CompactString copies**.

Kelley's insight: the value bytes already exist in the raw input we scanned. Why copy them at all?

### The Arena Design

```rust
pub struct FixMessage {
    /// Flat byte buffer containing all field values concatenated in parse order.
    /// FixField stores (start, len) offsets into this arena —
    /// one allocation per message instead of one CompactString per field.
    pub arena:  Vec<u8>,
    pub fields: Vec<FixField>,
    // hot extracted fields stay as CompactString for O(1) table view access:
    pub sender:  CompactString,
    pub time:    CompactString,
    // ...
}

pub struct FixField {
    pub tag:         u16,  // 2 bytes
    pub value_len:   u16,  // 2 bytes  ← ordered for zero padding
    pub value_start: u32,  // 4 bytes
}
// Total: 8 bytes — vs 32 bytes before = 4× smaller
```

`FixField` is now **8 bytes**. The same trick Kelley showed: store an index into a flat buffer, not a copy of the data.

To access a value:

```rust
impl FixField {
    #[inline]
    pub fn value_in<'a>(&self, arena: &'a [u8]) -> &'a str {
        let slice = &arena[self.value_start as usize..][..self.value_len as usize];
        // SAFETY: FIX protocol is 7-bit ASCII ⊆ valid UTF-8.
        unsafe { std::str::from_utf8_unchecked(slice) }
    }
}

// Usage:
let val = field.value_in(&msg.arena);
```

**Impact:**

- `FixField`: 32 bytes → 8 bytes = **4× reduction**
- 1M messages × 20 fields: **~480 MB less field storage**
- 4× fewer cache lines touched when scanning `msg.fields` to find a tag
- The `tag_val` helpers in session_health, fill_quality, etc. all iterate fields — they're now 4× more cache-friendly

However, the initial implementation caused a **performance regression**: 104ms → 111ms.

---

### Phase 7b — Understanding the Regression

The first arena implementation appended values one by one during parsing:

```rust
fn parse_single_simd(raw: &[u8]) -> FixMessage {
    let mut msg = FixMessage {
        fields: Vec::with_capacity(24),
        arena:  Vec::with_capacity(raw.len()),  // empty, grows as we parse
        ..Default::default()
    };
    // ...
}

// In apply_token (called ~20× per message):
let value_start = msg.arena.len() as u32;
msg.arena.extend_from_slice(val_b);  // ← 20 extend_from_slice calls per message
let value_len = val_b.len() as u16;
```

Two problems:

1. **Two allocations per message**: `fields: Vec::with_capacity(24)` + `arena: Vec::with_capacity(raw.len())` = 2× malloc calls. For 1M messages: **2 million allocations** instead of 1 million.
2. **Per-field arena writes**: 20 `extend_from_slice` calls per message = 20M small writes, each reading `msg.arena.len()` and then incrementing it.

**Lesson #7: Counting allocations matters. Two mallocs per item in a 1M-item loop is 2M mallocs — that adds up even with a fast allocator like mimalloc.**

### Fix: Copy Raw Bytes Once, Not Per Field

The insight: `raw` (the per-message byte slice from the mmap) already contains every value byte in exactly the right order. Instead of appending value slices one-by-one, copy the whole slice at once:

```rust
fn parse_single_simd(raw: &[u8]) -> FixMessage {
    let mut msg = FixMessage {
        fields: Vec::with_capacity(24),
        arena:  raw.to_vec(),  // ONE memcpy of the full message slice
        ..Default::default()
    };
    fill_message(raw, &mut msg);
    msg
}
```

Now in `apply_token`, the arena is already populated. We compute the value offset directly — pure arithmetic, zero copies:

```rust
// apply_token(raw, start, end, msg)
// raw[start..end] is the current "tag=value" token.
// The arena is a verbatim copy of raw, so offsets are identical.
let value_start = (start + eq_index + 1) as u32;  // absolute offset in raw = offset in arena
let value_len   = val_b.len() as u16;
// No extend_from_slice — value bytes are already in msg.arena!
msg.fields.push(FixField { tag: tag_num, value_len, value_start });
```

**Result:**

| Benchmark | After DOD regression | After arena fix |
|---|---|---|
| single simd_bytes | 323 ns | **257 ns** (−21%) |
| 100k simd_bytes | 1.88 GiB/s | **2.12 GiB/s** (+13%) |
| 1M simd_bytes | 111 ms | **101 ms** (−9%) |

We recovered the regression and then some — 101ms, right at the target.

**Lesson #8: The full-slice copy trick.** When you have a flat arena that is a verbatim copy of some input buffer, value offsets equal input offsets. You can replace N small `extend_from_slice` calls with one `raw.to_vec()`, and compute offsets as arithmetic instead of tracking the arena's `len()` after each append.

---

## Phase 8 — Parallel Boundary Scan (101ms → 85ms)

We were at 101ms and profiling showed that `message_start_offsets` — the serial memmem scan to find all "8=FIX" boundaries — was taking ~19ms on its own:

```text
195 MB input / ~10 GB/s memmem throughput ≈ 19ms
```

19ms of work that only one CPU core was doing, while the other 9 sat idle.

The existing code ran this scan serially because we had learned (the hard way in Phase 4) that mixing boundary detection with parsing causes double-scanning. But the boundary scan alone is embarrassingly parallel — each byte is independent.

### Fix: Parallel memmem Scan with Ownership Regions

The trick to parallelising a scan without duplicates: divide the input into N chunks, have each worker scan its chunk plus a 4-byte overlap (since "8=FIX" is 5 bytes), but only **keep** markers that fall within the worker's own ownership region `[own_start, own_end)`:

```rust
const OVERLAP: usize = 4;  // "8=FIX" is 5 bytes → max 4 bytes can straddle a boundary
let chunk_size = (input.len() + thread_count - 1) / thread_count;

let per_chunk: Vec<Vec<u32>> = (0..thread_count).into_par_iter().map(|i| {
    let own_start = i * chunk_size;
    let own_end   = ((i + 1) * chunk_size).min(input.len());
    let scan_end  = (own_end + OVERLAP).min(input.len());

    let chunk = &input[own_start..scan_end];
    let mut v = Vec::with_capacity(chunk_size / AVG_MSG_BYTES);

    for pos in memmem::find_iter(chunk, b"8=FIX") {
        let abs = own_start + pos;
        // Last worker claims everything; all others only claim [own_start, own_end).
        if abs < own_end || i + 1 == thread_count {
            v.push(abs as u32);
        }
    }
    v
}).collect();

// Workers process sequential regions → result is already sorted. Just flatten.
let mut offsets: Vec<u32> = Vec::with_capacity(estimated_total);
for chunk in per_chunk { offsets.extend_from_slice(&chunk); }
```

Why does this work without a sort? Each worker processes `[i*chunk_size, (i+1)*chunk_size)` in index order. Worker 0's offsets are all less than worker 1's, which are all less than worker 2's, and so on. Flattening in order gives a sorted result for free.

**Result:**

| Benchmark | Before | After | Change |
|---|---|---|---|
| single simd_bytes | 257 ns | **254 ns** | flat |
| 100k simd_bytes | 2.12 GiB/s | **2.56 GiB/s** | **+21%** |
| **1M simd_bytes** | 101 ms | **85 ms** | **−16%** |

**Target achieved: 87ms steady state for 1 million messages on 195 MB of SOH-delimited FIX data (best single run measured: 85ms; Criterion noise on 10 samples is ±3–5%).**

**Lesson #9: Serial scans are a hidden bottleneck in parallel pipelines. A single-threaded memmem scan over 195 MB was consuming 19ms — nearly 20% of total parse time — while 9 cores idled.**

**Lesson #10: Parallel scans need ownership regions, not sorting.** Assign each worker a non-overlapping ownership range. Let workers scan slightly past their boundary (for pattern straddle), but discard anything outside their range. Sequential assignment means results are already sorted — no sort step needed.

---

## Phase 9 — The Two-Path Problem (13.6ms → 10.4ms on 100k)

After hitting 85ms on 1M messages, we looked at a part of the codebase we hadn't touched: the `parse_all` str path. This is the path used when the user pastes FIX data into the UI rather than loading a file. The benchmark showed something alarming:

| Path | 100k messages | Throughput |
|---|---|---|
| `parse_all_simd_bytes` | 7.9ms | 2.4 GiB/s |
| `parse_all` (str) | **13.6ms** | **1.4 GiB/s** |

The str path was **1.7× slower** — while the 1M file benchmark was the headline, this is the path users hit when they paste a log snippet. 13ms for 100k messages is noticeably slow in a responsive UI.

### Why Two Parse Paths Existed

`parse_all` handles `&str` input (pasted text), normalises SOH/`\x01`/`^A` delimiters to pipe, then calls `parse_single` per message. `parse_single` was written as an independent function with its own parse loop — no NEON, per-field `arena.extend_from_slice`, scalar `memchr_iter` — the old approach we had optimized away in the SIMD path months earlier. The two paths had quietly diverged.

Looking at the `parse_single` source:

```rust
fn parse_single(raw: &str) -> FixMessage {
    let mut msg = FixMessage {
        fields: Vec::with_capacity(AVG_FIELDS_PER_MSG),
        arena:  Vec::with_capacity(raw.len()),  // ← empty arena, grows per-field
        ..Default::default()
    };
    for end in memchr_iter(b'|', bytes).chain(std::iter::once(bytes.len())) {
        let token = raw[start..end].trim();
        // ...
        let value_start = msg.arena.len() as u32;
        msg.arena.extend_from_slice(value.as_bytes());  // ← 20× per message
        // ...
    }
}
```

Every optimization from phases 7 and 8 (the pre-copy arena trick, the NEON loop) was sitting right next door in `parse_single_simd`, unused by this path. We had fixed the regression for the bytes path but never applied it to the str path.

### Fix: Collapse parse_single to a One-Liner

`parse_single_simd` already handles both SOH and pipe delimiters — `is_delimiter` checks `b == 0x01 || b == b'|'`. The input to `parse_single` is always pipe-normalized. So:

```rust
/// Parse a single pipe-delimited FIX message string into a FixMessage.
/// Delegates to parse_single_simd so both paths share the same NEON/AVX2
/// hot loop and the pre-copy arena trick.
#[inline]
fn parse_single(raw: &str) -> FixMessage {
    parse_single_simd(raw.as_bytes())
}
```

50 lines of duplicated code → 3 lines. The old `parse_single` loop, the `memchr_iter` import, the separate `arena.extend_from_slice` chain — all gone.

**Results:**

| Benchmark | Before | After | Change |
|---|---|---|---|
| single `parse_all_pipe` | 575 ns | **279 ns** | **−51.7%** |
| 100k `parse_all_pipe` | 13.6 ms | **10.4 ms** | **−23.5%** |
| 100k throughput | 1.4 GiB/s | **1.86 GiB/s** | **+33%** |

A 50-line delete and 3-line add. No new algorithm, no new data structure.

The single-message `parse_all_pipe` benchmark now shows 279ns — nearly matching the 254ns of `parse_all_simd_bytes`. The remaining 25ns gap is the overhead of `normalize_delimiters` (an extra scan for SOH bytes) and `str_message_slices` (an extra memmem pass to split the input).

**Lesson #11: Optimizations applied to one path don't automatically propagate to another.** If the same logical operation has two implementations, they will drift. The str path silently carried six months of "pre-optimization" while the bytes path got all the improvements. The fix was to delete the duplication.

---

## Phase 10 — The Memory Bandwidth Ceiling (analysis)

After Phase 9 we took stock. The 1M benchmark was holding steady at **~87ms** (within noise of the 85ms we'd measured earlier — Criterion noise on 10 samples with a complex parallel workload is ±3–5%). The 100k parse_all_pipe came down to 10.4ms. Progress.

But the further we looked, the more we found we were bumping into something that can't be optimized away: **memory bandwidth**.

### What Happens to Memory During a 1M Parse

Tracing the memory operations for 1M messages on 195 MB of input:

1. **Read 1**: mmap gives us 195 MB of input pages. `message_start_offsets` scans all 195 MB to find "8=FIX" boundaries → **195 MB read**
2. **Write 1**: 1M × `raw.to_vec()` copies the per-message slice into its arena → **195 MB written** (total — every byte of input is copied once into an arena)
3. **Read 2**: NEON parse loop reads the arena (verbatim copy of raw) → **195 MB read** again
4. **Write 2**: 1M × `fields: Vec::with_capacity(24)` pushes ~20 `FixField` entries per message → **~160 MB written** (1M × 20 × 8 bytes per FixField)
5. **Write 3**: 11 `CompactString` hot fields per message → **~264 MB written** (1M × 11 × 24 bytes)
6. **Write 4**: `Vec<FixMessage>` output collection → **1M × FixMessage structs** = roughly **350 MB written**

Total memory traffic: roughly **1.4 GB of reads and writes** for 195 MB of input. That's a 7× amplification factor.

On an M1 Max, unified memory bandwidth is ~400 GB/s. 1.4 GB / 400 GB/s ≈ **3.5ms** theoretical minimum. We're at 87ms — so memory bandwidth alone isn't the wall; we have room. But we can see where the copies are piling up.

### The Arena Copy

The biggest single source of write amplification is step 2: `raw.to_vec()`. Every byte of input is copied into a per-message arena. 195 MB of input produces 195 MB of arena data, spread across 1M individual `Vec<u8>` allocations.

An alternative architecture would share one arena across all messages: a single `Arc<[u8]>` wrapping the entire 195 MB input, with each `FixMessage` storing `arena_offset: u32` so `field.value_in(shared_arena)` still works. Zero copy of the input data.

This would save:

- 195 MB of memory writes (the `raw.to_vec()` calls)
- 1M `Vec::new()` allocations (for the arena Vecs)
- The corresponding 1M `drop` calls when messages are freed

But it requires a lifetime thread through `FixMessage` or an `Arc` to keep the shared buffer alive. The Dioxus UI state stores `Vec<FixMessage>` — adding a lifetime or `Arc` touches every consumer. We considered it and parked it: the architectural cost is high, the theoretical win is real but bounded (195 MB at 400 GB/s is only ~0.5ms), and the code simplicity matters.

**Lesson #12: Know your amplification factor.** Count every read and write that flows from your primary input. When you see 7× amplification on 195 MB of input, you know exactly what ceiling you're approaching — and you can decide whether shrinking the factor is worth the architectural cost.

### The Parallel Efficiency Gap

There's another ceiling: parallel efficiency. We have 10 cores, each `FixMessage` takes ~254ns to parse sequentially. For 1M messages:

- **Ideal parallel time**: 1M × 254ns / 10 cores = **25.4ms**
- **Actual time**: ~87ms
- **Parallel efficiency**: 25.4ms / 87ms = **~29%**

That's not great. Where does the 71% overhead go?

1. **Thread pool overhead** — Rayon wakes sleeping worker threads; the first batch always pays this tax
2. **Memory contention** — 10 cores simultaneously writing to separate but nearby arenas causes cache-line thrashing at the allocator level (even mimalloc's thread-local arenas serialize when requesting new pages from the OS)
3. **Serial phases** — `message_start_offsets` is parallel but has a sequential flatten step; the final `.collect()` is sequential
4. **Load imbalance** — `par_windows(2)` distributes equal-count windows across workers, but real FIX messages vary in size. A thread assigned 100 large ExecutionReports does more work than a thread assigned 100 short Heartbeats

Perfect parallelism on a memory-allocation-heavy workload over 10 cores is never 10×. The realistic ceiling is probably 5–6× on this workload. We're at 87ms/25.4ms = 3.4×. There's headroom, but it would require deeper changes: pre-allocating a shared pool for all `FixField` arrays, eliminating per-message allocations, and changing how the UI receives results (streaming rather than a complete `Vec<FixMessage>`).

**Lesson #13: Parallel efficiency on allocation-heavy workloads is bounded by the allocator, not by the algorithm.** Each `Vec::with_capacity` is a write to shared allocator state. Even with mimalloc's thread-local arenas, 10 threads each doing 100k allocations will serialize on arena page requests. The fix is to pre-allocate — but that often means changing your data structures and API.

---

## Phase 11 — The Small-Input Tax

After Phase 10's analysis, we looked at the paste path from a different angle. The `message_start_offsets` function introduced in Phase 8 always used the parallel rayon path — even for tiny inputs. For the 1M benchmark that was the right call, but what about someone pasting 50 messages into the UI?

A 50-message snippet is maybe 8 KB. `PARALLEL_SCAN_MIN_BYTES` wasn't a concept yet — every call to `parse_all_simd_bytes`, no matter how small the input, paid the full rayon overhead: wake sleeping workers from their thread pool, distribute an 8 KB slice (1 chunk per worker on 10 cores means 9 workers get essentially nothing), collect results. The rayon bookkeeping alone is tens of microseconds.

### Fix: Adaptive Serial/Parallel Threshold

```rust
/// Threshold above which the boundary scan is parallelised across rayon workers.
/// Below this, the serial memmem scan is faster than spawning rayon tasks.
const PARALLEL_SCAN_MIN_BYTES: usize = 2 * 1024 * 1024; // 2 MB

fn message_start_offsets(input: &[u8]) -> Vec<u32> {
    let thread_count = rayon::current_num_threads().max(1);

    if input.len() < PARALLEL_SCAN_MIN_BYTES || thread_count == 1 {
        // Serial path — no rayon overhead for small inputs.
        let capacity = (input.len() / AVG_MSG_BYTES).max(4);
        let mut offsets = Vec::with_capacity(capacity);
        for pos in memmem::find_iter(input, b"8=FIX") {
            offsets.push(pos as u32);
        }
        return offsets;
    }

    // Parallel path (as before) ...
}
```

The threshold is 2 MB. The serial memmem scan runs at ~10 GB/s — scanning 2 MB takes ~200 µs, well below the rayon spawn overhead. Above 2 MB there are enough messages to justify distributing across cores. Below 2 MB, the serial path wins unconditionally.

The 1M benchmark file is 195 MB — not affected. But any call from a small file or paste path now skips rayon entirely.

**Lesson #16: Parallel primitives have a fixed setup cost.** For rayon, that cost is on the order of tens of microseconds for waking workers and distributing tasks. Any input that can be processed serially in less time than that cost should stay serial. Use a threshold — measure where the crossover is, pick a conservative value below it, and put it in a named constant so the intent is clear.

---

## Phase 12 — Eliminating the Magic Number and Hardening the NEON Path

The last `parser.rs` change was small but worth documenting: two inline copies of `0x8040201008040201` in `simd_parse_neon` — one for `weights_lo`, one for `weights_hi`.

```rust
// Before — duplicated inline.
let weights_lo = vcreate_u8(0x8040201008040201_u64); // lanes 0-7:  [1,2,4,8,16,32,64,128]
let weights_hi = vcreate_u8(0x8040201008040201_u64); // lanes 8-15: same weights
```

These must always be identical. The NEON movemask emulation depends on both halves using the same powers-of-two weights — if they ever diverge (copy-paste drift, a hasty fix), the bitmask for the high 8 lanes becomes garbage and the parser silently produces wrong field offsets.

```rust
// After — one definition, impossible to drift.
const LANE_WEIGHTS: u64 = 0x8040201008040201;
let weights_lo = vcreate_u8(LANE_WEIGHTS);
let weights_hi = vcreate_u8(LANE_WEIGHTS);
```

We also added a suite of negative and boundary tests that were missing:

```rust
#[test]
fn test_token_without_equals_is_skipped() {
    // "GARBAGE" contains no '=' — apply_token must return early without panic.
    let input = b"8=FIX.4.4|GARBAGE|35=A|10=001|";
    let msgs = parse_all_simd_bytes(input);
    assert_eq!(msgs[0].msg_type_raw, "A");  // valid fields still parsed
}

#[test]
fn test_truncated_message_no_panic() {
    // A message that ends mid-field must not panic.
    let input = b"8=FIX.4.4|9=61|35=";
    let msgs = parse_all_simd_bytes(input);
    assert_eq!(msgs[0].msg_type_raw, "");  // empty value, no crash
}
```

These tests document the parser's error-tolerance contract: malformed tokens are skipped silently, truncated messages produce partial results without panicking. Before this, these invariants were assumed but untested — any future refactor of `apply_token` could break them invisibly.

**Lesson #17: Two copies of a magic constant are one defect waiting to happen.** Name it. The original duplication went unnoticed because the numbers looked identical in hex. A named constant makes the identity explicit and enforced by the compiler.

**Lesson #18: Negative tests document contracts, not just correctness.** The real-world FIX corpus contains truncated messages (session drops), corrupt frames (network errors), and ambiguous tokens (vendor extensions). Tests that assert "this doesn't crash" are as valuable as tests that assert "this returns X".

---

## Final Results

All optimizations stacked, on a 195 MB SOH-delimited file of 1M FIX 4.4 messages.

*Note on the numbers: intermediate phase numbers throughout this post are approximate — measurements were taken at different points in the development timeline against slightly different test harnesses, and Criterion noise on this workload is ±3–5%. The per-phase deltas in the second table below reflect each change's impact measured in relative isolation; they do not sum to the total improvement because each optimization changed what the next bottleneck was.*

| Metric | Baseline | Final | Improvement |
|---|---|---|---|
| **1M messages** | 263ms | **87ms** | **3.0× faster** |
| **100k messages (SIMD)** | ~217ms | **7.9ms** | **27× faster** |
| **100k messages (str)** | ~217ms | **10.4ms** | **21× faster** |
| **Single message** | ~4µs | **254ns** | **16× faster** |
| **Throughput (1M)** | ~720 MiB/s | **2.2 GiB/s** | **+215%** |

The optimizations in sequence:

| Phase | Change | Δ on 1M |
|---|---|---|
| 1 | CompactString, memchr, Rayon, single-pass normalize | −123ms |
| 2 | ARM NEON SIMD (was scalar on M1) | −60ms |
| 3 | mimalloc global allocator | −12ms |
| 4 | Serial scan + `par_windows(2)` on `Vec<u32>` offsets | −30ms |
| 5 | **`FixField.tag: u16`** (jump table, −15M CompactString constructions) | **−34ms** |
| 6 | Startup prewarm (perceived first-load latency) | −140ms perceived |
| 7 | **DOD: `FixField` 32 → 8 bytes (arena-indexed values)** | −6ms (memory) |
| 8 | **Arena pre-copy (one memcpy, zero per-field writes)** | **−16ms** |
| 9 | **Parallel memmem boundary scan** | **−16ms** |
| 10 | **Unified str path via `parse_single_simd`** | −3.2ms on 100k str path |
| 11 | **Adaptive serial/parallel threshold (`PARALLEL_SCAN_MIN_BYTES`)** | negligible on 1M; eliminates rayon overhead for small inputs |
| 12 | **`LANE_WEIGHTS` constant; negative boundary tests** | correctness + maintainability |

---

## Lessons Learned

**1. Measure on the actual hardware.**
AVX2 SIMD was in the code. It wasn't running. One `cfg` guard silently disabled it on every M1 machine we shipped to.

**2. Profile before optimizing.**
We spent effort on `MADV_SEQUENTIAL` and a page pre-fault loop. Neither helped — the real bottleneck was the parse itself, not I/O. Measuring correctly pointed us to the right target.

**3. NEON has no movemask — emulate carefully.**
The powers-of-two weight trick is correct and fast. Getting the weights wrong gives silently broken results that only show up as wrong values deep in test assertions.

**4. Data layout beats algorithm.**
Changing `tag: CompactString` → `tag: u16` was not a clever algorithm. It was a type annotation. It was the largest single-phase improvement in the entire project — a 30% drop within that phase alone (113ms → 79ms), achieved by removing 15M short-string constructions from the hot path.

**5. Separate phases in a parallel pipeline.**
Combining boundary detection with parsing made each thread scan every byte twice. Separate serial scan + parallel parse is faster because the serial scan is O(n) at ~10 GB/s and not the bottleneck — the per-message parse is.

**6. Serial operations inside a parallel pipeline are invisible bottlenecks.**
The memmem boundary scan was taking 19ms while 9 threads idled. It only became visible once everything else was fast enough that 19ms was 20% of the total.

**7. Use the smallest integer type that fits.**
`Vec<u32>` offsets vs `Vec<usize>` pointers: 4× smaller working set for the same data. This matters when you're distributing 1M entries across rayon workers.

**8. "Use indexes instead of pointers" (Andrew Kelley).**
`FixField` storing `(u32 start, u16 len)` arena offsets instead of a `CompactString` value: 32 bytes → 8 bytes per field. 1M messages × 20 fields = 480 MB of memory saved. 4× fewer cache lines when scanning `msg.fields`.

**9. The arena memcpy trick.**
When you have a flat arena that mirrors the input buffer, value offsets equal input offsets. One `raw.to_vec()` replaces 20 per-field `extend_from_slice` calls and eliminates the bookkeeping of tracking the arena's growing length.

**10. Parallel scans with ownership regions don't need a sort.**
Assign workers sequential chunks. Let each scan slightly past its boundary for pattern straddle. Discard anything outside the worker's ownership range. Flatten in order — result is already sorted.

**11. Optimizations applied to one path don't propagate to another.**
The str parse path silently carried the old, slow design while the bytes path received every improvement. Collapsing both paths to a single implementation fixed six months of silent drift in one commit.

**12. Know your amplification factor.**
Count every read and write flowing from your primary input. 195 MB of input → ~1.4 GB of total memory traffic is a 7× amplification. Identifying where each copy comes from lets you decide which ones are worth eliminating.

**13. Parallel efficiency on allocation-heavy workloads is bounded by the allocator.**
Each `Vec::with_capacity` is a write to shared allocator state. Even with mimalloc, 10 threads each doing 100k allocations will contend. The fix is pre-allocation — but that usually requires changing data structures and API contracts.

**14. The warm-up gap is real.**
Benchmark numbers and first-run app numbers are different problems. Thread pools, allocator arenas, and instruction caches all need one touch before they perform at full speed.

**15. The SSD floor is real too.**
195 MB at ~5 GB/s NVMe = ~39ms of unavoidable I/O on a cold read. No software optimization can parse bytes that haven't arrived. Cold-load time will always be warm-load time + I/O time.

**16. Parallel primitives have a fixed setup cost.**
Rayon workers need to wake, be assigned work, and synchronize results — a cost on the order of tens of microseconds regardless of input size. Any workload that can complete serially faster than that overhead should stay serial. Measure the crossover point, pick a threshold just below it, and name it.

**17. Two copies of a magic constant are one defect waiting to happen.**
The NEON movemask emulation uses `0x8040201008040201` for both halves of the 128-bit register. Both must always be identical — they encode the same mathematical property. A named constant makes that identity enforced by the compiler rather than assumed by the reader.

**18. Negative tests document contracts, not just correctness.**
Tests that assert "this doesn't crash" are as valuable as tests that assert "this returns X". The parser's tolerance for truncated messages and malformed tokens was implicit; making it explicit in tests means future refactors can't silently break it.

---

## Try It

AI FIX Parser is available at [aifixparser.com](https://aifixparser.com). Drop any FIX log file — pipe-delimited or SOH — and see your messages parsed and displayed in under a second even for multi-million-message files.

The parser is written in Rust with Dioxus for the UI. If you have a FIX file that breaks it or takes longer than expected, open an issue.
