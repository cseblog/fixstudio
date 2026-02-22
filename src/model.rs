/// A single parsed FIX message with its extracted header fields.
#[derive(Clone, Default, PartialEq)]
pub struct FixMessage {
    pub fields: Vec<FixField>,
    pub time: String,
    pub sender: String,
    pub target: String,
    pub msg_type_raw: String,
    pub msg_type_label: &'static str,
    pub cl_ord_id: String,
    pub side: String,
    pub order_qty: String,
    pub symbol: String,
    pub text: String,
}

/// A single tag=value pair with resolved descriptions.
#[derive(Clone, PartialEq)]
pub struct FixField {
    pub tag: String,
    pub value: String,
    pub tag_description: &'static str,
    pub value_description: String,
}
