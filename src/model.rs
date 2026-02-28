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
    pub side: CompactString,
    pub order_qty: CompactString,
    pub symbol: CompactString,
    pub text: CompactString,
}

/// A single tag=value pair. tag_description is resolved lazily in the UI.
#[derive(Clone, PartialEq)]
pub struct FixField {
    pub tag: CompactString,
    pub value: CompactString,
}
