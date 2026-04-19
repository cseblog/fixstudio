use std::borrow::Cow;

use compact_str::{CompactString, format_compact};
use memchr::{memchr3, memmem};
use rayon::prelude::*;

use crate::dictionary::{msg_type_label, side_label};
use crate::model::{FixField, FixMessage};

// ── Tuning constants ──────────────────────────────────────────────────────────

/// Conservative estimate of average FIX message size in bytes (SOH-delimited).
/// Used only to pre-size the output Vec — undershooting is fine; it just reallocates.
const AVG_MSG_BYTES: usize = 140;

/// Conservative estimate for the pipe-delimited (str) path, which has slightly
/// longer fields due to no SOH byte saved.
const AVG_MSG_BYTES_STR: usize = 160;

/// Typical number of fields per FIX message; used to pre-size `fields` Vec.
const AVG_FIELDS_PER_MSG: usize = 24;

/// Normalize delimiters: SOH (0x01), \x01, ^A → pipe.
/// Returns a borrowed slice when no special delimiters are present (zero allocation).
fn normalize_delimiters(input: &str) -> Cow<'_, str> {
    if memchr3(0x01, b'\\', b'^', input.as_bytes()).is_none() {
        return Cow::Borrowed(input);
    }
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            0x01 => { out.push('|'); i += 1; }
            b'\\' if bytes.get(i + 1..i + 4) == Some(b"x01") => {
                out.push('|'); i += 4;
            }
            b'^' if bytes.get(i + 1) == Some(&b'A') => {
                out.push('|'); i += 2;
            }
            b => { out.push(b as char); i += 1; }
        }
    }
    Cow::Owned(out)
}

/// Threshold above which the boundary scan is parallelised across rayon workers.
/// Below this, the serial memmem scan is faster than spawning rayon tasks.
const PARALLEL_SCAN_MIN_BYTES: usize = 2 * 1024 * 1024; // 2 MB

/// Split raw bytes on "8=FIX" boundaries.
///
/// Returns a `Vec<u32>` of start offsets rather than fat `&[u8]` pointers (16 bytes each).
/// At 4 bytes per entry, the Vec is 4× smaller — better L1 cache utilization when
/// distributing slices across rayon workers.
///
/// For inputs ≥ `PARALLEL_SCAN_MIN_BYTES`, the memmem scan itself runs in parallel
/// across rayon workers — each worker scans its own chunk plus a 4-byte overlap,
/// then discards any marker that belongs to the next worker's range. The chunks are
/// processed in index order so the resulting Vec is already sorted; no sort needed.
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

    // Parallel path.
    // "8=FIX" is 5 bytes → a marker can start at most 4 bytes before a chunk boundary.
    // Each worker scans [own_start .. own_end + 4] but only keeps markers in
    // [own_start .. own_end), so markers are never double-counted.
    const OVERLAP: usize = 4;
    let chunk_size = (input.len() + thread_count - 1) / thread_count;
    let msgs_per_chunk = (chunk_size / AVG_MSG_BYTES).max(4);

    let per_chunk: Vec<Vec<u32>> = (0..thread_count).into_par_iter().map(|i| {
        let own_start = i * chunk_size;
        let own_end   = ((i + 1) * chunk_size).min(input.len());
        let scan_end  = (own_end + OVERLAP).min(input.len());

        let chunk = &input[own_start..scan_end];
        let mut v   = Vec::with_capacity(msgs_per_chunk);
        for pos in memmem::find_iter(chunk, b"8=FIX") {
            let abs = own_start + pos;
            // Last worker claims everything up to input.len().
            if abs < own_end || i + 1 == thread_count {
                v.push(abs as u32);
            }
        }
        v
    }).collect();

    // Chunks are in index order → result is already sorted; just flatten.
    let total = per_chunk.iter().map(|v| v.len()).sum();
    let mut offsets = Vec::with_capacity(total);
    for chunk in per_chunk {
        offsets.extend_from_slice(&chunk);
    }
    offsets
}

/// Parse a raw input string that may contain multiple FIX messages.
/// Normalizes SOH/^A/\x01 delimiters to pipe before parsing.
pub fn parse_all(input: &str) -> Vec<FixMessage> {
    let normalized = normalize_delimiters(input);
    let slices = str_message_slices(&normalized);
    if slices.is_empty() {
        if normalized.trim().is_empty() { vec![] } else { vec![parse_single(&normalized)] }
    } else {
        slices.into_par_iter().map(parse_single).collect()
    }
}

/// Split a pipe-delimited `&str` on "8=FIX" boundaries.
fn str_message_slices(input: &str) -> Vec<&str> {
    let bytes = input.as_bytes();
    let capacity = (bytes.len() / AVG_MSG_BYTES_STR).max(4);
    let mut msgs = Vec::with_capacity(capacity);
    let mut start = 0;
    let mut found = false;

    for pos in memmem::find_iter(bytes, b"8=FIX") {
        if found {
            let slice = input[start..pos].trim();
            if !slice.is_empty() { msgs.push(slice); }
        }
        start = pos;
        found = true;
    }
    if found {
        let slice = input[start..].trim();
        if !slice.is_empty() { msgs.push(slice); }
    }
    msgs
}

// ── SIMD path ─────────────────────────────────────────────────────────────────

/// Preferred hot path for memory-mapped file loading (zero copy from the OS page cache).
/// Handles both SOH and pipe delimiters without normalizing or copying the input.
///
/// Strategy: one serial `memmem` boundary scan → parallel parse.
/// memmem over the full input is fast (~2 GiB/s) and not the bottleneck; the per-message
/// SIMD parse is. A single serial scan followed by even distribution across rayon workers
/// (via `par_chunks`) is faster than parallel boundary detection, which causes each worker
/// to scan its chunk twice — once to align and once inside `parse_chunk`.
pub fn parse_all_simd_bytes(input: &[u8]) -> Vec<FixMessage> {
    let mut offsets = message_start_offsets(input);

    if offsets.is_empty() {
        return if input.iter().all(|b| b.is_ascii_whitespace()) { vec![] }
               else { vec![parse_single_simd(input)] };
    }

    // Append sentinel so every window [offsets[i], offsets[i+1]] is a complete slice.
    offsets.push(input.len() as u32);
    debug_assert!(*offsets.last().unwrap() == input.len() as u32);

    if offsets.len() == 2 {
        return vec![parse_single_simd(&input[offsets[0] as usize..])];
    }

    // par_windows(2) over u32 offsets: each window defines one message slice.
    // 4-byte offsets vs 16-byte fat pointers = 4× smaller Vec, better L1 utilization.
    offsets
        .par_windows(2)
        .map(|w| parse_single_simd(&input[w[0] as usize..w[1] as usize]))
        .collect()
}


/// Parse a single FIX message from raw bytes.
/// Dispatches to: AVX2 (x86_64) → NEON (aarch64) → scalar fallback.
fn parse_single_simd(raw: &[u8]) -> FixMessage {
    let mut msg = FixMessage {
        fields: Vec::with_capacity(AVG_FIELDS_PER_MSG),
        // Copy raw bytes as the arena in one memcpy. apply_token then computes
        // value offsets directly from (start + eq + 1), with no per-field
        // extend_from_slice calls in the hot loop.
        arena:  raw.to_vec(),
        ..Default::default()
    };
    fill_message(raw, &mut msg);
    msg
}

/// Arch-specific dispatch: fills `msg` from `raw` using the best available path.
/// Keeping dispatch in one function (with no branching in the arch impls) follows
/// "push ifs up" — each arch impl is a pure, branchless worker.
#[cfg(target_arch = "x86_64")]
fn fill_message(raw: &[u8], msg: &mut FixMessage) {
    if is_x86_feature_detected!("avx2") {
        unsafe { simd_parse_avx2(raw, msg) };
    } else {
        simd_parse_scalar(raw, msg);
    }
}

#[cfg(target_arch = "aarch64")]
fn fill_message(raw: &[u8], msg: &mut FixMessage) {
    unsafe { simd_parse_neon(raw, msg) };
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn fill_message(raw: &[u8], msg: &mut FixMessage) {
    simd_parse_scalar(raw, msg);
}

// ── x86_64 AVX2 path ─────────────────────────────────────────────────────────

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn simd_parse_avx2(raw: &[u8], msg: &mut FixMessage) {
    use std::arch::x86_64::*;

    let soh_vec  = _mm256_set1_epi8(0x01_i8);
    let pipe_vec = _mm256_set1_epi8(b'|' as i8);
    let byte_count  = raw.len();
    let chunk_count = byte_count / 32;
    let mut start = 0;

    for chunk_index in 0..chunk_count {
        let chunk = _mm256_loadu_si256(raw.as_ptr().add(chunk_index * 32) as *const __m256i);
        let any = _mm256_or_si256(
            _mm256_cmpeq_epi8(chunk, soh_vec),
            _mm256_cmpeq_epi8(chunk, pipe_vec),
        );
        let mut mask = _mm256_movemask_epi8(any) as u32;
        let base = chunk_index * 32;
        while mask != 0 {
            let end = base + mask.trailing_zeros() as usize;
            apply_token(raw, start, end, msg);
            start = end + 1;
            mask &= mask - 1;
        }
    }
    // Scalar tail: handle remaining bytes that didn't fill a full 32-byte chunk.
    for i in (chunk_count * 32)..byte_count {
        if is_delimiter(raw[i]) {
            apply_token(raw, start, i, msg);
            start = i + 1;
        }
    }
    if start < byte_count {
        apply_token(raw, start, byte_count, msg);
    }
}

// ── aarch64 NEON path ─────────────────────────────────────────────────────────

/// NEON 128-bit vectorized delimiter scan (16 bytes / iteration).
///
/// Emulates x86 `movemask_epi8` via a weight-sum trick:
///   1. Compare each byte against SOH and `|`; OR the masks.
///   2. AND with per-lane powers-of-two weights → `vaddv` collapses to a u8 bitmask
///      for the low 8 lanes and separately for the high 8 lanes.
///   3. Combine into a `u16` where bit i means byte i is a delimiter.
///   4. Iterate over set bits via `trailing_zeros` + bit-clear loop (same as AVX2).
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn simd_parse_neon(raw: &[u8], msg: &mut FixMessage) {
    use std::arch::aarch64::*;

    let soh_vec  = vdupq_n_u8(0x01);
    let pipe_vec = vdupq_n_u8(b'|');
    // Per-lane powers-of-two used to collapse an 8-lane mask to a single byte.
    // Lane k gets weight 2^k: in little-endian u64 byte 0 = 0x01, byte 7 = 0x80.
    // The same weight pattern applies to both the low (lanes 0-7) and high (lanes 8-15)
    // halves because vaddv_u8 operates on 8 lanes independently for each half.
    const LANE_WEIGHTS: u64 = 0x8040201008040201;
    let weights_lo = vcreate_u8(LANE_WEIGHTS);
    let weights_hi = vcreate_u8(LANE_WEIGHTS);

    let byte_count  = raw.len();
    let chunk_count = byte_count / 16;
    let mut start = 0;

    for chunk_index in 0..chunk_count {
        let chunk = vld1q_u8(raw.as_ptr().add(chunk_index * 16));
        let any = vorrq_u8(
            vceqq_u8(chunk, soh_vec),
            vceqq_u8(chunk, pipe_vec),
        );

        // Build a 16-bit bitmask (bit i = 1 means byte i is a delimiter).
        // No quick-rejection: FIX messages average 1 delimiter per ~10 bytes so nearly
        // every 16-byte chunk has at least one — the vmaxvq check would almost always
        // fall through and costs an extra instruction + branch per chunk.
        let lo_half = vget_low_u8(any);
        let hi_half = vget_high_u8(any);
        let lo_bits = vaddv_u8(vand_u8(lo_half, weights_lo)) as u16;
        let hi_bits = (vaddv_u8(vand_u8(hi_half, weights_hi)) as u16) << 8;
        let mut mask: u16 = lo_bits | hi_bits;

        let base = chunk_index * 16;
        while mask != 0 {
            let end = base + mask.trailing_zeros() as usize;
            apply_token(raw, start, end, msg);
            start = end + 1;
            mask &= mask - 1;
        }
    }
    // Scalar tail: handle remaining bytes that didn't fill a full 16-byte chunk.
    for i in (chunk_count * 16)..byte_count {
        if is_delimiter(raw[i]) {
            apply_token(raw, start, i, msg);
            start = i + 1;
        }
    }
    if start < byte_count {
        apply_token(raw, start, byte_count, msg);
    }
}

/// Returns true if `byte` is a FIX field delimiter (SOH or pipe).
#[inline(always)]
fn is_delimiter(byte: u8) -> bool {
    byte == 0x01 || byte == b'|'
}

/// Scalar fallback: used on unknown targets and on x86_64 without AVX2 at runtime.
#[cfg(not(target_arch = "aarch64"))]
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

/// Extract one `tag=value` token and update `msg`.
///
/// Hot-path design decisions:
/// - `tag_to_u16` converts the ASCII tag to u16 → integer jump table (no string comparison).
/// - No `trim_bytes` inside tokens — FIX log files never have whitespace within a field.
/// - Unchecked UTF-8: FIX is pure ASCII; skipping validation removes two scans per token.
#[inline(always)]
fn apply_token(raw: &[u8], start: usize, end: usize, msg: &mut FixMessage) {
    debug_assert!(start <= raw.len());
    debug_assert!(end   <= raw.len());
    if end <= start { return; }
    let token = &raw[start..end];

    // FIX tags are 1–4 ASCII digits; '=' appears at index 1, 2, 3, or 4.
    let eq_index = match (token.get(1), token.get(2), token.get(3), token.get(4)) {
        (Some(&b'='), ..)          => 1,
        (_, Some(&b'='), ..)       => 2,
        (_, _, Some(&b'='), ..)    => 3,
        (_, _, _, Some(&b'='))     => 4,
        _                          => return,
    };

    debug_assert!(eq_index < token.len());
    let tag_b = &token[..eq_index];
    let val_b = &token[eq_index + 1..];

    // SAFETY: FIX protocol is 7-bit ASCII throughout.
    let value = unsafe { std::str::from_utf8_unchecked(val_b) };

    let tag_num = tag_to_u16(tag_b);

    // The arena already holds a verbatim copy of `raw` (set in parse_single_simd).
    // Value offset = absolute position within raw = token_start + eq_index + 1.
    // No per-field extend_from_slice — all value bytes are already present.
    let value_start = (start + eq_index + 1) as u32;
    let value_len   = val_b.len() as u16;

    msg.fields.push(FixField { tag: tag_num, value_len, value_start });

    match tag_num {
        52  => msg.time         = extract_time(value),
        49  => msg.sender       = CompactString::from(value),
        56  => msg.target       = CompactString::from(value),
        35  => {
            msg.msg_type_raw   = CompactString::from(value);
            msg.msg_type_label = msg_type_label(value);
        }
        11  => msg.cl_ord_id   = CompactString::from(value),
        117 => msg.quote_id    = CompactString::from(value),
        131 => msg.quote_req_id = CompactString::from(value),
        54  => msg.side        = CompactString::from(side_label(value)),
        38  => msg.order_qty   = CompactString::from(value),
        55  => msg.symbol      = CompactString::from(value),
        58  => msg.text        = CompactString::from(value),
        150 => {
            msg.msg_type_label = match value {
                "F" | "2" => "ER FILL",
                "1"       => "ER PARTIAL",
                "4" | "C" => "ER CANCELED",
                "8"       => "Reject",
                _         => msg.msg_type_label,
            };
        }
        _ => {}
    }
}

/// Convert a 1–4 byte ASCII-digit FIX tag to `u16`.
/// Slice-pattern match compiles to a branch tree with no loop or string allocation.
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

// ── Validation entry point ────────────────────────────────────────────────────

/// Parse a single FIX message from raw bytes (pipe or SOH delimited).
/// Used by the validator for single-message debugger mode.
pub fn parse_single_for_validation(raw: &[u8]) -> FixMessage {
    parse_single_simd(raw)
}

/// Parse a single pipe-delimited FIX message string into a [`FixMessage`].
/// Used only by `parse_all` (the normalized string path).
///
/// Delegates to `parse_single_simd` so that both the str path and the
/// bytes path share the same NEON/AVX2 hot loop and the pre-copy arena trick.
/// `parse_single_simd` already handles both SOH and pipe delimiters, and
/// the input here is already pipe-normalised, so the semantics are identical.
#[inline]
fn parse_single(raw: &str) -> FixMessage {
    parse_single_simd(raw.as_bytes())
}

/// Format SendingTime (tag 52): YYYYMMDD-HH:MM:SS → YYYY-MM-DD HH:MM:SS
fn extract_time(s: &str) -> CompactString {
    if let Some(dash) = s.find('-') {
        let date = &s[..dash];
        let time = s[dash + 1..].trim_end();
        if date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()) {
            return format_compact!("{}-{}-{} {}", &date[0..4], &date[4..6], &date[6..8], time);
        }
        return format_compact!("{date} {time}");
    }
    CompactString::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sample() {
        let input = "8=FIX.4.4|9=61|35=A|34=1|49=EXEC|52=20121105-23:24:06|56=BANZAI|98=0|108=30|10=003|";
        let msgs = parse_all(input);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].msg_type_raw, "A");
        assert_eq!(msgs[0].sender, "EXEC");
        assert_eq!(msgs[0].time, "2012-11-05 23:24:06");
    }

    #[test]
    fn test_normalize_soh() {
        let input = "8=FIX.4.1\x019=61\x0135=A|34=1";
        let msgs = parse_all(input);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].fields.len(), 4);
    }

    #[test]
    fn test_normalize_borrowed() {
        let input = "8=FIX.4.4|9=5|35=0|10=001|";
        let result = normalize_delimiters(input);
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn test_parse_all_simd_bytes_soh() {
        let input = b"8=FIX.4.4\x019=61\x0135=A\x0134=1\x0149=EXEC\x0152=20121105-23:24:06\x0156=BANZAI\x0198=0\x01108=30\x0110=003\x01";
        let msgs = parse_all_simd_bytes(input);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].msg_type_raw, "A");
        assert_eq!(msgs[0].sender, "EXEC");
        assert_eq!(msgs[0].time, "2012-11-05 23:24:06");
    }

    #[test]
    fn test_parse_all_simd_bytes_multi() {
        let m1 = b"8=FIX.4.4\x019=30\x0135=A\x0134=1\x0149=EXEC\x0156=CLIENT\x0110=001\x01";
        let m2 = b"8=FIX.4.4\x019=30\x0135=0\x0134=2\x0149=EXEC\x0156=CLIENT\x0110=002\x01";
        let mut input = Vec::new();
        input.extend_from_slice(m1);
        input.extend_from_slice(m2);
        let msgs = parse_all_simd_bytes(&input);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].msg_type_raw, "A");
        assert_eq!(msgs[1].msg_type_raw, "0");
    }

    #[test]
    fn test_simd_bytes_pipe_delimited() {
        let input = b"8=FIX.4.4|9=61|35=8|49=EXEC|56=CLIENT|55=AAPL|54=1|38=100|10=001|";
        let msgs = parse_all_simd_bytes(input);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].msg_type_raw, "8");
        assert_eq!(msgs[0].sender, "EXEC");
        assert_eq!(msgs[0].symbol, "AAPL");
    }

    // ── Negative / boundary tests ─────────────────────────────────────────────

    #[test]
    fn test_parse_all_empty_string() {
        assert!(parse_all("").is_empty());
    }

    #[test]
    fn test_parse_all_simd_bytes_empty() {
        assert!(parse_all_simd_bytes(b"").is_empty());
    }

    #[test]
    fn test_parse_all_whitespace_only() {
        // A string of only whitespace should yield no messages.
        assert!(parse_all("   \n  ").is_empty());
    }

    #[test]
    fn test_token_without_equals_is_skipped() {
        // "GARBAGE" contains no '=' — apply_token must return early without panic.
        let input = b"8=FIX.4.4|GARBAGE|35=A|10=001|";
        let msgs = parse_all_simd_bytes(input);
        assert_eq!(msgs.len(), 1);
        // The GARBAGE token is silently skipped; the valid fields are parsed.
        assert_eq!(msgs[0].msg_type_raw, "A");
    }

    #[test]
    fn test_truncated_message_no_panic() {
        // A message that ends mid-field must not panic.
        let input = b"8=FIX.4.4|9=61|35=";
        let msgs = parse_all_simd_bytes(input);
        assert_eq!(msgs.len(), 1);
        // The truncated tag-35 token has an empty value — msg_type_raw is empty.
        assert_eq!(msgs[0].msg_type_raw, "");
    }

    #[test]
    fn test_delimiter_only_input() {
        // Input with nothing but delimiters produces a message with no fields.
        let input = b"||||";
        let msgs = parse_all_simd_bytes(input);
        // No "8=FIX" boundary → treated as a single degenerate message.
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].fields.is_empty());
    }
}
