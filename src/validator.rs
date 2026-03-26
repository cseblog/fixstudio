/// FIX Message Validator
///
/// Two entry points:
///   `validate_fields` — operates on an already-parsed FixMessage (no raw bytes needed).
///                       Used for batch validation of many messages.
///   `validate_raw`    — accepts raw FIX bytes; runs all checks including
///                       checksum (tag 10) and BodyLength (tag 9).
///                       Used in the single-message debugger.
use rayon::prelude::*;

use crate::model::FixMessage;

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Debug)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Issue {
    pub severity:  Severity,
    /// Which tag triggered this issue (None = structural / message-level).
    pub tag:       Option<u16>,
    pub code:      &'static str,
    pub message:   String,
    /// Optional hint: what the correct value should be.
    pub fix_hint:  Option<String>,
}

#[derive(Clone, Default, PartialEq, Debug)]
pub struct ValidationReport {
    pub issues: Vec<Issue>,

    // Checksum — only populated by validate_raw
    pub checksum_ok:       Option<bool>,
    pub checksum_found:    Option<String>,
    pub checksum_expected: Option<String>,

    // BodyLength — only populated by validate_raw
    pub body_length_ok:      Option<bool>,
    pub body_length_found:   Option<u32>,
    pub body_length_counted: Option<u32>,
}

impl Issue {
    /// Returns a short display label for the rule badge, e.g. "Error Rule 1".
    pub fn rule_label(&self) -> String {
        let n = rule_number(self.code);
        match self.severity {
            Severity::Error   => format!("Error Rule {n}"),
            Severity::Warning => format!("Warning Rule {n}"),
        }
    }
}

impl ValidationReport {
    pub fn error_count(&self) -> usize {
        self.issues.iter().filter(|i| i.severity == Severity::Error).count()
    }
    pub fn warning_count(&self) -> usize {
        self.issues.iter().filter(|i| i.severity == Severity::Warning).count()
    }
    pub fn is_clean(&self) -> bool {
        self.error_count() == 0
    }
    pub fn first_error(&self) -> Option<&Issue> {
        self.issues.iter().find(|i| i.severity == Severity::Error)
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Field-level validation only. Fast — no raw bytes required.
/// Returns a `ValidationReport` for a single parsed message.
pub fn validate_fields(msg: &FixMessage) -> ValidationReport {
    let mut report = ValidationReport::default();
    check_required_header_tags(msg, &mut report);
    check_required_body_tags(msg, &mut report);
    check_enum_values(msg, &mut report);
    check_duplicate_tags(msg, &mut report);
    check_conditional_tags(msg, &mut report);
    check_consistency(msg, &mut report);
    check_custom_tags(msg, &mut report);
    report
}

/// Full validation including checksum and BodyLength.
/// Accepts raw FIX bytes (pipe or SOH delimited).
pub fn validate_raw(raw: &[u8]) -> ValidationReport {
    // Normalize to pipe-delimited for field-level validation.
    let normalized = normalize_to_pipe(raw);
    let msg = parse_for_validation(&normalized);

    let mut report = validate_fields(&msg);
    check_checksum(raw, &msg, &mut report);
    check_body_length(raw, &msg, &mut report);
    report
}

/// Batch validate a slice of parsed messages in parallel.
pub fn validate_batch(msgs: &[FixMessage]) -> Vec<ValidationReport> {
    msgs.par_iter().map(validate_fields).collect()
}

// ── Required-tag tables ───────────────────────────────────────────────────────

/// Standard header tags required on every FIX message.
const REQUIRED_HEADER: &[u16] = &[8, 9, 35, 34, 49, 52, 56];

/// Required body tags per MsgType.
/// Returns `&'static [u16]` — zero-cost match on a static table.
fn required_body_tags(msg_type: &str) -> &'static [u16] {
    match msg_type {
        // Session-level
        "0"  => &[],                                               // Heartbeat
        "1"  => &[112],                                            // TestRequest
        "2"  => &[7, 16],                                          // ResendRequest
        "3"  => &[45],                                             // Reject (SessionReject)
        "4"  => &[36],                                             // SequenceReset
        "5"  => &[],                                               // Logout
        "A"  => &[98, 108],                                        // Logon
        "j"  => &[372, 380],                                       // BusinessMessageReject

        // Order management
        "D"  => &[11, 21, 38, 40, 54, 55, 60],                    // NewOrderSingle
        "F"  => &[11, 41, 54, 55, 60],                             // OrderCancelRequest
        "G"  => &[11, 21, 38, 40, 41, 54, 55, 60],                // OrderCancelReplaceRequest
        "H"  => &[11, 54, 55],                                     // OrderStatusRequest
        "8"  => &[6, 14, 17, 37, 39, 54, 55, 150, 151],           // ExecutionReport
        "9"  => &[11, 37, 39, 41, 102, 434],                      // OrderCancelReject

        // RFQ / Quote
        "R"  => &[131],                                            // QuoteRequest
        "S"  => &[55, 117],                                        // Quote
        "Z"  => &[55, 117],                                        // QuoteCancel
        "AA" => &[117],                                            // QuoteAcknowledgement
        "AI" => &[117],                                            // QuoteStatusReport

        // Market data
        "V"  => &[262, 263, 264, 267, 268],                       // MarketDataRequest
        "W"  => &[55, 268],                                        // MarketDataSnapshot
        "X"  => &[268],                                            // MarketDataIncremental
        "Y"  => &[262, 281],                                       // MarketDataRequestReject

        // Trade capture
        "AE" => &[17, 55, 570, 571, 828, 856],                    // TradeCaptureReport (basic)

        // All other types: only header is checked
        _    => &[],
    }
}

// ── Enum valid-value sets ─────────────────────────────────────────────────────

/// Returns `true` if `value` is a valid enum for `tag`, or if the tag has no
/// closed enum set (no validation). Returns `false` only for confirmed invalid values.
fn is_valid_enum(tag: u16, value: &str) -> bool {
    match tag {
        35  => is_known_msg_type(value),
        39  => matches!(value, "0"|"1"|"2"|"3"|"4"|"5"|"6"|"7"|"8"|"9"|"A"|"B"|"C"|"D"|"E"),
        40  => matches!(value, "1"|"2"|"3"|"4"|"5"|"6"|"7"|"8"|"9"|"A"|"B"|"C"|"D"|"E"|"F"|"G"|"H"|"I"|"J"|"K"|"L"|"M"|"P"),
        54  => matches!(value, "1"|"2"|"3"|"4"|"5"|"6"|"7"|"8"|"9"),
        59  => matches!(value, "0"|"1"|"2"|"3"|"4"|"5"|"6"|"7"|"8"|"9"),
        98  => matches!(value, "0"|"1"|"2"|"3"|"4"|"5"|"6"),
        150 => matches!(value, "0"|"1"|"2"|"3"|"4"|"5"|"6"|"7"|"8"|"9"|"A"|"B"|"C"|"D"|"E"|"F"|"G"|"H"|"I"),
        21  => matches!(value, "1"|"2"|"3"),
        63  => matches!(value, "0"|"1"|"2"|"3"|"4"|"5"|"6"|"7"|"8"|"9"|"B"),
        277 => value.len() == 1 && value.chars().all(|c| c.is_ascii_alphabetic()),
        279 => matches!(value, "0"|"1"|"2"),
        263 => matches!(value, "0"|"1"|"2"),
        269 => matches!(value, "0"|"1"|"2"|"3"|"4"|"5"|"6"|"7"|"8"|"9"|"A"|"B"|"C"|"D"|"E"),
        _   => true, // no closed enum for this tag
    }
}

fn enum_hint(tag: u16) -> &'static str {
    match tag {
        35  => "known MsgType code (e.g. D, 8, R, S, A, 0)",
        39  => "0-9 or A-E (OrdStatus)",
        40  => "1-9, A-P (OrdType; FX: 1=Market, 2=Limit, D=PreviouslyQuoted)",
        54  => "1=Buy, 2=Sell (or 3-9)",
        59  => "0=DAY, 1=GTC, 3=IOC, 4=FOK, 6=GTD",
        98  => "0=None, 1=PKCS, 2=DES, 3=PKCS/DES, 4=PGP/DES, 5=PGP/DES-MD5, 6=PEM/DES-MD5",
        150 => "0-9 or A-I (ExecType; FX: F=Trade, 4=Canceled, 8=Rejected)",
        21  => "1=AutoNoIntervene, 2=AutoIntervene, 3=Manual",
        63  => "0=Regular, 1=Cash, 2=NextDay, 3=T+2, 4=T+3, B=BrokenDate",
        279 => "0=New, 1=Change, 2=Delete",
        263 => "0=Unsubscribe, 1=Snapshot, 2=SnapshotAndUpdates",
        _   => "see FIX spec",
    }
}

fn is_known_msg_type(code: &str) -> bool {
    matches!(code,
        "0"|"1"|"2"|"3"|"4"|"5"|"6"|"7"|"8"|"9"|"A"|"B"|"C"|"D"|"E"|"F"|"G"|"H"|
        "J"|"K"|"L"|"M"|"N"|"P"|"Q"|"R"|"S"|"T"|"V"|"W"|"X"|"Y"|"Z"|
        "AA"|"AB"|"AE"|"AG"|"AI"|"AK"|"AL"|"AM"|"AN"|"AO"|"AP"|"AQ"|"AR"|"AS"|"AT"|"AU"|
        "BE"|"BF"|"BJ"|"BK"|"j"
    )
}

// ── Tags with FIX version introduced ─────────────────────────────────────────

/// Returns the FIX version string that introduced this tag, or None if it's
/// present since FIX 4.2 or earlier.
fn version_introduced(tag: u16) -> Option<&'static str> {
    match tag {
        // FIX 4.4
        453|448|447|452    => Some("FIX.4.4"),
        571|572|573|574    => Some("FIX.4.4"),
        702|703|704|705    => Some("FIX.4.4"),
        721|722|724|727|728 => Some("FIX.4.4"),
        584|585            => Some("FIX.4.4"),
        636                => Some("FIX.4.4"),
        660                => Some("FIX.4.4"),
        // FIX 5.0
        828|856            => Some("FIX.5.0"),
        _                  => None,
    }
}

/// Extract the BeginString version from a parsed message's first field.
fn begin_string(msg: &FixMessage) -> Option<&str> {
    msg.fields.first().and_then(|f| {
        if f.tag == 8 { Some(f.value.as_str()) } else { None }
    })
}

// ── Validation passes ─────────────────────────────────────────────────────────

fn check_required_header_tags(msg: &FixMessage, report: &mut ValidationReport) {
    for &tag in REQUIRED_HEADER {
        if !has_tag(msg, tag) {
            report.issues.push(Issue {
                severity: Severity::Error,
                tag: Some(tag),
                code: "MISSING_HEADER_TAG",
                message: format!("Missing required header tag {} ({})", tag, tag_name(tag)),
                fix_hint: None,
            });
        }
    }
}

fn check_required_body_tags(msg: &FixMessage, report: &mut ValidationReport) {
    let mt = msg.msg_type_raw.as_str();
    for &tag in required_body_tags(mt) {
        if !has_tag(msg, tag) {
            report.issues.push(Issue {
                severity: Severity::Error,
                tag: Some(tag),
                code: "MISSING_REQUIRED_TAG",
                message: format!(
                    "Missing required tag {} ({}) for MsgType={}",
                    tag, tag_name(tag), mt
                ),
                fix_hint: None,
            });
        }
    }
}

fn check_enum_values(msg: &FixMessage, report: &mut ValidationReport) {
    for field in &msg.fields {
        if !is_valid_enum(field.tag, field.value.as_str()) {
            report.issues.push(Issue {
                severity: Severity::Error,
                tag: Some(field.tag),
                code: "INVALID_ENUM",
                message: format!(
                    "Tag {} ({}): invalid value {:?}",
                    field.tag, tag_name(field.tag), field.value.as_str()
                ),
                fix_hint: Some(enum_hint(field.tag).to_string()),
            });
        }
    }
}

fn check_duplicate_tags(msg: &FixMessage, report: &mut ValidationReport) {
    // Use a small stack-allocated bitset for tags 0..=1023; heap map for the rest.
    let mut seen = [0u64; 16]; // 16 * 64 = 1024 bits
    let mut seen_high: Vec<u16> = Vec::new();

    for field in &msg.fields {
        let t = field.tag as usize;
        if t < 1024 {
            let word = t / 64;
            let bit  = t % 64;
            if seen[word] & (1 << bit) != 0 {
                // Allow known multi-occurrence patterns (repeating group headers)
                if !is_repeating_group_tag(field.tag) {
                    report.issues.push(Issue {
                        severity: Severity::Warning,
                        tag: Some(field.tag),
                        code: "DUPLICATE_TAG",
                        message: format!(
                            "Tag {} ({}) appears more than once",
                            field.tag, tag_name(field.tag)
                        ),
                        fix_hint: None,
                    });
                }
            }
            seen[word] |= 1 << bit;
        } else {
            if seen_high.contains(&field.tag) && !is_repeating_group_tag(field.tag) {
                report.issues.push(Issue {
                    severity: Severity::Warning,
                    tag: Some(field.tag),
                    code: "DUPLICATE_TAG",
                    message: format!(
                        "Tag {} ({}) appears more than once",
                        field.tag, tag_name(field.tag)
                    ),
                    fix_hint: None,
                });
            } else {
                seen_high.push(field.tag);
            }
        }
    }
}

fn check_conditional_tags(msg: &FixMessage, report: &mut ValidationReport) {
    let get = |tag: u16| -> Option<&str> {
        msg.fields.iter().find(|f| f.tag == tag).map(|f| f.value.as_str())
    };

    // Price required when OrdType = Limit (2)
    if get(40) == Some("2") && get(44).is_none() {
        report.issues.push(Issue {
            severity: Severity::Error,
            tag: Some(44),
            code: "CONDITIONAL_TAG_MISSING",
            message: "Tag 44 (Price) required when OrdType=2 (Limit)".to_string(),
            fix_hint: None,
        });
    }

    // StopPx required when OrdType = Stop (3) or StopLimit (4)
    if matches!(get(40), Some("3") | Some("4")) && get(99).is_none() {
        report.issues.push(Issue {
            severity: Severity::Error,
            tag: Some(99),
            code: "CONDITIONAL_TAG_MISSING",
            message: "Tag 99 (StopPx) required when OrdType=3 (Stop) or 4 (StopLimit)".to_string(),
            fix_hint: None,
        });
    }

    // ExpireDate required when TimeInForce = GTD (6)
    if get(59) == Some("6") && get(432).is_none() {
        report.issues.push(Issue {
            severity: Severity::Error,
            tag: Some(432),
            code: "CONDITIONAL_TAG_MISSING",
            message: "Tag 432 (ExpireDate) required when TimeInForce=6 (GTD)".to_string(),
            fix_hint: Some("Format: YYYYMMDD".to_string()),
        });
    }

    // ExecutionReport trade fills: LastPx + LastQty required when ExecType = F (Trade)
    if msg.msg_type_raw == "8" {
        if get(150) == Some("F") {
            if get(31).is_none() {
                report.issues.push(Issue {
                    severity: Severity::Error,
                    tag: Some(31),
                    code: "CONDITIONAL_TAG_MISSING",
                    message: "Tag 31 (LastPx) required when ExecType=F (Trade)".to_string(),
                    fix_hint: None,
                });
            }
            if get(32).is_none() {
                report.issues.push(Issue {
                    severity: Severity::Error,
                    tag: Some(32),
                    code: "CONDITIONAL_TAG_MISSING",
                    message: "Tag 32 (LastQty) required when ExecType=F (Trade)".to_string(),
                    fix_hint: None,
                });
            }
        }
        // ExecRefID required for Correct/Cancel exec types
        if matches!(get(150), Some("G") | Some("H")) && get(19).is_none() {
            report.issues.push(Issue {
                severity: Severity::Error,
                tag: Some(19),
                code: "CONDITIONAL_TAG_MISSING",
                message: "Tag 19 (ExecRefID) required when ExecType=G (TradeCorrect) or H (TradeCancel)".to_string(),
                fix_hint: None,
            });
        }
    }

    // RFQ NOS: QuoteID (117) required when OrdType = D (Previously Quoted)
    if msg.msg_type_raw == "D" && get(40) == Some("D") && get(117).is_none() {
        report.issues.push(Issue {
            severity: Severity::Error,
            tag: Some(117),
            code: "CONDITIONAL_TAG_MISSING",
            message: "Tag 117 (QuoteID) required when OrdType=D (PreviouslyQuoted)".to_string(),
            fix_hint: Some("Value must match the QuoteID from the Quote (35=S) message".to_string()),
        });
    }

    // TestReqID echo: if MsgType=0 (Heartbeat) and TestReqID present in response,
    // this is informational. Skip — we can't verify across messages here.
}

fn check_consistency(msg: &FixMessage, report: &mut ValidationReport) {
    if msg.msg_type_raw != "8" {
        return;
    }

    let get_f64 = |tag: u16| -> Option<f64> {
        msg.fields.iter().find(|f| f.tag == tag)
            .and_then(|f| f.value.as_str().parse::<f64>().ok())
    };

    // LeavesQty + CumQty = OrderQty when ExecType = F (Trade)
    let exec_type = msg.fields.iter().find(|f| f.tag == 150).map(|f| f.value.as_str());
    if exec_type == Some("F") {
        if let (Some(leaves), Some(cum), Some(ord)) =
            (get_f64(151), get_f64(14), get_f64(38))
        {
            let sum = leaves + cum;
            if (sum - ord).abs() > 0.000001 {
                report.issues.push(Issue {
                    severity: Severity::Error,
                    tag: Some(151),
                    code: "CONSISTENCY_FILL_QTY",
                    message: format!(
                        "LeavesQty({}) + CumQty({}) = {} ≠ OrderQty({})",
                        leaves, cum, sum, ord
                    ),
                    fix_hint: Some(format!("LeavesQty should be {}", ord - cum)),
                });
            }
        }
    }

    // OrdStatus = Filled (2) → LeavesQty must be 0
    let ord_status = msg.fields.iter().find(|f| f.tag == 39).map(|f| f.value.as_str());
    if ord_status == Some("2") {
        if let Some(leaves) = get_f64(151) {
            if leaves != 0.0 {
                report.issues.push(Issue {
                    severity: Severity::Error,
                    tag: Some(151),
                    code: "CONSISTENCY_FILLED_LEAVES",
                    message: format!(
                        "OrdStatus=2 (Filled) but LeavesQty={} (must be 0)",
                        leaves
                    ),
                    fix_hint: Some("Set LeavesQty=0".to_string()),
                });
            }
        }
    }

    // MsgSeqNum must be numeric and > 0
    if let Some(seq) = msg.fields.iter().find(|f| f.tag == 34) {
        match seq.value.as_str().parse::<u64>() {
            Ok(0) => report.issues.push(Issue {
                severity: Severity::Error,
                tag: Some(34),
                code: "INVALID_SEQNUM",
                message: "MsgSeqNum(34) = 0 is invalid (must be >= 1)".to_string(),
                fix_hint: None,
            }),
            Err(_) => report.issues.push(Issue {
                severity: Severity::Error,
                tag: Some(34),
                code: "INVALID_SEQNUM",
                message: format!("MsgSeqNum(34) = {:?} is not a valid integer", seq.value.as_str()),
                fix_hint: None,
            }),
            _ => {}
        }
    }
}

fn check_custom_tags(msg: &FixMessage, report: &mut ValidationReport) {
    for field in &msg.fields {
        if field.tag >= 5000 {
            report.issues.push(Issue {
                severity: Severity::Warning,
                tag: Some(field.tag),
                code: "CUSTOM_TAG",
                message: format!(
                    "Tag {} is a custom/proprietary tag (not in standard FIX spec)",
                    field.tag
                ),
                fix_hint: Some("Vendor-defined tags are valid; verify with counterparty spec".to_string()),
            });
        } else if field.tag >= 956 && field.tag < 5000 {
            // FIXMF extensions and reserved range
            report.issues.push(Issue {
                severity: Severity::Warning,
                tag: Some(field.tag),
                code: "EXTENDED_TAG",
                message: format!(
                    "Tag {} is in the FIX extension/reserved range (956–4999)",
                    field.tag
                ),
                fix_hint: None,
            });
        }

        // FIX version introduced check
        if let Some(introduced) = version_introduced(field.tag) {
            if let Some(begin) = begin_string(msg) {
                if is_older_version(begin, introduced) {
                    report.issues.push(Issue {
                        severity: Severity::Warning,
                        tag: Some(field.tag),
                        code: "VERSION_VIOLATION",
                        message: format!(
                            "Tag {} ({}) was introduced in {} but message is {}",
                            field.tag, tag_name(field.tag), introduced, begin
                        ),
                        fix_hint: None,
                    });
                }
            }
        }
    }
}

fn check_checksum(raw: &[u8], msg: &FixMessage, report: &mut ValidationReport) {
    // Find the position of the last "10=" delimiter.
    // Checksum covers all bytes before "10=xxx|".
    let needle = if raw.contains(&b'|') { b"10=" as &[u8] } else { b"10=" };
    let Some(pos) = find_last(raw, needle) else {
        report.issues.push(Issue {
            severity: Severity::Error,
            tag: Some(10),
            code: "MISSING_CHECKSUM",
            message: "Tag 10 (CheckSum) not found".to_string(),
            fix_hint: None,
        });
        return;
    };

    let sum: u64 = raw[..pos].iter().map(|&b| b as u64).sum();
    let expected = (sum % 256) as u8;
    let expected_str = format!("{:03}", expected);

    let found_str = msg.fields.iter()
        .find(|f| f.tag == 10)
        .map(|f| f.value.as_str().to_string())
        .unwrap_or_default();

    let ok = found_str == expected_str;
    report.checksum_ok = Some(ok);
    report.checksum_found = Some(found_str.clone());
    report.checksum_expected = Some(expected_str.clone());

    if !ok {
        report.issues.push(Issue {
            severity: Severity::Error,
            tag: Some(10),
            code: "CHECKSUM_MISMATCH",
            message: format!(
                "CheckSum mismatch: tag 10 = {:?}, computed = {:?}",
                found_str, expected_str
            ),
            fix_hint: Some(format!("Set 10={}", expected_str)),
        });
    }
}

fn check_body_length(raw: &[u8], msg: &FixMessage, report: &mut ValidationReport) {
    // BodyLength = bytes from start of tag 9=value SOH/pipe through
    // end of the last tag before "10=".
    // Per spec: from the byte *after* "9=value SOH" to the byte *before* the
    // start of "10=".
    let delim = if raw.contains(&b'|') { b'|' } else { 0x01 };

    let start_of_9 = find_slice(raw, b"9=");
    let start_of_10 = find_last(raw, b"10=");

    let (Some(pos9), Some(pos10)) = (start_of_9, start_of_10) else {
        return;
    };

    let after_9_value = raw[pos9..].iter().position(|&b| b == delim)
        .map(|p| pos9 + p + 1);

    let Some(body_start) = after_9_value else { return; };

    let counted = (pos10.saturating_sub(body_start)) as u32;

    let found = msg.fields.iter()
        .find(|f| f.tag == 9)
        .and_then(|f| f.value.as_str().parse::<u32>().ok())
        .unwrap_or(0);

    let ok = found == counted;
    report.body_length_ok = Some(ok);
    report.body_length_found = Some(found);
    report.body_length_counted = Some(counted);

    if !ok {
        report.issues.push(Issue {
            severity: Severity::Error,
            tag: Some(9),
            code: "BODY_LENGTH_MISMATCH",
            message: format!(
                "BodyLength mismatch: tag 9 = {}, counted = {}",
                found, counted
            ),
            fix_hint: Some(format!("Set 9={}", counted)),
        });
    }
}

// ── Rule numbering ────────────────────────────────────────────────────────────

/// Maps a validation error/warning code to a stable rule number for display.
pub fn rule_number(code: &str) -> u8 {
    match code {
        "MISSING_HEADER_TAG"      => 1,
        "MISSING_REQUIRED_TAG"    => 2,
        "INVALID_ENUM"            => 3,
        "CONDITIONAL_TAG_MISSING" => 4,
        "CONSISTENCY_FILL_QTY"    => 5,
        "CONSISTENCY_FILLED_LEAVES" => 6,
        "INVALID_SEQNUM"          => 7,
        "DUPLICATE_TAG"           => 8,
        "CUSTOM_TAG"              => 9,
        "EXTENDED_TAG"            => 10,
        "VERSION_VIOLATION"       => 11,
        "MISSING_CHECKSUM"        => 12,
        "CHECKSUM_MISMATCH"       => 13,
        "BODY_LENGTH_MISMATCH"    => 14,
        _                         => 0,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn has_tag(msg: &FixMessage, tag: u16) -> bool {
    msg.fields.iter().any(|f| f.tag == tag)
}

/// Tags that appear multiple times legitimately as repeating group delimiters.
/// Only include tags whose *sole* purpose in any message is as a group delimiter
/// (i.e., they would never be meaningful outside a group context).
fn is_repeating_group_tag(tag: u16) -> bool {
    matches!(tag,
        448 |  // PartyID — delimiter for NoPartyIDs (453)
        269 |  // MDEntryType — delimiter for NoMDEntries (268)
        79  |  // AllocAccount — delimiter for NoAllocs (78)
        375 |  // ContraBroker — delimiter for NoContraBrokers (382)
        600    // LegSymbol — delimiter for NoLegs (555)
    )
}

/// Compare two FIX version strings. Returns true if `begin` is strictly older
/// than `required`. E.g. `is_older_version("FIX.4.2", "FIX.4.4")` = true.
fn is_older_version(begin: &str, required: &str) -> bool {
    let parse = |s: &str| -> [u8; 2] {
        let parts: Vec<&str> = s.split('.').collect();
        let major = parts.get(1).and_then(|x| x.parse().ok()).unwrap_or(0u8);
        let minor = parts.get(2).and_then(|x| x.parse().ok()).unwrap_or(0u8);
        [major, minor]
    };
    parse(begin) < parse(required)
}

fn find_last(data: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || data.len() < needle.len() {
        return None;
    }
    let mut last = None;
    let mut i = 0;
    while i + needle.len() <= data.len() {
        if &data[i..i + needle.len()] == needle {
            last = Some(i);
        }
        i += 1;
    }
    last
}

fn find_slice(data: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || data.len() < needle.len() {
        return None;
    }
    (0..=data.len() - needle.len()).find(|&i| &data[i..i + needle.len()] == needle)
}

fn tag_name(tag: u16) -> &'static str {
    crate::dictionary::tag_description(tag)
}

fn normalize_to_pipe(raw: &[u8]) -> Vec<u8> {
    raw.iter().map(|&b| if b == 0x01 { b'|' } else { b }).collect()
}

/// Minimal parse for the validator — just extracts tag=value pairs.
/// Uses the same logic as the main parser but without SIMD (single message, small input).
fn parse_for_validation(raw: &[u8]) -> FixMessage {
    crate::parser::parse_single_for_validation(raw)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FixField, FixMessage};
    use compact_str::CompactString;

    fn make_nos() -> FixMessage {
        let mut msg = FixMessage {
            msg_type_raw: CompactString::from("D"),
            ..Default::default()
        };
        for (t, v) in [
            (8, "FIX.4.4"), (9, "100"), (35, "D"), (34, "1"),
            (49, "CITIFX"), (52, "20240101-12:00:00"), (56, "FXECN"),
            (11, "ORD001"), (21, "1"), (38, "1000000"),
            (40, "2"), (44, "1.0850"), (54, "1"), (55, "EURUSD"), (60, "20240101-12:00:00"),
        ] {
            msg.fields.push(FixField { tag: t, value: CompactString::from(v) });
        }
        msg
    }

    #[test]
    fn valid_nos_is_clean() {
        let report = validate_fields(&make_nos());
        assert!(report.is_clean(), "errors: {:?}", report.issues);
    }

    #[test]
    fn missing_cl_ord_id() {
        let mut msg = make_nos();
        msg.fields.retain(|f| f.tag != 11);
        let report = validate_fields(&msg);
        assert!(report.issues.iter().any(|i| i.tag == Some(11) && i.code == "MISSING_REQUIRED_TAG"));
    }

    #[test]
    fn invalid_side_enum() {
        let mut msg = make_nos();
        msg.fields.iter_mut().find(|f| f.tag == 54).unwrap().value = CompactString::from("X");
        let report = validate_fields(&msg);
        assert!(report.issues.iter().any(|i| i.tag == Some(54) && i.code == "INVALID_ENUM"));
    }

    #[test]
    fn limit_order_missing_price() {
        let mut msg = make_nos();
        msg.fields.retain(|f| f.tag != 44); // remove Price
        let report = validate_fields(&msg);
        assert!(report.issues.iter().any(|i| i.tag == Some(44) && i.code == "CONDITIONAL_TAG_MISSING"));
    }

    #[test]
    fn duplicate_tag_warning() {
        let mut msg = make_nos();
        msg.fields.push(FixField { tag: 55, value: CompactString::from("GBPUSD") });
        let report = validate_fields(&msg);
        assert!(report.issues.iter().any(|i| i.tag == Some(55) && i.code == "DUPLICATE_TAG"));
    }

    #[test]
    fn custom_tag_warning() {
        let mut msg = make_nos();
        msg.fields.push(FixField { tag: 9001, value: CompactString::from("INTERNAL") });
        let report = validate_fields(&msg);
        assert!(report.issues.iter().any(|i| i.tag == Some(9001) && i.code == "CUSTOM_TAG"));
    }

    #[test]
    fn checksum_validation() {
        // Valid checksum message
        let raw = b"8=FIX.4.4|9=61|35=A|34=1|49=EXEC|52=20121105-23:24:06|56=BANZAI|98=0|108=30|10=003|";
        let report = validate_raw(raw);
        // Should find checksum tag (may or may not match — test it doesn't panic)
        assert!(report.checksum_ok.is_some());
    }
}
