use std::borrow::Cow;

use compact_str::{CompactString, format_compact};
use memchr::{memchr_iter, memmem};
use memchr::memchr3;
use rayon::prelude::*;

use crate::dictionary::{msg_type_label, side_label};
use crate::model::{FixField, FixMessage};

/// Normalize delimiters: SOH (0x01), \x01, ^A -> pipe.
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

/// Split `input` on "8=FIX" boundaries using SIMD substring search.
fn message_slices(input: &str) -> Vec<&str> {
    let bytes = input.as_bytes();
    let capacity = (bytes.len() / 160).max(4);
    let mut msgs = Vec::with_capacity(capacity);
    let mut start = 0;
    let mut found = false;

    for pos in memmem::find_iter(bytes, b"8=FIX") {
        if found {
            let s = input[start..pos].trim();
            if !s.is_empty() { msgs.push(s); }
        }
        start = pos;
        found = true;
    }
    if found {
        let s = input[start..].trim();
        if !s.is_empty() { msgs.push(s); }
    }
    msgs
}

/// Split raw bytes on "8=FIX" boundaries — works directly on `&[u8]` (mmap-friendly).
fn message_slices_bytes(input: &[u8]) -> Vec<&[u8]> {
    let capacity = (input.len() / 140).max(4);
    let mut msgs = Vec::with_capacity(capacity);
    let mut start = 0usize;
    let mut found = false;

    for pos in memmem::find_iter(input, b"8=FIX") {
        if found {
            let s = trim_bytes(&input[start..pos]);
            if !s.is_empty() { msgs.push(s); }
        }
        start = pos;
        found = true;
    }
    if found {
        let s = trim_bytes(&input[start..]);
        if !s.is_empty() { msgs.push(s); }
    }
    msgs
}

/// Parse a raw input string that may contain multiple FIX messages.
pub fn parse_all(input: &str) -> Vec<FixMessage> {
    let normalized = normalize_delimiters(input);
    let slices = message_slices(&normalized);
    if slices.is_empty() {
        if normalized.trim().is_empty() { vec![] } else { vec![parse_single(&normalized)] }
    } else {
        slices.into_par_iter().map(parse_single).collect()
    }
}

// ── AVX2 / NEON / SIMD path ───────────────────────────────────────────────────

/// Like [`parse_all`] but skips the normalise step — handles both SOH and pipe
/// delimiters without allocating a normalised copy of the input.
pub fn parse_all_simd(input: &str) -> Vec<FixMessage> {
    let slices = message_slices(input);
    if slices.is_empty() {
        if input.trim().is_empty() {
            vec![]
        } else {
            vec![parse_single_simd(input.as_bytes())]
        }
    } else if slices.len() == 1 {
        vec![parse_single_simd(slices[0].as_bytes())]
    } else {
        let n = rayon::current_num_threads().max(1);
        let chunk_size = (slices.len() + n - 1) / n;
        slices
            .par_chunks(chunk_size)
            .flat_map_iter(|chunk| chunk.iter().map(|s| parse_single_simd(s.as_bytes())))
            .collect()
    }
}

/// Like [`parse_all_simd`] but accepts raw bytes — the preferred hot path for
/// memory-mapped file loading (zero copy from the OS page cache).
///
/// **Parallel boundary detection**: instead of one serial memmem scan over the
/// whole file followed by parallel parse, we split on "8=FIX" alignment points
/// and have each thread independently scan *and* parse its slice.  This hides
/// the O(input.len()) boundary-scan cost behind the parallel parse.
pub fn parse_all_simd_bytes(input: &[u8]) -> Vec<FixMessage> {
    let n = rayon::current_num_threads().max(1);

    // Small inputs: single-threaded is cheaper (no boundary-finding overhead).
    if n == 1 || input.len() < 512 * 1024 {
        let slices = message_slices_bytes(input);
        return match slices.len() {
            0 => if input.iter().all(|b| b.is_ascii_whitespace()) { vec![] }
                 else { vec![parse_single_simd(input)] },
            1 => vec![parse_single_simd(slices[0])],
            _ => slices.iter().map(|&s| parse_single_simd(s)).collect(),
        };
    }

    // Use 16× the thread count for fine-grained work-stealing and load-balance.
    let num_chunks  = n * 16;
    let chunk_size  = (input.len() + num_chunks - 1) / num_chunks;
    let mut starts: Vec<usize> = std::iter::once(0usize)
        .chain((1..num_chunks).filter_map(|i| {
            let nominal = i * chunk_size;
            if nominal >= input.len() { return None; }
            memmem::find(&input[nominal..], b"8=FIX").map(|p| nominal + p)
        }))
        .collect();
    starts.dedup();
    starts.push(input.len()); // sentinel

    // Each thread scans+parses its chunk in one pass — no intermediate Vec<&[u8]>.
    // flat_map_iter keeps output in-order (par_windows is indexed parallel iter).
    starts.par_windows(2)
        .flat_map_iter(|w| parse_chunk(&input[w[0]..w[1]]))
        .collect()
}

/// Scan a byte slice for "8=FIX" boundaries and parse each message inline,
/// yielding `FixMessage` items without building an intermediate `Vec<&[u8]>`.
fn parse_chunk(chunk: &[u8]) -> Vec<FixMessage> {
    let capacity = (chunk.len() / 140).max(4);
    let mut out = Vec::with_capacity(capacity);
    let mut start = 0usize;
    let mut found = false;

    for pos in memmem::find_iter(chunk, b"8=FIX") {
        if found {
            let s = trim_bytes(&chunk[start..pos]);
            if !s.is_empty() { out.push(parse_single_simd(s)); }
        }
        start = pos;
        found = true;
    }
    if found {
        let s = trim_bytes(&chunk[start..]);
        if !s.is_empty() { out.push(parse_single_simd(s)); }
    }
    out
}

/// Parse a single FIX message from raw bytes.
/// Dispatches to: AVX2 (x86_64) → NEON (aarch64) → scalar fallback.
fn parse_single_simd(raw: &[u8]) -> FixMessage {
    let mut msg = FixMessage {
        fields: Vec::with_capacity(24),
        ..Default::default()
    };
    #[cfg(target_arch = "x86_64")]
    if is_x86_feature_detected!("avx2") {
        unsafe { simd_parse_avx2(raw, &mut msg) };
        return msg;
    }
    #[cfg(target_arch = "aarch64")]
    {
        unsafe { simd_parse_neon(raw, &mut msg) };
        return msg;
    }
    #[allow(unreachable_code)]
    simd_parse_scalar(raw, &mut msg);
    msg
}

// ── x86_64 AVX2 path ─────────────────────────────────────────────────────────

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn simd_parse_avx2(raw: &[u8], msg: &mut FixMessage) {
    use std::arch::x86_64::*;

    let soh_vec  = _mm256_set1_epi8(0x01_i8);
    let pipe_vec = _mm256_set1_epi8(b'|' as i8);
    let n      = raw.len();
    let chunks = n / 32;
    let mut start = 0usize;

    for i in 0..chunks {
        let chunk = _mm256_loadu_si256(raw.as_ptr().add(i * 32) as *const __m256i);
        let any = _mm256_or_si256(
            _mm256_cmpeq_epi8(chunk, soh_vec),
            _mm256_cmpeq_epi8(chunk, pipe_vec),
        );
        let mut mask = _mm256_movemask_epi8(any) as u32;
        let base = i * 32;
        while mask != 0 {
            let end = base + mask.trailing_zeros() as usize;
            apply_token(raw, start, end, msg);
            start = end + 1;
            mask &= mask - 1;
        }
    }
    for i in (chunks * 32)..n {
        if raw[i] == 0x01 || raw[i] == b'|' {
            apply_token(raw, start, i, msg);
            start = i + 1;
        }
    }
    if start < n {
        apply_token(raw, start, n, msg);
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
    // Per-lane powers-of-two: lane k gets weight 2^k.
    // In little-endian u64: byte 0 = lane 0 = 0x01, …, byte 7 = lane 7 = 0x80.
    let weights_lo = vcreate_u8(0x8040201008040201_u64); // [1,2,4,8,16,32,64,128]
    let weights_hi = vcreate_u8(0x8040201008040201_u64); // same for lanes 8-15

    let n      = raw.len();
    let chunks = n / 16;
    let mut start = 0usize;

    for i in 0..chunks {
        let chunk = vld1q_u8(raw.as_ptr().add(i * 16));
        let any = vorrq_u8(
            vceqq_u8(chunk, soh_vec),
            vceqq_u8(chunk, pipe_vec),
        );

        // Quick rejection: if no byte matches, skip.
        if vmaxvq_u8(any) == 0 { continue; }

        // Build a 16-bit bitmask (bit i = delimiter at byte i).
        let lo_nibble = vget_low_u8(any);
        let hi_nibble = vget_high_u8(any);
        let lo_bits = vaddv_u8(vand_u8(lo_nibble, weights_lo)) as u16;
        let hi_bits = (vaddv_u8(vand_u8(hi_nibble, weights_hi)) as u16) << 8;
        let mut mask: u16 = lo_bits | hi_bits;

        let base = i * 16;
        while mask != 0 {
            let bit = mask.trailing_zeros() as usize;
            let end = base + bit;
            apply_token(raw, start, end, msg);
            start = end + 1;
            mask &= mask - 1;
        }
    }
    // Scalar tail (< 16 bytes remaining).
    for i in (chunks * 16)..n {
        if raw[i] == 0x01 || raw[i] == b'|' {
            apply_token(raw, start, i, msg);
            start = i + 1;
        }
    }
    if start < n {
        apply_token(raw, start, n, msg);
    }
}

/// Scalar fallback for non-x86_64 / non-aarch64 targets.
fn simd_parse_scalar(raw: &[u8], msg: &mut FixMessage) {
    let n = raw.len();
    let mut start = 0;
    for (i, &b) in raw.iter().enumerate() {
        if b == 0x01 || b == b'|' {
            apply_token(raw, start, i, msg);
            start = i + 1;
        }
    }
    if start < n {
        apply_token(raw, start, n, msg);
    }
}

/// Extract one `tag=value` token and update `msg`.
///
/// Hot-path design decisions:
/// - **#4**: `tag_to_u16` converts the ASCII tag to u16 → integer jump table (no memcmp).
/// - **#5**: no `trim_bytes` — FIX log files never have whitespace inside tokens.
/// - **unchecked UTF-8**: FIX is pure ASCII; skipping validation removes 2 scans per token.
#[inline(always)]
fn apply_token(raw: &[u8], start: usize, end: usize, msg: &mut FixMessage) {
    if end <= start { return; }
    let token = &raw[start..end];

    // FIX tags are 1–4 ASCII digits; = appears at index 1, 2, 3, or 4.
    let eq = match (token.get(1), token.get(2), token.get(3), token.get(4)) {
        (Some(&b'='), ..)          => 1,
        (_, Some(&b'='), ..)       => 2,
        (_, _, Some(&b'='), ..)    => 3,
        (_, _, _, Some(&b'='))     => 4,
        _                          => return,
    };

    let tag_b = &token[..eq];
    let val_b = &token[eq + 1..];

    // SAFETY: FIX protocol is 7-bit ASCII throughout.
    let value   = unsafe { std::str::from_utf8_unchecked(val_b) };

    // #4: integer tag → jump table; also stored directly in FixField (no string alloc).
    let tag_num = tag_to_u16(tag_b);

    msg.fields.push(FixField {
        tag:   tag_num,
        value: CompactString::from(value),
    });

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
/// Slice-pattern match compiles to a branch tree with no loop.
#[inline(always)]
fn tag_to_u16(b: &[u8]) -> u16 {
    match b {
        [a]          => (a - b'0') as u16,
        [a, b]       => (a - b'0') as u16 * 10   + (b - b'0') as u16,
        [a, b, c]    => (a - b'0') as u16 * 100  + (b - b'0') as u16 * 10  + (c - b'0') as u16,
        [a, b, c, d] => (a - b'0') as u16 * 1000 + (b - b'0') as u16 * 100 + (c - b'0') as u16 * 10 + (d - b'0') as u16,
        _            => 0,
    }
}

/// Trim ASCII whitespace from both ends of a byte slice without allocation.
/// Used only in the message_slices boundary functions and the scalar parse path.
#[inline]
fn trim_bytes(b: &[u8]) -> &[u8] {
    let s = b.iter().position(|x| !x.is_ascii_whitespace()).unwrap_or(b.len());
    let e = b.iter().rposition(|x| !x.is_ascii_whitespace()).map(|i| i + 1).unwrap_or(0);
    if s < e { &b[s..e] } else { &[] }
}

// ── Scalar path ───────────────────────────────────────────────────────────────

/// Parse a single FIX message from raw bytes (pipe or SOH delimited).
/// Used by the validator for single-message debugger mode.
pub fn parse_single_for_validation(raw: &[u8]) -> FixMessage {
    parse_single_simd(raw)
}

/// Parse a single FIX message string into a [`FixMessage`].
fn parse_single(raw: &str) -> FixMessage {
    let mut msg = FixMessage {
        fields: Vec::with_capacity(24),
        ..Default::default()
    };

    let bytes = raw.as_bytes();
    let mut start = 0;

    for end in memchr_iter(b'|', bytes).chain(std::iter::once(bytes.len())) {
        let token = raw[start..end].trim();
        start = end + 1;
        if token.is_empty() { continue; }
        let Some((tag, value)) = token.split_once('=') else { continue };
        let tag_num: u16 = tag.bytes().fold(0u16, |a, b| a * 10 + (b - b'0') as u16);

        msg.fields.push(FixField {
            tag: tag_num,
            value: CompactString::from(value),
        });

        match tag {
            "52"  => msg.time         = extract_time(value),
            "49"  => msg.sender       = CompactString::from(value),
            "56"  => msg.target       = CompactString::from(value),
            "35"  => {
                msg.msg_type_raw   = CompactString::from(value);
                msg.msg_type_label = msg_type_label(value);
            }
            "11"  => msg.cl_ord_id    = CompactString::from(value),
            "117" => msg.quote_id     = CompactString::from(value),
            "131" => msg.quote_req_id = CompactString::from(value),
            "54"  => msg.side         = CompactString::from(side_label(value)),
            "38"  => msg.order_qty    = CompactString::from(value),
            "55"  => msg.symbol      = CompactString::from(value),
            "58"  => msg.text        = CompactString::from(value),
            "150" => {
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
    msg
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
}
