//! AVX2-accelerated field-delimiter scanner for FIX protocol messages.
//!
//! Core idea: a single `_mm256_cmpeq_epi8` + `_mm256_movemask_epi8` pass over
//! 32-byte windows produces a bitmask where every set bit marks a delimiter.
//! Two `cmpeq` results are OR-ed together so SOH (0x01) **and** pipe ('|') are
//! both found in a single instruction — no normalisation step required.
//!
//! Falls back to a scalar loop on non-x86_64 targets or when AVX2 is absent
//! at runtime.

/// Returns positions of every SOH (0x01) or pipe ('|') byte in `bytes`.
///
/// On x86-64 with AVX2 this processes 32 bytes per iteration using:
/// ```text
/// chunk    = load 256-bit window
/// any_delim = cmpeq(chunk, soh) | cmpeq(chunk, pipe)
/// mask     = movemask(any_delim)   // 32-bit: bit i set ↔ byte i is a delimiter
/// ```
/// All set bits in `mask` are extracted with `trailing_zeros` + bit-clear tricks.
pub fn find_delimiters(bytes: &[u8]) -> Vec<usize> {
    #[cfg(target_arch = "x86_64")]
    if is_x86_feature_detected!("avx2") {
        return unsafe { avx2_delimiters(bytes) };
    }
    scalar_delimiters(bytes)
}

// ── AVX2 path ────────────────────────────────────────────────────────────────

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn avx2_delimiters(bytes: &[u8]) -> Vec<usize> {
    use std::arch::x86_64::*;

    let soh_vec  = _mm256_set1_epi8(0x01_i8);
    let pipe_vec = _mm256_set1_epi8(b'|' as i8);

    let n      = bytes.len();
    let chunks = n / 32;
    // Rough capacity: 1 delimiter per ~8 bytes is typical for FIX messages.
    let mut out = Vec::with_capacity(n / 8);

    for i in 0..chunks {
        let chunk = _mm256_loadu_si256(bytes.as_ptr().add(i * 32) as *const __m256i);

        // OR the two comparisons — finds SOH and pipe simultaneously.
        let any  = _mm256_or_si256(
            _mm256_cmpeq_epi8(chunk, soh_vec),
            _mm256_cmpeq_epi8(chunk, pipe_vec),
        );
        // Each bit in `mask` corresponds to one byte in the 32-byte window.
        let mut mask = _mm256_movemask_epi8(any) as u32;

        let base = i * 32;
        while mask != 0 {
            out.push(base + mask.trailing_zeros() as usize);
            mask &= mask - 1; // clear the lowest set bit
        }
    }

    // Scalar tail for the remaining < 32 bytes.
    for i in (chunks * 32)..n {
        if bytes[i] == 0x01 || bytes[i] == b'|' {
            out.push(i);
        }
    }

    out
}

// ── Scalar fallback ──────────────────────────────────────────────────────────

fn scalar_delimiters(bytes: &[u8]) -> Vec<usize> {
    bytes
        .iter()
        .enumerate()
        .filter_map(|(i, &b)| (b == 0x01 || b == b'|').then_some(i))
        .collect()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn positions(input: &[u8]) -> Vec<usize> {
        input
            .iter()
            .enumerate()
            .filter_map(|(i, &b)| (b == 0x01 || b == b'|').then_some(i))
            .collect()
    }

    #[test]
    fn matches_scalar_pipe() {
        let input = b"8=FIX.4.4|9=61|35=A|34=1|";
        assert_eq!(find_delimiters(input), positions(input));
    }

    #[test]
    fn matches_scalar_soh() {
        let input = b"8=FIX.4.4\x019=61\x0135=A\x0134=1\x01";
        assert_eq!(find_delimiters(input), positions(input));
    }

    #[test]
    fn matches_scalar_mixed() {
        // Real files sometimes mix SOH and pipe.
        let input = b"8=FIX.4.4|9=61\x0135=A|34=1\x01";
        assert_eq!(find_delimiters(input), positions(input));
    }

    #[test]
    fn handles_64_byte_input() {
        // Exactly 2 AVX2 chunks.
        let mut input = vec![b'X'; 64];
        input[7]  = b'|';
        input[31] = 0x01;
        input[32] = b'|';
        input[63] = 0x01;
        assert_eq!(find_delimiters(&input), positions(&input));
    }

    #[test]
    fn handles_tail_only() {
        // Input shorter than one AVX2 chunk.
        let input = b"35=A|49=EXEC\x01";
        assert_eq!(find_delimiters(input), positions(input));
    }

    #[test]
    fn empty_input() {
        assert_eq!(find_delimiters(&[]), Vec::<usize>::new());
    }
}
