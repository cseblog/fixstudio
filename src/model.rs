use compact_str::CompactString;

/// A single parsed FIX message with its extracted header fields.
#[derive(Clone, Default, PartialEq)]
pub struct FixMessage {
    /// Flat byte buffer containing all field values, concatenated in parse order.
    ///
    /// Kelley DOD principle: "use indexes instead of pointers."
    /// Each `FixField` stores a `(u32, u16)` offset+length into this arena rather
    /// than an owned `CompactString`. This shrinks `FixField` from 32 bytes → 8 bytes
    /// — a 4× reduction in field-iteration memory bandwidth.
    pub arena:          Vec<u8>,
    pub fields:         Vec<FixField>,
    pub time:           CompactString,
    pub sender:         CompactString,
    pub target:         CompactString,
    pub msg_type_raw:   CompactString,
    pub msg_type_label: &'static str,
    pub cl_ord_id:      CompactString,
    pub quote_id:       CompactString,
    pub quote_req_id:   CompactString,
    pub side:           CompactString,
    pub order_qty:      CompactString,
    pub symbol:         CompactString,
    pub text:           CompactString,
}

impl FixMessage {
    /// Append `value` bytes to the arena and push a new [`FixField`].
    ///
    /// Use this wherever `fields.push(FixField { tag, value: … })` was written.
    #[allow(dead_code)]
    pub fn push_field(&mut self, tag: u16, value: &str) {
        let value_start = self.arena.len() as u32;
        self.arena.extend_from_slice(value.as_bytes());
        let value_len = value.len() as u16;
        self.fields.push(FixField { tag, value_len, value_start });
    }

    /// Replace the value of the first field with `tag` by appending `value` to
    /// the arena. Old arena bytes are not reclaimed (arena is append-only).
    #[allow(dead_code)]
    pub fn set_field_value(&mut self, tag: u16, value: &str) {
        if let Some(idx) = self.fields.iter().position(|f| f.tag == tag) {
            let value_start = self.arena.len() as u32;
            self.arena.extend_from_slice(value.as_bytes());
            let value_len = value.len() as u16;
            self.fields[idx].value_start = value_start;
            self.fields[idx].value_len   = value_len;
        }
    }
}

/// A single tag=value pair stored as arena offsets.
///
/// Memory layout (field order chosen for zero padding):
///   offset 0: tag         (u16, 2 bytes)
///   offset 2: value_len   (u16, 2 bytes)
///   offset 4: value_start (u32, 4 bytes)
///   total: 8 bytes — vs 32 bytes for the old CompactString design (4× smaller).
///
/// At 20 fields/message × 1M messages this saves ~480 MB of field storage and
/// halves the number of cache lines touched when scanning a message's fields.
#[derive(Clone, PartialEq)]
pub struct FixField {
    pub tag:         u16,
    pub value_len:   u16,
    pub value_start: u32,
}

impl FixField {
    /// Borrow the field value as `&str` from the parent message's arena.
    ///
    /// `arena` must be `&msg.arena` for the [`FixMessage`] that owns this field.
    /// FIX protocol is 7-bit ASCII, so the byte slice is always valid UTF-8.
    #[inline]
    pub fn value_in<'a>(&self, arena: &'a [u8]) -> &'a str {
        let start = self.value_start as usize;
        let end   = start + self.value_len as usize;
        // Catch offset corruption early in debug builds before the slice panics
        // with a cryptic index-out-of-bounds rather than a meaningful message.
        debug_assert!(end <= arena.len(),
            "FixField offset out of bounds: end={} arena.len()={}", end, arena.len());
        let slice = &arena[start..end];
        // SAFETY: FIX protocol fields are 7-bit ASCII, which is valid UTF-8.
        unsafe { std::str::from_utf8_unchecked(slice) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── push_field ────────────────────────────────────────────────────────────

    #[test]
    fn push_field_stores_value_in_arena() {
        let mut msg = FixMessage::default();
        msg.push_field(35, "D");
        assert_eq!(msg.fields.len(), 1);
        assert_eq!(msg.fields[0].tag, 35);
        assert_eq!(msg.fields[0].value_in(&msg.arena), "D");
    }

    #[test]
    fn push_field_multiple_values_share_arena() {
        let mut msg = FixMessage::default();
        msg.push_field(49, "CITIFX");
        msg.push_field(56, "FXECN");
        assert_eq!(msg.fields[0].value_in(&msg.arena), "CITIFX");
        assert_eq!(msg.fields[1].value_in(&msg.arena), "FXECN");
        // Both values live in the same arena — total bytes = 6 + 5 = 11.
        assert_eq!(msg.arena.len(), 11);
    }

    #[test]
    fn push_field_empty_value() {
        let mut msg = FixMessage::default();
        msg.push_field(58, "");
        assert_eq!(msg.fields.len(), 1);
        assert_eq!(msg.fields[0].value_len, 0);
        assert_eq!(msg.fields[0].value_in(&msg.arena), "");
    }

    // ── set_field_value ───────────────────────────────────────────────────────

    #[test]
    fn set_field_value_updates_existing_field() {
        let mut msg = FixMessage::default();
        msg.push_field(54, "1");
        msg.set_field_value(54, "2");
        // The field now points to the new "2" appended to the arena.
        assert_eq!(msg.fields[0].value_in(&msg.arena), "2");
    }

    #[test]
    fn set_field_value_no_op_when_tag_absent() {
        let mut msg = FixMessage::default();
        msg.push_field(54, "1");
        msg.set_field_value(99, "X"); // tag 99 does not exist — must not panic
        assert_eq!(msg.fields.len(), 1);
        assert_eq!(msg.fields[0].value_in(&msg.arena), "1");
    }

    // ── value_in ─────────────────────────────────────────────────────────────

    #[test]
    fn value_in_returns_correct_slice() {
        let arena = b"EURUSD".to_vec();
        let field = FixField { tag: 55, value_start: 0, value_len: 6 };
        assert_eq!(field.value_in(&arena), "EURUSD");
    }

    #[test]
    fn value_in_mid_arena_offset() {
        // Simulate two fields packed into one arena: "BUY" at 0, "SELL" at 3.
        let arena = b"BUYSELL".to_vec();
        let f1 = FixField { tag: 54, value_start: 0, value_len: 3 };
        let f2 = FixField { tag: 54, value_start: 3, value_len: 4 };
        assert_eq!(f1.value_in(&arena), "BUY");
        assert_eq!(f2.value_in(&arena), "SELL");
    }
}
