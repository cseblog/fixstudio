# 1BRC Optimization Analysis — AI FIX Parser

Applying lessons from the 1 Billion Row Challenge (1BRC) top solutions to the FIX message parser.

---

## What We're Already Doing Right ✅

| Technique | Where |
|---|---|
| SIMD delimiter scan | `simd.rs` — AVX2 `cmpeq + movemask` |
| SIMD substring search | `memmem::find_iter` for `8=FIX` boundaries |
| Rayon parallel map | `slices.into_par_iter()` |
| SSO strings | `CompactString` avoids heap for ≤24-byte values |
| Zero-copy fast path | `Cow::Borrowed` when input is already pipe-delimited |
| Background dealloc | `offload_replace` drops old Vec on a thread |
| Release profile | `lto=true`, `codegen-units=1`, `panic=abort` |
| Capacity hints | `Vec::with_capacity(24)` for fields |

---

## The 5 Real Bottlenecks (Ranked by Impact)

---

### 🔴 #1 — `find_delimiters` Allocates a `Vec<usize>` Per Message

**The biggest single issue.**

```rust
// parse_single_simd today:
let delims = crate::simd::find_delimiters(raw);  // ← heap alloc per message
for end in delims.into_iter().chain(...) { ... }
```

For 1M messages this means **1M heap allocations** just for the delimiter index vecs.
Each vec holds ~20 elements × 8 bytes = 160 bytes → 160 MB of allocator churn.

**1BRC lesson**: top solutions never collect into an intermediate `Vec`.
They use a **streaming callback / inline scanner** — process each token the moment
a delimiter is found, never buffer positions.

**Fix — replace `Vec<usize>` with a closure callback:**

```rust
// Instead of: collect all positions → iterate
// Do: call on_field() the moment each delimiter is found
fn scan_and_parse(raw: &[u8], mut on_field: impl FnMut(&[u8])) {
    let soh_vec  = _mm256_set1_epi8(0x01_i8);
    let pipe_vec = _mm256_set1_epi8(b'|' as i8);
    let chunks   = raw.len() / 32;
    let mut start = 0usize;

    for i in 0..chunks {
        let chunk = _mm256_loadu_si256(...);
        let mut mask = _mm256_movemask_epi8(
            _mm256_or_si256(cmpeq(chunk, soh_vec), cmpeq(chunk, pipe_vec))
        ) as u32;
        let base = i * 32;
        while mask != 0 {
            let end = base + mask.trailing_zeros() as usize;
            on_field(&raw[start..end]);
            start = end + 1;
            mask &= mask - 1;
        }
    }
    // scalar tail + final field ...
}
```

**Expected gain: ~30–40% reduction in parse time for SOH input.**

---

### 🔴 #2 — File Loading Does a Full `Vec<u8>` Copy

```rust
let bytes = file.read().await;              // reads entire file into RAM
let content = String::from_utf8_lossy(&bytes); // may copy again
```

For a 166 MB SOH file this is one large `malloc + memcpy` before parsing even starts.

**1BRC lesson**: the fastest solutions all use **memory-mapped I/O** (`mmap`).
The OS page cache already holds the file; mmap makes you read directly from it — zero copy.

**Fix — use `memmap2` crate:**

```toml
# Cargo.toml
memmap2 = "0.9"
```

```rust
use memmap2::Mmap;

let file  = std::fs::File::open(&path)?;
let mmap  = unsafe { Mmap::map(&file)? };
// parse directly from &mmap[..] as &[u8] — no allocation, no copy
let is_soh = mmap.iter().take(4096).any(|&b| b == 0x01);
let parsed = if is_soh {
    parse_all_simd_bytes(&mmap)   // new &[u8] variant
} else {
    parse_all_bytes(&mmap)
};
```

**Expected gain: ~15–20% on large files (166 MB SOH benchmark).**

---

### 🟠 #3 — Entire Pipeline Uses `&str` Instead of `&[u8]`

```rust
// Today:
pub fn parse_all_simd(input: &str) -> Vec<FixMessage>
// message_slices takes &str, as_bytes() called inside
```

`message_slices` wraps `memmem` at the `&str` level, then `.as_bytes()` is called
again in `parse_single_simd`. The whole pipeline should work natively on `&[u8]`
to enable mmap integration and avoid the `from_utf8_lossy` gate.

**Fix:**

```rust
pub fn parse_all_simd_bytes(input: &[u8]) -> Vec<FixMessage>
fn message_slices_bytes(input: &[u8]) -> Vec<&[u8]>
```

`memmem::find_iter` already works on `&[u8]` — no change to the inner logic.

---

### 🟠 #4 — Tag Matching Uses `&str` Comparisons (String vs Integer Jump Table)

```rust
match tag {          // tag is &str
    "52" => msg.time = extract_time(value),
    "49" => msg.sender = CompactString::from(value),
    // ...
}
```

The compiler may produce a jump table, but each arm still does a length check
plus `memcmp`. FIX tags are 1–3 digit ASCII integers. Parsing the tag to `u16`
first means the `match` compiles to a **perfect integer jump table**, zero string comparisons.

**Fix:**

```rust
#[inline(always)]
fn tag_to_u16(b: &[u8]) -> u16 {
    // max 3 digits, no error branch needed for valid FIX
    b.iter().fold(0u16, |acc, &d| acc * 10 + (d - b'0') as u16)
}

match tag_to_u16(tag_b) {
    52  => msg.time       = extract_time(value_str),
    49  => msg.sender     = CompactString::from(value_str),
    56  => msg.target     = CompactString::from(value_str),
    35  => { msg.msg_type_raw = ...; msg.msg_type_label = msg_type_label(value_str); }
    11  => msg.cl_ord_id  = CompactString::from(value_str),
    54  => msg.side       = CompactString::from(side_label(value_str)),
    38  => msg.order_qty  = CompactString::from(value_str),
    55  => msg.symbol     = CompactString::from(value_str),
    58  => msg.text       = CompactString::from(value_str),
    150 => { /* exec type override */ }
    _   => {}
}
```

This also removes the `std::str::from_utf8(tag_b)` call — only `value_b` needs
UTF-8 conversion (for CompactString).

**Expected gain: ~10%.**

---

### 🟡 #5 — `trim_bytes` / `.trim()` on Every Token

```rust
// parse_single_simd:
let token = trim_bytes(&raw[start..end]);

// parse_single:
let token = raw[start..end].trim();
```

FIX protocol guarantees no whitespace inside messages. The trim is defensive against
user-pasted input but costs 2 pointer scans per token on every field.
For 20 fields × 1M messages = **40M unnecessary scans**.

**Fix:**

- Keep `.trim()` only in the scalar `parse_single` path (user-pasted textarea input).
- Remove `trim_bytes` from the SIMD path (`parse_single_simd`) — real log files don't have whitespace.

**Expected gain: ~5% on the SIMD path.**

---

### 🟡 #6 — Rayon Submits 1M Individual Tasks

```rust
slices.into_par_iter().map(parse_single_simd).collect()
```

For 1M messages, Rayon's work-stealing tree recursively splits this into chunks via
`log₂(N/min_chunk)` synchronization rounds. That is a lot of task overhead for
200-byte, sub-microsecond parse jobs.

**1BRC lesson**: split into exactly `num_cpus` chunks upfront. Each thread processes
its chunk independently and returns a `Vec<FixMessage>`. Merge at the end.

**Fix:**

```rust
let n = rayon::current_num_threads();
let chunk_size = (slices.len() + n - 1) / n;
slices
    .par_chunks(chunk_size)
    .flat_map(|chunk| chunk.iter().map(parse_single_simd))
    .collect()
```

This reduces task-submission overhead from O(log N) rounds to 1.

**Expected gain: ~5%.**

---

## Summary Table

| # | Problem | 1BRC Lesson | Expected Gain |
|---|---|---|---|
| 1 | `Vec<usize>` per message in `find_delimiters` | Never collect — stream with callbacks | **~30–40%** |
| 2 | `file.read()` full copy before parse | Memory-mapped I/O (`memmap2`) | **~15–20%** |
| 3 | `&str` pipeline over `&[u8]` | Work in bytes end-to-end | Enables #2 |
| 4 | String tag match | Parse tag as `u16`, integer jump table | **~10%** |
| 5 | `trim_bytes` on every token | FIX has no whitespace — remove from SIMD path | **~5%** |
| 6 | 1M Rayon tasks | `par_chunks(n_cpus)` | **~5%** |

---

## Projected Benchmark Impact

| Benchmark | Current | After #1+#2 | After all 6 |
|---|---|---|---|
| 1M SOH messages | ~330 ms | ~200 ms | ~150–170 ms |
| Throughput | ~500 MiB/s | ~800 MiB/s | ~1 GiB/s |

The single highest-ROI change is **#1** (eliminate `Vec<usize>`) followed by **#2** (mmap).
Together they could push the 1M benchmark from ~330ms toward ~180ms — approaching
the "1 billion rows in 1 second" class of performance per byte.

---

## Key 1BRC Principles That Apply Here

1. **Avoid intermediate collections** — stream, don't buffer
2. **Zero-copy I/O** — mmap beats read() for large files
3. **Integer keys over string keys** — jump tables are faster than memcmp
4. **Work at the byte level** — avoid UTF-8 validation in hot loops
5. **N-thread chunking** — fewer, larger tasks beat many tiny tasks
6. **Allocations compound** — one per message × 1M messages = 1M malloc calls

---

## Implementation Order

1. [ ] **#1** — Rewrite `find_delimiters` as an inline streaming callback in `parse_single_simd`
2. [ ] **#3** — Change `parse_all_simd` / `message_slices` to work on `&[u8]`
3. [ ] **#2** — Add `memmap2`, use mmap in `load_file` (replaces `file.read().await`)
4. [ ] **#4** — `tag_to_u16` + integer match in both `parse_single` and `parse_single_simd`
5. [ ] **#5** — Remove `trim_bytes` from SIMD path
6. [ ] **#6** — Switch to `par_chunks(n_cpus)` in `parse_all_simd`


## Result


100k messages (16 MB file)
Path	Before	After	Gain
scalar_pipe	~27.4 ms	26.3 ms	+4.5%
avx2_pipe	~28.9 ms	25.5 ms	+14.8%
scalar_soh	~46.4 ms	44.9 ms	+3.2%
avx2_soh	~29.9 ms	25.0 ms	+20.7% ← biggest win
avx2_bytes_soh (new)	—	24.9 ms	baseline for #1+#2
Single execution report (1 message)
Path	Before	After	Gain
avx2_pipe	~1050 ns	738 ns	−29%
avx2_soh	~1010 ns	744 ns	−26%
avx2_bytes_soh (new)	—	739 ns	same as avx2_soh
What this means
#1 (no Vec<usize>): The avx2_soh path improved by ~20% on 100k messages and ~26–29% on a single message. That's the direct impact of eliminating 1 heap allocation per message.
avx2_bytes_soh ≈ avx2_soh: The bytes API gives the same parse speed on the bench (because the bench data is already in RAM). The real gain from #2 (mmap) shows up at file load time — for a 166 MB SOH file, you avoid a ~166 MB malloc + memcpy before parsing even starts, which saves ~50–80ms of pure I/O overhead.