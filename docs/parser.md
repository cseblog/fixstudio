# Parser — Design & Implementation

Supersedes the stale `docs/parser_data_flow.md`.

---

## Entry Points

| Function | Input | Delimiter handling | SIMD | When used |
|---|---|---|---|---|
| `parse_all(&str)` | String slice | normalize SOH/^A/\x01 → pipe first | No | User-pasted text (UI) |
| `parse_all_simd_bytes(&[u8])` | Byte slice | both SOH and pipe, natively | Yes | **Hot path** — mmap / file load |
| `parse_single_for_validation(&[u8])` | Byte slice | both, natively | Yes | Validator single-msg debugger |

> **Note:** `parse_all_simd(&str)` appeared in old docs but **does not exist**. It was removed
> when `parse_single` (the normalized string path) was unified to call `parse_single_simd`.

---

## parse_all — Normalized String Path

Used for user-pasted text input in the UI. Not the benchmark hot path.

```
&str input (e.g. user paste)
  │
  ▼
normalize_delimiters(&str) → Cow<'_, str>
  If no 0x01 / \\ / ^ bytes:  Cow::Borrowed (zero alloc, zero copy)
  Otherwise:                   Cow::Owned (single-pass rewrite: SOH → |, ^A → |, \x01 → |)
  │
  ▼
str_message_slices(&str) → Vec<&str>
  memmem "8=FIX" scan → start offsets
  slice input into per-message &str slices
  │
  └─► rayon::par_iter().map(parse_single).collect()

parse_single(&str) → FixMessage
  Delegates to: parse_single_simd(raw.as_bytes())
  (see SIMD path below — identical hot loop, same arena trick)
```

**Overhead vs bytes path:** One extra pass (normalize_delimiters) and an extra memmem scan
in `str_message_slices`. Single-message benchmark: 279 ns (str path) vs 254 ns (SIMD bytes).
100k benchmark: 10.4 ms (str) vs 7.9 ms (SIMD bytes).

---

## parse_all_simd_bytes — Hot Path

Used for memory-mapped file data. No normalization, no extra scan.

```
&[u8] input (mmap or file.read())
  │
  ▼
message_start_offsets(&[u8]) → Vec<u32>
  │
  ├─ Small input (<2 MB) or single thread:
  │    Serial memmem::find_iter("8=FIX") → push abs offset as u32
  │
  └─ Large input (≥2 MB), multi-core:
       PARALLEL_SCAN_MIN_BYTES = 2 MB threshold
       chunk_size = ceil(input.len() / thread_count)
       OVERLAP = 4  ("8=FIX" is 5 bytes → marker can start 4 bytes before chunk end)

       (0..thread_count).into_par_iter().map(|i| {
           own_start = i * chunk_size
           own_end   = min((i+1)*chunk_size, input.len())
           scan_end  = min(own_end + OVERLAP, input.len())

           chunk = &input[own_start..scan_end]
           for pos in memmem::find_iter(chunk, "8=FIX") {
               abs = own_start + pos
               if abs < own_end || i+1 == thread_count {  // ownership check
                   v.push(abs as u32)
               }
           }
       }).collect()

       // Chunks in index order → already sorted, just flatten
  │
  ▼
offsets.push(input.len() as u32)    ← sentinel

if offsets.len() == 1 { return empty or single }

offsets.par_windows(2)
  .map(|w| parse_single_simd(&input[w[0] as usize .. w[1] as usize]))
  .collect::<Vec<FixMessage>>()
```

**Why u32 offsets instead of &[u8] slices?**
`Vec<u32>` is 4× smaller than `Vec<&[u8]>` (fat pointer = 16 bytes). For 1M messages that's
4 MB vs 16 MB. Better L1 cache utilization when distributing to Rayon workers.

**Why ownership regions in the parallel scan?**
Without ownership checks, a marker near a chunk boundary would appear in two workers' results,
causing a duplicate message. Each worker scans slightly past its boundary (OVERLAP=4) to catch
straddle cases, but only keeps markers where `abs < own_end`. The last worker always keeps
everything up to `input.len()`.

---

## parse_single_simd — Per-Message Parse

```rust
fn parse_single_simd(raw: &[u8]) -> FixMessage {
    let mut msg = FixMessage {
        fields: Vec::with_capacity(AVG_FIELDS_PER_MSG),  // = 24
        arena:  raw.to_vec(),  // ONE memcpy of full message (~190 bytes avg)
        ..Default::default()
    };
    fill_message(raw, &mut msg);
    msg
}
```

**Arena pre-copy trick:** Copies the entire raw message into `msg.arena` in one `memcpy`-like
call before parsing starts. `apply_token` then computes value offsets as pure arithmetic
(`start + eq_index + 1`) rather than appending per-field bytes. This avoids 20+
`extend_from_slice` calls per message — each of which reads `arena.len()`, writes bytes, and
updates the length.

---

## fill_message — SIMD Dispatch

Runtime dispatch based on CPU architecture and feature detection.

```rust
// x86_64: check for AVX2 at runtime
#[cfg(target_arch = "x86_64")]
fn fill_message(raw: &[u8], msg: &mut FixMessage) {
    if is_x86_feature_detected!("avx2") {
        unsafe { simd_parse_avx2(raw, msg) }
    } else {
        simd_parse_scalar(raw, msg)
    }
}

// aarch64 (Apple M1/M2/M3): NEON always available
#[cfg(target_arch = "aarch64")]
fn fill_message(raw: &[u8], msg: &mut FixMessage) {
    unsafe { simd_parse_neon(raw, msg) }
}

// Other targets: scalar
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn fill_message(raw: &[u8], msg: &mut FixMessage) {
    simd_parse_scalar(raw, msg)
}
```

> Early versions had AVX2 guarded by `#[cfg(target_arch = "x86_64")]` at parse time —
> meaning the app compiled for ARM **silently fell back to scalar** for every message.
> Adding `simd_parse_neon` for aarch64 gave a ~43% speedup on M1 (~140ms → ~80ms for 1M).

---

## simd_parse_avx2 — x86_64 Path

Processes 32 bytes per loop iteration. Finds SOH (0x01) and pipe (|) simultaneously with one OR.

```
for chunk_index in 0..byte_count/32:
  chunk    = _mm256_loadu_si256(raw[chunk_index*32])   // load 32 bytes
  soh_mask = _mm256_cmpeq_epi8(chunk, soh_vec)         // each byte == 0x01?
  pip_mask = _mm256_cmpeq_epi8(chunk, pipe_vec)        // each byte == '|'?
  any      = _mm256_or_si256(soh_mask, pip_mask)        // union
  mask     = _mm256_movemask_epi8(any) as u32          // 32-bit: bit i set = byte i is delimiter

  while mask != 0:
    end = base + mask.trailing_zeros()    // position of first delimiter
    apply_token(raw, start, end, msg)
    start = end + 1
    mask &= mask - 1                       // clear lowest set bit → next delimiter

scalar tail for remaining < 32 bytes
```

`_mm256_movemask_epi8` is a native x86 instruction that collapses a 256-bit comparison result
into a 32-bit integer where each bit represents one byte. This is the key advantage of AVX2 over
NEON.

---

## simd_parse_neon — aarch64 Path

Processes 16 bytes per loop iteration. NEON lacks a native `movemask`, so it is emulated with
a weighted-sum trick.

```
weights = [1, 2, 4, 8, 16, 32, 64, 128]  (powers of two)

for chunk_index in 0..byte_count/16:
  chunk   = vld1q_u8(raw[chunk_index*16])             // load 16 bytes
  any     = vorrq_u8(vceqq_u8(chunk, soh_vec),        // each byte == 0x01?
                     vceqq_u8(chunk, pipe_vec))         // each byte == '|'?

  // Build 16-bit bitmask:
  //   Multiply each lane's comparison result by its weight 2^lane.
  //   Sum all 8 low lanes → lo_bits (u8 → cast to u16)
  //   Sum all 8 high lanes → hi_bits << 8
  //   OR together → mask: bit i set means byte i is a delimiter
  lo_bits = vaddv_u8(vand_u8(vget_low_u8(any), weights_lo))
  hi_bits = vaddv_u8(vand_u8(vget_high_u8(any), weights_hi)) << 8
  mask: u16 = lo_bits | hi_bits

  while mask != 0:
    end = base + mask.trailing_zeros()
    apply_token(raw, start, end, msg)
    start = end + 1
    mask &= mask - 1                    // clear lowest set bit

scalar tail for remaining < 16 bytes
```

**Common weight mistake:** Using sequential weights `[1, 2, 3, 4, 5, 6, 7, 8]` instead of
powers of two gives wrong bit positions — the delimiter appears to be at the wrong index, the
tag parse reads garbage bytes, and tests fail with wrong values but no panic.

---

## simd_parse_scalar — Fallback

Used on x86_64 without AVX2 at runtime, and on all other architectures.

```rust
fn simd_parse_scalar(raw: &[u8], msg: &mut FixMessage) {
    let byte_count = raw.len();
    let mut start = 0;
    for (i, &byte) in raw.iter().enumerate() {
        if is_delimiter(byte) {
            apply_token(raw, start, i, msg);
            start = i + 1;
        }
    }
    if start < byte_count {
        apply_token(raw, start, byte_count, msg);
    }
}

fn is_delimiter(byte: u8) -> bool {
    byte == 0x01 || byte == b'|'
}
```

---

## apply_token — The Inner Loop

Called once per field (20–30× per message). Marked `#[inline(always)]`.

```rust
fn apply_token(raw: &[u8], start: usize, end: usize, msg: &mut FixMessage) {
    if end <= start { return; }
    let token = &raw[start..end];

    // FIX tags are 1–4 digits. '=' at position 1, 2, 3, or 4.
    // Slice-pattern match → branch tree with no loop.
    let eq_index = match (token.get(1), token.get(2), token.get(3), token.get(4)) {
        (Some(&b'='), ..)          => 1,
        (_, Some(&b'='), ..)       => 2,
        (_, _, Some(&b'='), ..)    => 3,
        (_, _, _, Some(&b'='))     => 4,
        _                          => return,  // malformed, skip
    };

    let tag_b = &token[..eq_index];
    let val_b = &token[eq_index + 1..];

    // FIX is 7-bit ASCII — skip UTF-8 validation (two scans per token saved)
    let value = unsafe { std::str::from_utf8_unchecked(val_b) };

    let tag_num = tag_to_u16(tag_b);

    // Arena already contains raw; value offset = absolute position in raw
    let value_start = (start + eq_index + 1) as u32;
    let value_len   = val_b.len() as u16;
    msg.fields.push(FixField { tag: tag_num, value_len, value_start });

    // Hot-field extraction: O(1) access later for timeline/detail panels
    match tag_num {
        52  => msg.time          = extract_time(value),
        49  => msg.sender        = CompactString::from(value),
        56  => msg.target        = CompactString::from(value),
        35  => { msg.msg_type_raw = CompactString::from(value);
                 msg.msg_type_label = msg_type_label(value); }
        11  => msg.cl_ord_id     = CompactString::from(value),
        117 => msg.quote_id      = CompactString::from(value),
        131 => msg.quote_req_id  = CompactString::from(value),
        54  => msg.side          = CompactString::from(side_label(value)),
        38  => msg.order_qty     = CompactString::from(value),
        55  => msg.symbol        = CompactString::from(value),
        58  => msg.text          = CompactString::from(value),
        150 => { /* adjust msg_type_label for ExecType fills/cancels */ }
        _   => {}
    }
}
```

---

## tag_to_u16 — Branch Tree

FIX tags are 1–4 ASCII digit strings. Converting them to `u16` via a slice-pattern match
compiles to a branch tree — no loop, no string comparison, no allocation.

```rust
#[inline(always)]
fn tag_to_u16(digits: &[u8]) -> u16 {
    #[inline(always)]
    fn d(byte: u8) -> u16 { (byte - b'0') as u16 }
    match digits {
        [a]             => d(*a),
        [a, b]          => d(*a) * 10   + d(*b),
        [a, b, c]       => d(*a) * 100  + d(*b) * 10  + d(*c),
        [a, b, c, e]    => d(*a) * 1000 + d(*b) * 100 + d(*c) * 10 + d(*e),
        _               => 0,
    }
}
```

Example: `b"35"` → length 2 → `3*10 + 5 = 35_u16`. The resulting `u16` feeds the `match` in
`apply_token` which the compiler can turn into a jump table for the common-tag arms.

---

## extract_time

Converts FIX SendingTime (tag 52) from compact form to display form:

```
"20240115-09:30:01.123"  →  "2024-01-15 09:30:01.123"
"20240115-09:30:01"      →  "2024-01-15 09:30:01"
```

Returns a `CompactString` (inline for timestamps ≤ 23 bytes, which covers most FIX timestamps).

---

## Performance Numbers (M1 Max, opt-level 3, thin LTO)

| Benchmark | Time | Throughput |
|---|---|---|
| Single ExecutionReport (SIMD bytes) | 254 ns | — |
| Single ExecutionReport (str path) | 279 ns | — |
| 100k messages (SIMD bytes, SOH) | 7.9 ms | 2.4 GiB/s |
| 100k messages (SIMD bytes, pipe) | 7.9 ms | 2.4 GiB/s |
| 100k messages (str path, pipe) | 10.4 ms | 1.86 GiB/s |
| **1M messages (SIMD bytes)** | **87 ms** | **2.2 GiB/s** |
| Delimiter scanner (100k, pipe) | 11.4 ms | 1.7 GiB/s |

The delimiter scanner (`simd.rs::find_delimiters`) is slower than the full parse on a per-byte
basis because it allocates a `Vec<usize>` of ALL delimiter positions (not just "8=FIX" message
boundaries). At 1 delimiter per ~10 bytes, it writes 2M entries for a 20MB file — Vec writes
dominate.

---

## Tuning Constants

```rust
const AVG_MSG_BYTES:       usize = 140;   // Pre-size boundary Vec
const AVG_MSG_BYTES_STR:   usize = 160;   // Pre-size str-path Vec (pipe is slightly larger)
const AVG_FIELDS_PER_MSG:  usize = 24;    // Pre-size fields Vec per message
const PARALLEL_SCAN_MIN_BYTES: usize = 2 * 1024 * 1024; // 2 MB
const OVERLAP:             usize = 4;     // "8=FIX" is 5 bytes → max straddle is 4 bytes
```

All are conservative (undershooting → extra realloc, overshooting → wasted capacity).
`AVG_FIELDS_PER_MSG = 24` works well for typical FIX 4.4 mixes; ExecutionReports have
~20 fields, NewOrderSingles ~15, session messages ~8.

---

## simd.rs — Standalone Delimiter Scanner

`src/simd.rs` exposes `find_delimiters(&[u8]) -> Vec<usize>` as a public API (benchmarkable
via `bench_scanner`). It is **not used by the parser** — it was extracted as a utility/benchmark
target to study standalone delimiter scan throughput.

The parser's hot path uses `simd_parse_avx2`/`simd_parse_neon` directly (which interleave
delimiter finding with token extraction in a single pass) rather than a separate scan step.
