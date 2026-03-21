//! FIX Message Validator — single-message debugger + batch summary.

use dioxus::prelude::*;

use crate::dictionary::tag_description;
use crate::model::FixMessage;
use crate::parser::parse_single_for_validation;
use crate::validator::{validate_batch, validate_raw, Issue, Severity, ValidationReport};

// ── Component props ───────────────────────────────────────────────────────────

#[derive(Props, Clone, PartialEq)]
pub struct ValidatorProps {
    pub messages: Signal<Vec<FixMessage>>,
}

// ── Sub-view tabs ─────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
enum Tab { Single, Batch }

// ── Component ─────────────────────────────────────────────────────────────────

pub fn validator_panel(props: ValidatorProps) -> Element {
    let mut tab             = use_signal(|| Tab::Batch);
    let mut raw_input       = use_signal(String::new);
    let mut report: Signal<Option<ValidationReport>> = use_signal(|| None);
    let mut parsed_fields: Signal<Vec<(u16, String)>> = use_signal(Vec::new);
    let mut batch_reports: Signal<Vec<(usize, String, ValidationReport)>> = use_signal(Vec::new);
    let mut batch_ran       = use_signal(|| false);
    let messages            = props.messages;

    // ── Validate single message ───────────────────────────────────────────────
    let validate_single = move |_| {
        let raw = raw_input.read().clone();
        if raw.trim().is_empty() {
            report.set(None);
            parsed_fields.set(vec![]);
            return;
        }
        let bytes: Vec<u8> = raw.bytes().collect();
        let r = validate_raw(&bytes);
        // Extract parsed fields for display
        let msg = parse_single_for_validation(&bytes);
        let fields: Vec<(u16, String)> = msg.fields
            .iter()
            .map(|f| (f.tag, f.value.to_string()))
            .collect();
        parsed_fields.set(fields);
        report.set(Some(r));
    };

    // ── Validate batch ────────────────────────────────────────────────────────
    let validate_all = move |_| {
        let msgs = messages.read().clone();
        if msgs.is_empty() {
            batch_reports.set(vec![]);
            batch_ran.set(true);
            return;
        }
        let reports = validate_batch(&msgs);
        let with_issues: Vec<(usize, String, ValidationReport)> = reports
            .into_iter()
            .enumerate()
            .filter(|(_, r)| !r.is_clean() || r.warning_count() > 0)
            .map(|(i, r)| {
                let mt = msgs[i].msg_type_raw.to_string();
                (i, mt, r)
            })
            .collect();
        batch_reports.set(with_issues);
        batch_ran.set(true);
    };

    // ── Drill: load a message into single-debugger ────────────────────────────
    let mut drill_msg = move |idx: usize| {
        let msgs = messages.read();
        if let Some(msg) = msgs.get(idx) {
            // Reconstruct pipe-delimited raw from parsed fields
            let raw: String = msg.fields
                .iter()
                .map(|f| format!("{}={}|", f.tag, f.value))
                .collect();
            drop(msgs);
            raw_input.set(raw.clone());
            let bytes: Vec<u8> = raw.bytes().collect();
            let r = validate_raw(&bytes);
            let fields: Vec<(u16, String)> = parse_single_for_validation(&bytes)
                .fields
                .iter()
                .map(|f| (f.tag, f.value.to_string()))
                .collect();
            parsed_fields.set(fields);
            report.set(Some(r));
            tab.set(Tab::Single);
        }
    };

    let msg_count = messages.read().len();
    let in_single = *tab.read() == Tab::Single;
    let in_batch  = *tab.read() == Tab::Batch;

    rsx! {
        div { class: "validator-panel",

            // ── Tab bar ──
            div { class: "validator-tabs",
                button {
                    class: if in_single { "panel-tab panel-tab-active" } else { "panel-tab" },
                    onclick: move |_| tab.set(Tab::Single),
                    "Message Debugger"
                }
                button {
                    class: if in_batch { "panel-tab panel-tab-active" } else { "panel-tab" },
                    onclick: move |_| tab.set(Tab::Batch),
                    "Batch Validate"
                    if msg_count > 0 {
                        span { class: "validator-msg-count", " ({msg_count})" }
                    }
                }
            }

            if in_single {
                // ── Single message debugger ──
                div { class: "validator-single",

                    div { class: "validator-input-row",
                        textarea {
                            class: "validator-input",
                            placeholder: "Paste a single FIX message here (pipe or SOH delimited)…\n\nExample:\n8=FIX.4.4|9=178|35=D|34=1|49=CITIFX|52=20240101-12:00:00|56=FXECN|11=ORD001|21=1|38=1000000|40=2|44=1.0850|54=1|55=EURUSD|60=20240101-12:00:00|10=123|",
                            value: "{raw_input.read()}",
                            oninput: move |e| raw_input.set(e.value()),
                        }
                        button {
                            class: "btn btn-process validator-validate-btn",
                            onclick: validate_single,
                            "Validate"
                        }
                    }

                    if let Some(rep) = report.read().clone() {
                        // ── Summary bar ──
                        {
                            let errs = rep.error_count();
                            let warns = rep.warning_count();
                            let err_label  = format!("✗ {} error{}",  errs,  if errs  == 1 { "" } else { "s" });
                            let warn_label = format!("⚠ {} warning{}", warns, if warns == 1 { "" } else { "s" });
                            rsx! {
                                div { class: "validator-summary",
                                    if errs == 0 && warns == 0 {
                                        span { class: "vsummary-ok", "✓ Valid — no issues found" }
                                    } else {
                                        if errs > 0 {
                                            span { class: "vsummary-err", "{err_label}" }
                                        }
                                        if warns > 0 {
                                            span { class: "vsummary-warn", "{warn_label}" }
                                        }
                                    }
                                    if let (Some(ok), Some(found), Some(exp)) =
                                        (rep.checksum_ok, rep.checksum_found.clone(), rep.checksum_expected.clone())
                                    {
                                        {
                                            let chk_text = if ok {
                                                format!("Checksum: {} ✓", found)
                                            } else {
                                                format!("Checksum: {} ✗ (expected {})", found, exp)
                                            };
                                            rsx! {
                                                span {
                                                    class: if ok { "vsummary-chk-ok" } else { "vsummary-chk-err" },
                                                    "{chk_text}"
                                                }
                                            }
                                        }
                                    }
                                    if let (Some(ok), Some(found), Some(counted)) =
                                        (rep.body_length_ok, rep.body_length_found, rep.body_length_counted)
                                    {
                                        {
                                            let bl_text = if ok {
                                                format!("BodyLength: {} ✓", found)
                                            } else {
                                                format!("BodyLength: {} ✗ (counted {})", found, counted)
                                            };
                                            rsx! {
                                                span {
                                                    class: if ok { "vsummary-chk-ok" } else { "vsummary-chk-err" },
                                                    "{bl_text}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // ── Field table ──
                        {
                            let fields = parsed_fields.read().clone();
                            let issues = rep.issues.clone();
                            rsx! {
                                div { class: "validator-field-table",
                                    // Header
                                    div { class: "vfield-header vfield-row",
                                        span { class: "vfield-tag", "Tag" }
                                        span { class: "vfield-name", "Name" }
                                        span { class: "vfield-value", "Value" }
                                        span { class: "vfield-status", "Status" }
                                    }
                                    // Rows
                                    for (tag, value) in fields.iter() {
                                        {
                                            let t = *tag;
                                            let v = value.clone();
                                            let field_issues: Vec<&Issue> = issues.iter()
                                                .filter(|i| i.tag == Some(t))
                                                .collect();
                                            let row_class = if field_issues.iter().any(|i| i.severity == Severity::Error) {
                                                "vfield-row vfield-error"
                                            } else if field_issues.iter().any(|i| i.severity == Severity::Warning) {
                                                "vfield-row vfield-warn"
                                            } else {
                                                "vfield-row vfield-ok"
                                            };
                                            rsx! {
                                                div { class: row_class,
                                                    span { class: "vfield-tag vfield-tag-num", "{t}" }
                                                    span { class: "vfield-name", "{tag_description(t)}" }
                                                    span { class: "vfield-value", "{v}" }
                                                    span { class: "vfield-status",
                                                        if field_issues.is_empty() {
                                                            span { class: "vstatus-ok", "✓" }
                                                        } else {
                                                            for issue in &field_issues {
                                                                span {
                                                                    class: if issue.severity == Severity::Error { "vstatus-err" } else { "vstatus-warn" },
                                                                    title: "{issue.message}",
                                                                    if issue.severity == Severity::Error { "✗" } else { "⚠" }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                // Inline issue messages under the field row
                                                for issue in field_issues {
                                                    div { class: "vfield-issue",
                                                        span {
                                                            class: if issue.severity == Severity::Error { "vissue-rule-err" } else { "vissue-rule-warn" },
                                                            "{issue.rule_label()}"
                                                        }
                                                        span {
                                                            class: if issue.severity == Severity::Error { "vissue-err" } else { "vissue-warn" },
                                                            "{issue.message}"
                                                        }
                                                        if let Some(hint) = &issue.fix_hint {
                                                            span { class: "vissue-hint", "→ {hint}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // ── Structural issues (no specific tag) ──
                        {
                            let structural: Vec<Issue> = rep.issues.iter()
                                .filter(|i| i.tag.is_none())
                                .cloned()
                                .collect();
                            if !structural.is_empty() {
                                rsx! {
                                    div { class: "validator-structural",
                                        for issue in &structural {
                                            div { class: "vfield-issue",
                                                span {
                                                    class: if issue.severity == Severity::Error { "vissue-rule-err" } else { "vissue-rule-warn" },
                                                    "{issue.rule_label()}"
                                                }
                                                span {
                                                    class: if issue.severity == Severity::Error { "vissue-err" } else { "vissue-warn" },
                                                    "{issue.message}"
                                                }
                                                if let Some(hint) = &issue.fix_hint {
                                                    span { class: "vissue-hint", "→ {hint}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                rsx! {}
                            }
                        }
                    }
                }
            }

            if in_batch {
                // ── Batch validation ──
                div { class: "validator-batch",
                    div { class: "validator-batch-toolbar",
                        button {
                            class: "btn btn-process",
                            onclick: validate_all,
                            disabled: msg_count == 0,
                            if msg_count == 0 {
                                "No messages loaded"
                            } else {
                                "Validate {msg_count} messages"
                            }
                        }
                        if *batch_ran.read() {
                            {
                                let rows = batch_reports.read();
                                let err_msgs  = rows.iter().filter(|(_, _, r)| r.error_count() > 0).count();
                                let warn_msgs = rows.iter().filter(|(_, _, r)| r.warning_count() > 0 && r.error_count() == 0).count();
                                let err_label = format!("{} message{} with errors", err_msgs, if err_msgs == 1 { "" } else { "s" });
                                rsx! {
                                    if err_msgs == 0 && warn_msgs == 0 {
                                        span { class: "vsummary-ok", "✓ All {msg_count} messages valid" }
                                    } else {
                                        span { class: "vsummary-err", "{err_label}" }
                                        span { class: "vsummary-warn", "{warn_msgs} with warnings only" }
                                    }
                                }
                            }
                        }
                    }

                    if *batch_ran.read() {
                        {
                            let rows = batch_reports.read().clone();
                            if rows.is_empty() {
                                rsx! {
                                    div { class: "validator-batch-empty",
                                        "All messages passed validation — no issues found."
                                    }
                                }
                            } else {
                                rsx! {
                                    div { class: "validator-batch-table",
                                        // Header
                                        div { class: "vbatch-header vbatch-row",
                                            span { class: "vbatch-idx", "#" }
                                            span { class: "vbatch-type", "MsgType" }
                                            span { class: "vbatch-issues", "Issues" }
                                            span { class: "vbatch-first", "First error" }
                                        }
                                        for (idx, mt, rep) in rows.iter() {
                                            {
                                                let i = *idx;
                                                let mt = mt.clone();
                                                let errs = rep.error_count();
                                                let warns = rep.warning_count();
                                                let first = rep.first_error()
                                                    .map(|e| e.message.clone())
                                                    .or_else(|| rep.issues.first().map(|e| e.message.clone()))
                                                    .unwrap_or_default();
                                                rsx! {
                                                    div {
                                                        class: if errs > 0 { "vbatch-row vbatch-error" } else { "vbatch-row vbatch-warn" },
                                                        onclick: move |_| drill_msg(i),
                                                        title: "Click to inspect in Message Debugger",
                                                        span { class: "vbatch-idx", "{i + 1}" }
                                                        span { class: "vbatch-type", "{mt}" }
                                                        span { class: "vbatch-issues",
                                                            if errs > 0 {
                                                                span { class: "vstatus-err", "✗ {errs}" }
                                                            }
                                                            if warns > 0 {
                                                                span { class: "vstatus-warn", " ⚠ {warns}" }
                                                            }
                                                        }
                                                        span { class: "vbatch-first", "{first}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
