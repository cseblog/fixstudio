# How We Parse 1 Million FIX Messages in Under 100ms

*A story of SIMD, wrong assumptions, and a type change that changed everything.*

---

## Background

[AI FIX Parser](https://aifixparser.com) is a desktop app that lets traders and engineers load FIX protocol log files and inspect, filter, and analyze every message. FIX logs from busy trading sessions can have millions of messages. If parsing takes seconds, the app feels broken. We set ourselves a target: **parse 1 million messages in under 100ms** on a developer's laptop.

This post documents every optimization we tried, what we measured, and — critically — what we learned when our assumptions turned out to be wrong.

---

## The Setup

- **Machine:** Apple M1 Max, 10 cores (8 performance + 2 efficiency)
- **Test file:** 1,000,000 FIX 4.4 messages, SOH-delimited, 141 MB
- **Mix:** 60% NewOrderSingle, 20% ExecutionReport, 20% OrderCancelRequest
- **Benchmark tool:** Criterion.rs, 10 samples per bench, `profile.bench` opt-level 3

**Baseline: 263ms — 1M messages, single pass, scalar parser.**

Everything below is the story of getting to **79ms**.

---

## Phase 1 — The Obvious Stuff (217ms → 36ms for 100k, but let's talk 1M)

Before the performance sprint, the parser was already through three rounds of basic optimization:

| Change | Effect |
|---|---|
| `String` → `CompactString` for fields | Eliminated ~3M heap allocs per 100k messages (tags/timestamps fit inline) |
| `memchr`/`memmem` for delimiter search | SIMD-accelerated scan, ~16–32 bytes/cycle |
| Rayon `par_iter` across messages | All 10 cores doing useful work |
| `normalize_delimiters` single-pass | SOH input: 1× alloc instead of 3× chained `.replace()` |
| `opt-level = 3`, `panic = "abort"` | LLVM vectorization + unrolling, ~10–30% uplift |
| Lazy `value_description` | Removed 1.5M dictionary lookups from the hot parse path |

These got us from ~217ms to a reasonable starting point. But for 1M messages over 141MB, we measured **263ms** — still too slow.

---

## Phase 2 — The Assumption That Was Wrong

Our parser had a `simd_parse_avx2` function — 32-byte AVX2 vectorized scanning for delimiters. We assumed it was running.

**It wasn't.**

AVX2 is x86. We were running on an **Apple M1 Max (ARM aarch64)**. The `#[cfg(target_arch = "x86_64")]` guard meant we fell back to a scalar byte loop for every single delimiter scan. With 15–30 fields per message and 1M messages, that's 15–30 million scalar iterations.

**Lesson #1: Always verify which code path is actually executing on your target hardware.**

### Fix: ARM NEON SIMD

We implemented `simd_parse_neon` — a 128-bit vectorized delimiter scanner using ARM NEON intrinsics.

```rust
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn simd_parse_neon(raw: &[u8], msg: &mut FixMessage) {
    use std::arch::aarch64::*;
    let soh_vec  = vdupq_n_u8(0x01);
    let pipe_vec = vdupq_n_u8(b'|');
    // Powers-of-two weights for movemask emulation: [1,2,4,8,16,32,64,128]
    let weights = vcreate_u8(0x8040201008040201_u64);
    // process 16 bytes per iteration ...
}
```

ARM has no hardware `movemask` instruction (unlike x86 `_mm256_movemask_epi8`). We emulated it: compare 16 bytes against SOH and pipe simultaneously, AND the result with a power-of-two weight vector, sum horizontally with `vaddv_u8` — the result is a 16-bit bitmask of delimiter positions.

**But we got the weights wrong on the first attempt.**

We used `[1,2,3,4,5,6,7,8]` (sequential) instead of `[1,2,4,8,16,32,64,128]` (powers of two). The bitmask positions were garbage. The parser read wrong byte offsets, `b - b'0'` underflowed, and tests panicked.

**Lesson #2: NEON movemask emulation requires exact powers of two as weights. Sequential weights produce wrong bit positions.**

Correct weights: `vcreate_u8(0x8040201008040201_u64)` — little-endian encoding of `[1,2,4,8,16,32,64,128]`.

---

## Phase 3 — Allocator Contention

With NEON running, 10 Rayon threads were all allocating `Vec<FixField>` simultaneously. The system allocator uses a global mutex for large allocations — under high parallelism, threads queue behind each other waiting for the lock.

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

## Phase 4 — Better Parallelism

The original parallel split was:

```rust
// Find all "8=FIX" boundaries, collect slices, then parallel-map
let slices = message_slices_bytes(input);  // serial O(n) scan
slices.par_iter().map(parse_single_simd).collect()
```

Two problems:
1. The serial boundary scan was a bottleneck before parallelism even started.
2. `N threads` of work means Rayon gets `N` tasks — if any task runs long, others sit idle (load imbalance).

**Fix: parallel boundary detection + 8× chunk multiplier**

```rust
let num_chunks = n * 8;  // 8× finer granularity for work-stealing
let chunk_size = (input.len() + num_chunks - 1) / num_chunks;

// Find N chunk-start positions in serial (fast: memmem finds "8=FIX" within ~135 bytes)
let mut starts: Vec<usize> = std::iter::once(0)
    .chain((1..num_chunks).filter_map(|i| {
        let nominal = i * chunk_size;
        memmem::find(&input[nominal..], b"8=FIX").map(|p| nominal + p)
    }))
    .collect();
starts.push(input.len());

// Each thread independently scans AND parses its chunk
starts.par_windows(2)
    .flat_map_iter(|w| parse_chunk(&input[w[0]..w[1]]))
    .collect()
```

The `memmem::find` calls are fast — each searches just `chunk_size / num_chunks` bytes on average before finding the next message header. With 8× chunks, Rayon's work-stealing keeps all 10 cores busy even when individual chunks vary in message density.

**Tuning the multiplier:**

| Multiplier | Time |
|---|---|
| 1× (= N threads) | 142ms |
| 4× | 122ms |
| 8× | 114ms |
| 16× | 113ms (plateau) |

We settled on **8×** — diminishing returns beyond that.

---

## Phase 5 — The Single Biggest Win

After all the above, we were at **113ms**. Still above target. We profiled and found:

Every field in every message was creating a `CompactString` from a tag number string:

```rust
// The field model:
pub struct FixField {
    pub tag: CompactString,  // "35", "49", "52" — always 1–5 digits
    pub value: CompactString,
}
```

For 1M messages with ~15 fields each, that's **15 million `CompactString::from(tag_str)` calls** — each touching the allocator or inline buffer, building a string we immediately pattern-match and throw away.

**Fix: `FixField.tag` as `u16`**

```rust
pub struct FixField {
    pub tag: u16,           // 2 bytes, zero allocation
    pub value: CompactString,
}
```

This required changing:
- `src/model.rs` — field type
- `src/parser.rs` — `apply_token` uses `tag_to_u16(tag_bytes)` directly
- `src/dictionary.rs` — four functions changed from `tag: &str` to `tag: u16`; all string match arms (`"35" =>`) converted to integer arms (`35 =>`)
- `src/components/detail.rs` — seven call sites updated

The integer match compiles to a **jump table** — O(1) dispatch vs O(tags) string comparison. `FixField` shrank from 48 bytes to 32 bytes (33% less data written per field). And 15M CompactString constructions simply ceased to exist.

**Result: 113ms → 79ms. A 30% drop from a type annotation change.**

**Lesson #3: Data layout is performance. A struct field type that seems cosmetic can dominate your hot path.**

---

## Phase 6 — Prewarm on Startup

The parser was fast in benchmarks. But the first file load in the actual app felt slower. Profiling revealed three cold-start costs paid once:

1. **Rayon thread spawn:** Rayon creates its thread pool lazily. First `par_windows` call blocked while macOS spawned 9 threads (20–80ms).
2. **mimalloc TLS initialization:** Each worker thread initializes its arena on first allocation.
3. **CPU instruction cache:** `simd_parse_neon`, `apply_token`, `tag_to_u16` — hot functions evicted from i-cache since app launch.

**Fix: prewarm at `main()` startup**

```rust
fn prewarm() {
    // Force thread pool creation immediately
    rayon::ThreadPoolBuilder::new().build_global().ok();

    // Touch the hot code paths with a tiny parse
    let dummy = b"8=FIX.4.2\x019=5\x0135=D\x0110=000\x01";
    let _ = parser::parse_all_simd_bytes(dummy);
}

fn main() {
    prewarm();
    // ... launch UI
}
```

This runs before the window opens — invisible to the user (~5ms). First real file load improved from ~250ms to ~110ms (first ever cold SSD read) and is effectively indistinguishable from warm loads thereafter.

---

## Final Results

| Benchmark | Before | After | Improvement |
|---|---|---|---|
| 1M messages (141MB) | 263ms | **79ms** | **3.3× faster** |
| 100k messages (15.8MB) | ~217ms | **9.2ms** | **23× faster** |
| Single message | ~4µs | **348ns** | **11× faster** |
| Throughput (1M) | 516 MiB/s | **1.67 GiB/s** | **+224%** |

All optimizations stacked:

| Optimization | When | Δ |
|---|---|---|
| CompactString, memchr, Rayon, single-pass normalize | Phase 1 | ~−140ms |
| ARM NEON SIMD (was scalar loop on M1) | Phase 2 | ~−60ms |
| mimalloc global allocator | Phase 3 | ~−10ms |
| par_windows + 8× chunk multiplier | Phase 4 | ~−30ms |
| **`FixField.tag: u16`** | **Phase 5** | **~−34ms** |
| Startup prewarm | Phase 6 | perceived ~−80ms |

---

## Lessons Learned

**1. Measure on the actual hardware.**
AVX2 SIMD was in the code. It wasn't running. One `cfg` guard silently disabled it on every M1 machine we shipped to.

**2. Profile before optimizing.**
We spent effort on `MADV_SEQUENTIAL` and a page pre-fault loop. Neither helped — the real bottleneck was the parse itself, not I/O. Measuring correctly pointed us to the right target.

**3. Data layout beats algorithm.**
Changing `tag: CompactString` → `tag: u16` was not a clever algorithm. It was a type annotation. It gave us 30% of our total improvement.

**4. NEON has no movemask — emulate carefully.**
The powers-of-two weight trick is correct and fast. Getting the weights wrong gives silently broken results that only show up as bounds panics in tests.

**5. The warm-up gap is real.**
Benchmark numbers and first-run app numbers are different problems. Thread pools, allocator arenas, and instruction caches all need one touch before they perform at full speed.

**6. The SSD floor is real too.**
141MB at ~5GB/s NVMe = ~28ms of unavoidable I/O. No software optimization can parse bytes that haven't arrived. Cold-load time will always be warm-load time + I/O time.

---

## Try It

AI FIX Parser is available at [aifixparser.com](https://aifixparser.com). Drop any FIX log file — pipe-delimited or SOH — and see your messages parsed and displayed in under a second even for multi-million-message files.

The parser is written in Rust with Dioxus for the UI. If you have a FIX file that breaks it or takes longer than expected, open an issue.
