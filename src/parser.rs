use std::borrow::Cow;

use compact_str::{CompactString, format_compact};
use memchr::{memchr3, memchr_iter, memmem};
use rayon::prelude::*;

use crate::dictionary::{msg_type_label, side_label};
use crate::model::{FixField, FixMessage};

/// Normalize delimiters: SOH (0x01), \x01, ^A -> pipe.
/// Returns a borrowed slice when no special delimiters are present (zero allocation).
/// When conversion is needed, a single pass is used — three chained `.replace()` calls
/// would each allocate the full buffer, tripling peak memory for large files.
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

    for pos in memmem::find_iter(input.as_bytes(), "8=FIX") {
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
            "52" => msg.time = extract_time(value),
            "49" => msg.sender = CompactString::from(value),
            "56" => msg.target = CompactString::from(value),
            "35" => {
                msg.msg_type_raw = CompactString::from(value);
                msg.msg_type_label = msg_type_label(value);
            }
            "11" => msg.cl_ord_id = CompactString::from(value),
            "54" => msg.side = CompactString::from(side_label(value)),
            "38" => msg.order_qty = CompactString::from(value),
            "55" => msg.symbol = CompactString::from(value),
            "58" => msg.text = CompactString::from(value),
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
}
