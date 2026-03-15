use compact_str::CompactString;

/// A single parsed FIX message with its extracted header fields.
#[derive(Clone, Default, PartialEq)]
pub struct FixMessage {
    pub fields: Vec<FixField>,
    pub time: CompactString,
    pub sender: CompactString,
    pub target: CompactString,
    pub msg_type_raw: CompactString,
    pub msg_type_label: &'static str,
    pub cl_ord_id: CompactString,
    pub quote_id: CompactString,
    pub quote_req_id: CompactString,
    pub side: CompactString,
    pub order_qty: CompactString,
    pub symbol: CompactString,
    pub text: CompactString,
}

/// A single tag=value pair.
/// `tag` is stored as `u16` (the numeric FIX tag number) to avoid a
/// `CompactString` heap/inline construction per field — reduces FixField
/// from 48 bytes to 32 bytes and eliminates 15M+ string constructions for
/// a 1M-message parse.
#[derive(Clone, PartialEq)]
pub struct FixField {
    pub tag: u16,
    pub value: CompactString,
}
