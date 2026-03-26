use crate::model::FixMessage;

pub(crate) fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Returns a UTC datetime string for use in file names: `yyyy-mm-dd_HH-MM-SS`.
/// Uses the civil_from_days algorithm (Howard Hinnant) — no external crates needed.
pub fn now_tag() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (year, month, day, hour, min, sec) = unix_secs_to_utc(secs);
    format!("{year:04}{month:02}{day:02}_{hour:02}{min:02}{sec:02}")
}

fn unix_secs_to_utc(secs: u64) -> (u16, u8, u8, u8, u8, u8) {
    let s   = (secs % 60) as u8;
    let m   = ((secs / 60) % 60) as u8;
    let h   = ((secs / 3_600) % 24) as u8;
    let z   = (secs / 86_400) as u32 + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y   = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp  = (5 * doy + 2) / 153;
    let d   = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let mo  = if mp < 10 { (mp + 3) as u8 } else { (mp - 9) as u8 };
    let yr  = if mo <= 2 { y + 1 } else { y } as u16;
    (yr, mo, d, h, m, s)
}

/// Serialises a slice of FIX messages to a CSV string (Timeline table columns).
pub fn messages_to_csv(messages: &[FixMessage]) -> String {
    let mut out = String::with_capacity(messages.len() * 120);
    out.push_str("Index,Time,Sender,Target,MsgType,ClOrdID,QuoteID,QuoteReqID,Side,Symbol,Qty,Text\n");
    for (i, m) in messages.iter().enumerate() {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{}\n",
            i + 1,
            csv_escape(&m.time),
            csv_escape(&m.sender),
            csv_escape(&m.target),
            csv_escape(m.msg_type_label),
            csv_escape(&m.cl_ord_id),
            csv_escape(&m.quote_id),
            csv_escape(&m.quote_req_id),
            csv_escape(&m.side),
            csv_escape(&m.symbol),
            csv_escape(&m.order_qty),
            csv_escape(&m.text),
        ));
    }
    out
}
