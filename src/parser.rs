use rayon::prelude::*;

use crate::dictionary::{msg_type_label, side_label, tag_description};
use crate::model::{FixField, FixMessage};

/// Normalize delimiters: SOH (0x01), \x01, ^A -> pipe.
fn normalize_delimiters(input: &str) -> String {
    input
        .replace('\u{01}', "|")   // SOH byte
        .replace("\\x01", "|")   // literal \x01 text
        .replace("^A", "|")      // caret-A
}

/// Parse a raw input string that may contain multiple FIX messages.
pub fn parse_all(input: &str) -> Vec<FixMessage> {
    let normalized = normalize_delimiters(input);

    // Split on "8=FIX" boundaries (also handles FIXT.1.1)
    let raw_msgs: Vec<String> = if normalized.contains("8=FIX") {
        normalized
            .split("8=FIX")
            .filter_map(|s| {
                let s = s.trim();
                if s.is_empty() { None } else { Some(format!("8=FIX{s}")) }
            })
            .collect()
    } else if !normalized.trim().is_empty() {
        vec![normalized]
    } else {
        return vec![];
    };

    raw_msgs.par_iter().map(|raw| parse_single(raw)).collect()
}

/// Parse a single FIX message string into a [`FixMessage`].
fn parse_single(raw: &str) -> FixMessage {
    let field_count = raw.matches('|').count() + 1;
    let mut msg = FixMessage {
        fields: Vec::with_capacity(field_count),
        ..Default::default()
    };

    for token in raw.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let Some((tag, value)) = token.split_once('=') else {
            continue;
        };

        msg.fields.push(FixField {
            tag: tag.to_string(),
            value: value.to_string(),
            tag_description: tag_description(tag),
        });

        match tag {
            "52" => msg.time = extract_time(value),
            "49" => msg.sender = value.to_string(),
            "56" => msg.target = value.to_string(),
            "35" => {
                msg.msg_type_raw = value.to_string();
                msg.msg_type_label = msg_type_label(value);
            }
            "11" => msg.cl_ord_id = value.to_string(),
            "54" => msg.side = side_label(value).to_string(),
            "38" => msg.order_qty = value.to_string(),
            "55" => msg.symbol = value.to_string(),
            "58" => msg.text = value.to_string(),
            "150" => {
                // If ExecType is filled, override the label
                if value == "F" || value == "2" {
                    msg.msg_type_label = "ER FILL";
                } else if value == "1" {
                    msg.msg_type_label = "ER PARTIAL";
                } else if value == "4" || value == "C" {
                    msg.msg_type_label = "ER CANCELED";
                } else if value == "8" {
                    msg.msg_type_label = "Reject";
                }
            }
            _ => {}
        }
    }
    msg
}

/// Format SendingTime (tag 52) for display: YYYYMMDD-HH:MM:SS -> YYYY-MM-DD HH:MM:SS
fn extract_time(sending_time: &str) -> String {
    // Format: YYYYMMDD-HH:MM:SS or YYYYMMDD-HH:MM:SS.sss
    if let Some(pos) = sending_time.find('-') {
        let date_part = &sending_time[..pos];
        let time_part = &sending_time[pos + 1..];
        // Reformat date: 20121105 -> 2012-11-05
        if date_part.len() == 8 && date_part.chars().all(|c| c.is_ascii_digit()) {
            let formatted = format!(
                "{}-{}-{} {}",
                &date_part[0..4],
                &date_part[4..6],
                &date_part[6..8],
                time_part
            );
            return formatted.trim_end().to_string();
        }
        format!("{date_part} {time_part}")
    } else {
        sending_time.to_string()
    }
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
}
