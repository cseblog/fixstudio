use crate::model::FixMessage;

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Converts a slice of parsed FIX messages into a CSV string.
pub fn messages_to_csv(messages: &[FixMessage]) -> String {
    let mut out = String::with_capacity(messages.len() * 80);
    out.push_str("Index,Time,Sender,Target,MsgType,ClOrdID,Side,Symbol,Qty,Text\n");
    for (i, m) in messages.iter().enumerate() {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            i + 1,
            csv_escape(&m.time),
            csv_escape(&m.sender),
            csv_escape(&m.target),
            csv_escape(m.msg_type_label),
            csv_escape(&m.cl_ord_id),
            csv_escape(&m.side),
            csv_escape(&m.symbol),
            csv_escape(&m.order_qty),
            csv_escape(&m.text),
        ));
    }
    out
}
