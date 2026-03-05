use std::borrow::Cow;

use compact_str::{CompactString, format_compact};
use memchr::{memchr3, memchr_iter, memmem};
use rayon::prelude::*;

use crate::dictionary::{msg_type_label, side_label};
use crate::model::{FixField, FixMessage};

/// Normalize delimiters: SOH (0x01), \x01, ^A -> pipe.
/// Returns a borrowed slice when no special delimiters are present (zero allocation).
fn normalize_delimiters(input: &str) -> Cow<'_, str> {
    // SIMD scan for SOH / backslash / caret — fast zero-alloc check for clean input.
    if memchr3(0x01, b'\\', b'^', input.as_bytes()).is_none() {
        return Cow::Borrowed(input);
    }
    // Single pass — one allocation, exactly input.len() capacity.
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
    let mut msgs = Vec::new();
    let mut start = 0;
    let mut found = false;

    for pos in memmem::find_iter(input.as_bytes(), b"8=FIX") {
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
    let mut msgs = Vec::new();
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

// ── AVX2 / SIMD path ─────────────────────────────────────────────────────────

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
    } else {
        slices.into_par_iter().map(|s| parse_single_simd(s.as_bytes())).collect()
    }
}

/// Like [`parse_all_simd`] but accepts raw bytes — the preferred hot path for
/// memory-mapped file loading (zero copy from the OS page cache).
///
/// The caller passes `&mmap[..]` directly without any `String` conversion.
/// The inlined AVX2 scanner produces no intermediate `Vec<usize>` per message.
pub fn parse_all_simd_bytes(input: &[u8]) -> Vec<FixMessage> {
    let slices = message_slices_bytes(input);
    if slices.is_empty() {
        if input.iter().all(|b| b.is_ascii_whitespace()) {
            vec![]
        } else {
            vec![parse_single_simd(input)]
        }
    } else {
        slices.into_par_iter().map(|s| parse_single_simd(s)).collect()
    }
}

/// Parse a single FIX message from raw bytes — handles both SOH and pipe delimiters.
///
/// **1BRC optimisation**: the previous version called `simd::find_delimiters(raw)`
/// which collected all delimiter positions into a `Vec<usize>` — one heap allocation
/// per message (= 1 M allocs for a 1 M message file). This version inlines the AVX2
/// scan and calls [`apply_token`] per field immediately — **zero intermediate allocs**.
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
    simd_parse_scalar(raw, &mut msg);
    msg
}

/// AVX2 inline scan + parse: 32 bytes per iteration, `apply_token` called per field
/// immediately — no intermediate `Vec<usize>` allocation.
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
        // OR the two comparisons — finds SOH and pipe simultaneously.
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
            mask &= mask - 1; // clear lowest set bit
        }
    }

    // Scalar tail for the remaining < 32 bytes.
    for i in (chunks * 32)..n {
        if raw[i] == 0x01 || raw[i] == b'|' {
            apply_token(raw, start, i, msg);
            start = i + 1;
        }
    }

    // Final field when there is no trailing delimiter.
    if start < n {
        apply_token(raw, start, n, msg);
    }
}

/// Scalar fallback for `parse_single_simd` on non-AVX2 / non-x86_64 targets.
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

/// Extract one `tag=value` token from `raw[start..end]` and update `msg`.
/// Shared by both the AVX2 and scalar parse paths — inlined at each call site.
#[inline(always)]
fn apply_token(raw: &[u8], start: usize, end: usize, msg: &mut FixMessage) {
    if end <= start { return; }
    let token = trim_bytes(&raw[start..end]);

    let Some(eq) = token.iter().position(|&b| b == b'=') else { return };
    let tag_b = trim_bytes(&token[..eq]);
    let val_b = &token[eq + 1..];

    let Ok(tag_str) = std::str::from_utf8(tag_b) else { return };
    let Ok(value)   = std::str::from_utf8(val_b) else { return };

    msg.fields.push(FixField {
        tag:   CompactString::from(tag_str),
        value: CompactString::from(value),
    });

    match tag_str {
        "52"  => msg.time         = extract_time(value),
        "49"  => msg.sender       = CompactString::from(value),
        "56"  => msg.target       = CompactString::from(value),
        "35"  => {
            msg.msg_type_raw   = CompactString::from(value);
            msg.msg_type_label = msg_type_label(value);
        }
        "11"  => msg.cl_ord_id   = CompactString::from(value),
        "54"  => msg.side        = CompactString::from(side_label(value)),
        "38"  => msg.order_qty   = CompactString::from(value),
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

/// Trim ASCII whitespace from both ends of a byte slice without allocation.
#[inline]
fn trim_bytes(b: &[u8]) -> &[u8] {
    let s = b.iter().position(|x| !x.is_ascii_whitespace()).unwrap_or(b.len());
    let e = b.iter().rposition(|x| !x.is_ascii_whitespace()).map(|i| i + 1).unwrap_or(0);
    if s < e { &b[s..e] } else { &[] }
}

// ── Scalar path ───────────────────────────────────────────────────────────────

/// Parse a single FIX message string into a [`FixMessage`].
fn parse_single(raw: &str) -> FixMessage {
    let mut msg = FixMessage {
        fields: Vec::with_capacity(24),
        ..Default::default()
    };

    let bytes = raw.as_bytes();
    let mut start = 0;

    // SIMD '|' search; chain bytes.len() as a sentinel to capture the final token.
    for end in memchr_iter(b'|', bytes).chain(std::iter::once(bytes.len())) {
        let token = raw[start..end].trim();
        start = end + 1;
        if token.is_empty() { continue; }
        let Some((tag, value)) = token.split_once('=') else { continue };

        msg.fields.push(FixField {
            tag: CompactString::from(tag),
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
            "11"  => msg.cl_ord_id   = CompactString::from(value),
            "54"  => msg.side        = CompactString::from(side_label(value)),
            "38"  => msg.order_qty   = CompactString::from(value),
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
        // Pipe-delimited input must NOT allocate (Cow::Borrowed)
        let input = "8=FIX.4.4|9=5|35=0|10=001|";
        let result = normalize_delimiters(input);
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn test_parse_all_simd_bytes_soh() {
        // Verify the new bytes API works end-to-end with SOH input.
        let input = b"8=FIX.4.4\x019=61\x0135=A\x0134=1\x0149=EXEC\x0152=20121105-23:24:06\x0156=BANZAI\x0198=0\x01108=30\x0110=003\x01";
        let msgs = parse_all_simd_bytes(input);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].msg_type_raw, "A");
        assert_eq!(msgs[0].sender, "EXEC");
        assert_eq!(msgs[0].time, "2012-11-05 23:24:06");
    }

    #[test]
    fn test_parse_all_simd_bytes_multi() {
        // Two messages, SOH-delimited — both must be found and parsed correctly.
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
}
