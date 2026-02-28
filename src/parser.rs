use std::borrow::Cow;

use compact_str::{CompactString, format_compact};
use rayon::prelude::*;

use crate::dictionary::{msg_type_label, side_label, tag_description};
use crate::model::{FixField, FixMessage};

/// Normalize delimiters: SOH (0x01), \x01, ^A -> pipe.
/// Returns a borrowed slice when no special delimiters are present (zero allocation).
fn normalize_delimiters(input: &str) -> Cow<'_, str> {
    // Fast byte scan: if no SOH, backslash, or caret exists, nothing to do.
    if !input.bytes().any(|b| matches!(b, 0x01 | b'\\' | b'^')) {
        return Cow::Borrowed(input);
    }
    Cow::Owned(
        input
            .replace('\u{01}', "|")   // SOH byte
            .replace("\\x01", "|")    // literal \x01 text
            .replace("^A", "|"),      // caret-A
    )
}

/// Split `input` on "8=FIX" boundaries, returning borrowed slices (no allocation per message).
fn message_slices(input: &str) -> Vec<&str> {
    let mut msgs = Vec::new();
    let mut start = 0;
    let mut found = false;

    for (pos, _) in input.match_indices("8=FIX") {
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

    if !normalized.contains("8=FIX") {
        return if normalized.trim().is_empty() { vec![] } else { vec![parse_single(&normalized)] };
    }

    message_slices(&normalized)
        .into_par_iter()
        .map(parse_single)
        .collect()
}

/// Parse a single FIX message string into a [`FixMessage`].
fn parse_single(raw: &str) -> FixMessage {
    // Use a fixed capacity — typical FIX message has 10-25 fields.
    // Avoids a full O(n) pre-scan of `raw.matches('|').count()`.
    let mut msg = FixMessage {
        fields: Vec::with_capacity(24),
        ..Default::default()
    };

    for token in raw.split('|') {
        let token = token.trim();
        if token.is_empty() { continue; }
        let Some((tag, value)) = token.split_once('=') else { continue };

        // CompactString::from(&str) is inline (no heap alloc) for strings ≤ 24 bytes.
        // FIX tags are 1-5 chars, values are usually ≤ 24 chars — nearly zero heap.
        msg.fields.push(FixField {
            tag: CompactString::from(tag),
            value: CompactString::from(value),
            tag_description: tag_description(tag),
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
