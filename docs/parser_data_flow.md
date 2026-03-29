# parser.rs — Data Flow  ⚠️ STALE — see docs/parser.md

> **This file is outdated.** It describes a previous version of the parser. Specifically:
>
> - `parse_all_simd(&str)` **no longer exists** — it was removed when `parse_single` was unified to call `parse_single_simd` directly.
> - `FixField.value` as `CompactString` **no longer exists** — replaced by the arena + offset design (`value_start: u32`, `value_len: u16`).
> - The "parallel chunk alignment" description is superseded by the ownership-region parallel scan (Phase 8 in the blog).
>
> See [`docs/parser.md`](parser.md) for the current design.

---

## parser.rs — Data Flow

## Entry Points

| Function | Input | Use case |
|---|---|---|
| `parse_all(&str)` | String slice | General / normalized path |
| `parse_all_simd(&str)` | String slice | SIMD path, skips normalize |
| `parse_all_simd_bytes(&[u8])` | Byte slice | Hot path — mmap / file load |
| `parse_single_for_validation(&[u8])` | Byte slice | Validator / single-msg debugger |

---

## `parse_all` — Normalized String Path

```
&str input
  │
  ▼
normalize_delimiters()
  SOH \x01 / ^A / \x01 → |
  Zero-alloc if already pipe-delimited (returns Cow::Borrowed)
  │
  ▼
message_slices()
  memmem "8=FIX" SIMD boundary split → Vec<&str>
  │
  └──► rayon::par_iter → parse_single()
                            memchr_iter('|') scan
                            split_once('=') per token
                            string tag match ("35", "49", …)
                            → FixMessage
```

---

## `parse_all_simd` — SIMD String Path

```
&str input
  │
  ▼
message_slices()
  memmem "8=FIX" SIMD boundary split → Vec<&str>
  │
  └──► rayon::par_chunks → parse_single_simd()
                              (see SIMD dispatch below)
                              → FixMessage
```

---

## `parse_all_simd_bytes` — Hot Path (mmap / file load)

```
&[u8] input
  │
  ├─── small (<512 KB) or single thread
  │       │
  │       ▼
  │   message_slices_bytes()
  │   memmem "8=FIX" split → Vec<&[u8]>
  │       │
  │       └──► parse_single_simd() per slice → FixMessage
  │
  └─── large + multi-thread
          │
          ▼
      Parallel boundary detection
      Divide input into num_threads × 16 nominal chunks
      Align each chunk start to next "8=FIX" occurrence
          │
          ▼
      starts[]  (deduped boundary offsets + sentinel)
          │
          └──► rayon::par_windows(2)
                  └──► parse_chunk(&input[start..end])
                          memmem "8=FIX" scan + parse in one pass
                          └──► parse_single_simd() → FixMessage
```

---

## `parse_single_simd` — SIMD Dispatch

```
&[u8] raw message
  │
  ├─ x86_64 + AVX2 detected  →  simd_parse_avx2()
  │     32-byte chunks via _mm256_loadu_si256
  │     Compare each byte against SOH and | simultaneously
  │     u32 bitmask → trailing_zeros loop over delimiter positions
  │
  ├─ aarch64                  →  simd_parse_neon()
  │     16-byte chunks via vld1q_u8
  │     vceqq_u8 for SOH and |, OR masks
  │     Weight-sum trick → u16 bitmask → trailing_zeros loop
  │
  └─ fallback                 →  simd_parse_scalar()
        byte-by-byte scan for 0x01 or |
  │
  └──► (all three converge) apply_token(raw, start, end, msg)
```

---

## `apply_token` — Token → FixMessage (Hot Inner Loop)

```
raw[start..end]  (one "tag=value" field)
  │
  ▼
Locate '=' at index 1, 2, 3, or 4
  │
  ├─ tag_to_u16()
  │     ASCII digit bytes → u16
  │     Slice-pattern match [a] / [a,b] / [a,b,c] / [a,b,c,d]
  │     Compiles to branch tree — no loop, no malloc
  │
  ├─ value = from_utf8_unchecked()   (FIX is pure ASCII — skip validation)
  │
  ├─ push FixField { tag: u16, value: CompactString }
  │
  └─ match hot tags → populate FixMessage summary fields
        35  → msg_type_raw + msg_type_label
        49  → sender
        52  → time  (via extract_time: YYYYMMDD-HH:MM:SS → YYYY-MM-DD HH:MM:SS)
        56  → target
        11  → cl_ord_id
        54  → side  (via side_label lookup)
        38  → order_qty
        55  → symbol
        58  → text
        117 → quote_id
        131 → quote_req_id
        150 → override msg_type_label (FILL / PARTIAL / CANCELED / Reject)
```

---

## Output

```rust
FixMessage {
    fields:        Vec<FixField>,   // all tag=value pairs
    time:          CompactString,   // tag 52 formatted
    sender:        CompactString,   // tag 49
    target:        CompactString,   // tag 56
    msg_type_raw:  CompactString,   // tag 35 raw value
    msg_type_label: &'static str,   // human label
    cl_ord_id:     CompactString,   // tag 11
    quote_id:      CompactString,   // tag 117
    quote_req_id:  CompactString,   // tag 131
    side:          CompactString,   // tag 54 label
    order_qty:     CompactString,   // tag 38
    symbol:        CompactString,   // tag 55
    text:          CompactString,   // tag 58
}
```

---

## Key Design Decisions

| Decision | Location | Reason |
|---|---|---|
| `Cow::Borrowed` in normalize | `normalize_delimiters` | Zero-alloc when input is already pipe-delimited |
| `memmem` for boundary split | `message_slices*` | SIMD substring search, faster than `split()` |
| Parallel chunk alignment | `parse_all_simd_bytes` large path | Hides O(n) boundary scan behind parallel parse |
| Runtime CPU dispatch | `parse_single_simd` | AVX2 → NEON → scalar without compile-time lock-in |
| `tag_to_u16` integer match | `apply_token` | Integer jump table avoids string comparisons |
| `from_utf8_unchecked` | `apply_token` | FIX is 7-bit ASCII — skips 2 validation scans per token |
| `CompactString` | all fields | Inline storage for short strings, avoids heap for typical tag values |



The selected line (into_par_iter) is actually Rayon — CPU thread parallelism, not SIMD. They're two different things. Here's how they differ in this file:

SIMD — Single Instruction, Multiple Data
One CPU instruction processes multiple bytes at once instead of one at a time.

In simd_parse_avx2:


// Load 32 bytes in ONE instruction
let chunk = _mm256_loadu_si256(raw.as_ptr().add(i * 32) as *const __m256i);

// Compare ALL 32 bytes against SOH simultaneously — still ONE instruction
let any = _mm256_or_si256(
    _mm256_cmpeq_epi8(chunk, soh_vec),   // byte[0..32] == 0x01 ?
    _mm256_cmpeq_epi8(chunk, pipe_vec),  // byte[0..32] == '|'  ?
);
Without SIMD the scalar fallback does this:


// Check ONE byte per loop iteration
for (i, &b) in raw.iter().enumerate() {
    if b == 0x01 || b == b'|' { ... }
}
Scalar	AVX2 SIMD	NEON SIMD
Bytes per iteration	1	32	16
Hardware	any CPU	x86_64 only	Apple M1/M2/ARM
Rayon (into_par_iter) — Thread Parallelism
Splits the work across multiple CPU cores. Each core independently calls parse_single on its own message slice.


Thread 1: parse_single(msg[0])  ──┐
Thread 2: parse_single(msg[1])  ──┤──► collect into Vec<FixMessage>
Thread 3: parse_single(msg[2])  ──┘
How they combine in this file

parse_all_simd_bytes (large file)
        │
        ├── Rayon splits file into chunks across cores    ← thread parallelism
        │
        └── each core calls parse_single_simd()
                └── AVX2/NEON scans 32/16 bytes at once  ← SIMD
So for a 200 MB FIX log: Rayon uses all cores in parallel, and within each core AVX2 scans 32 bytes per instruction. Both layers stack on top of each other.

The into_par_iter you selected is on line 87 inside parse_all — that's the slower normalized path which gets Rayon parallelism but no SIMD (it uses parse_single not parse_single_simd).