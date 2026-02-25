use crate::dictionary::{msg_type_label, side_label, tag_description, value_description};
use crate::model::{FixField, FixMessage};

/// Parse a raw input string that may contain multiple FIX messages.
pub fn parse_all(input: &str) -> Vec<FixMessage> {
    let normalized = input
        .replace('\u{01}', "|") // SOH
        .replace("\\x01", "|")
        .replace("^A", "|");

    // Split on "8=FIX" boundaries so we can handle multiple messages in one blob
    // (also works for 8=FIXT.1.1 since "8=FIX" matches and remainder is "T.1.1")
    let mut raw_msgs: Vec<String> = Vec::new();
    let mut current = String::new();

    for segment in normalized.split("8=FIX") {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        if !current.is_empty() {
            raw_msgs.push(current.clone());
        }
        current = format!("8=FIX{segment}");
    }
    if !current.is_empty() {
        raw_msgs.push(current);
    }
    // Fallback: if no "8=FIX" found, treat whole input as one message
    if raw_msgs.is_empty() && !normalized.trim().is_empty() {
        raw_msgs.push(normalized);
    }

    raw_msgs.iter().map(|raw| parse_single(raw)).collect()
}

/// Parse a single FIX message string into a [`FixMessage`].
fn parse_single(raw: &str) -> FixMessage {
    let mut msg = FixMessage::default();

    for token in raw.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let Some((tag, value)) = token.split_once('=') else {
            continue;
        };
        let tag_desc = tag_description(tag);
        let val_desc = value_description(tag, value);

        msg.fields.push(FixField {
            tag: tag.to_string(),
            value: value.to_string(),
            tag_description: tag_desc,
            value_description: val_desc,
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

fn extract_time(sending_time: &str) -> String {
    // Format: YYYYMMDD-HH:MM:SS or YYYYMMDD-HH:MM:SS.sss
    if let Some(pos) = sending_time.find('-') {
        sending_time[pos + 1..].to_string()
    } else {
        sending_time.to_string()
    }
}
