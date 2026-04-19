//! FIX Message Validator — single-message debugger + auto batch summary.

use dioxus::prelude::*;

use crate::dictionary::tag_description;
use crate::export::{csv_escape, now_tag};
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
    let mut tab          = use_signal(|| Tab::Batch);
    let mut raw_input    = use_signal(String::new);
    let mut report: Signal<Option<ValidationReport>> = use_signal(|| None);
    let mut parsed_fields: Signal<Vec<(u16, String)>> = use_signal(Vec::new);

    // Batch state — populated automatically via use_effect
    let mut batch_reports: Signal<Vec<(usize, String, ValidationReport)>> = use_signal(Vec::new);
    let mut batch_total   = use_signal(|| 0usize);
    let mut validating    = use_signal(|| false);
    let mut filter_text   = use_signal(String::new);

    let messages = props.messages;

    // ── Auto-validate whenever messages change ────────────────────────────────
    use_effect(move || {
        let msgs = messages.read().clone();
        let count = msgs.len();
        batch_total.set(count);
        batch_reports.set(vec![]);

        if count == 0 {
            validating.set(false);
            return;
        }

        validating.set(true);
        let (tx, rx) = tokio::sync::oneshot::channel::<Vec<(usize, String, ValidationReport)>>();

        rayon::spawn(move || {
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
            let _ = tx.send(with_issues);
        });

        spawn(async move {
            if let Ok(data) = rx.await {
                batch_reports.set(data);
                validating.set(false);
            }
        });
    });

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
        let msg = parse_single_for_validation(&bytes);
        let fields: Vec<(u16, String)> = msg.fields
            .iter()
            .map(|f| (f.tag, f.value_in(&msg.arena).to_string()))
            .collect();
        parsed_fields.set(fields);
        report.set(Some(r));
    };

    // ── Drill: load a batch message into the single-debugger ─────────────────
    let mut drill_msg = move |idx: usize| {
        let msgs = messages.read();
        if let Some(msg) = msgs.get(idx) {
            let raw: String = msg.fields
                .iter()
                .map(|f| format!("{}={}|", f.tag, f.value_in(&msg.arena)))
                .collect();
            drop(msgs);
            let bytes: Vec<u8> = raw.bytes().collect();
            raw_input.set(raw);
            let r = validate_raw(&bytes);
            let reparsed = parse_single_for_validation(&bytes);
            let fields: Vec<(u16, String)> = reparsed.fields
                .iter()
                .map(|f| (f.tag, f.value_in(&reparsed.arena).to_string()))
                .collect();
            parsed_fields.set(fields);
            report.set(Some(r));
            tab.set(Tab::Single);
        }
    };

    let msg_count    = messages.read().len();
    let in_single    = *tab.read() == Tab::Single;
    let in_batch     = *tab.read() == Tab::Batch;
    let is_validating = *validating.read();

    rsx! {
        div { class: "validator-panel",

            // ── Header ──
            div { class: "panel-header",
                div { class: "panel-title",
                    if msg_count > 0 {
                        span { class: "parse-stats", "{msg_count} messages loaded" }
                    } else {
                        span { class: "parse-stats", "No messages loaded — paste a message in the debugger below" }
                    }
                }
            }

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
                                    div { class: "vfield-header vfield-row",
                                        span { class: "vfield-tag", "Tag" }
                                        span { class: "vfield-name", "Name" }
                                        span { class: "vfield-value", "Value" }
                                        span { class: "vfield-status", "Status" }
                                    }
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

                    // ── Summary report (always visible) ──
                    {
                        let rows       = batch_reports.read();
                        let total      = *batch_total.read();
                        let err_msgs   = rows.iter().filter(|(_, _, r)| r.error_count() > 0).count();
                        let warn_msgs  = rows.iter().filter(|(_, _, r)| r.warning_count() > 0 && r.error_count() == 0).count();
                        let valid_msgs = total.saturating_sub(err_msgs + warn_msgs);
                        let err_label    = format!("{} error{}",   err_msgs,  if err_msgs  == 1 { "" } else { "s" });
                        let warn_label   = format!("{} warning{}", warn_msgs, if warn_msgs == 1 { "" } else { "s" });
                        let err_in_msgs  = format!("in {} msg{}",  err_msgs,  if err_msgs  == 1 { "" } else { "s" });
                        let warn_in_msgs = format!("in {} msg{}",  warn_msgs, if warn_msgs == 1 { "" } else { "s" });
                        rsx! {
                            div { class: "vbatch-summary",
                                if is_validating {
                                    span { class: "vbatch-summary-running", "Validating…" }
                                } else if total == 0 {
                                    span { class: "vbatch-summary-empty", "No messages loaded" }
                                } else if err_msgs == 0 && warn_msgs == 0 {
                                    span { class: "vbatch-summary-stat vbatch-stat-ok",
                                        span { class: "vbatch-stat-value", "✓ {total}" }
                                        span { class: "vbatch-stat-label", "all valid" }
                                    }
                                } else {
                                    span { class: "vbatch-summary-stat vbatch-stat-ok",
                                        span { class: "vbatch-stat-value", "{valid_msgs}" }
                                        span { class: "vbatch-stat-label", "valid" }
                                    }
                                    span { class: "vbatch-summary-stat vbatch-stat-err",
                                        span { class: "vbatch-stat-value", "{err_label}" }
                                        span { class: "vbatch-stat-label", "{err_in_msgs}" }
                                    }
                                    if warn_msgs > 0 {
                                        span { class: "vbatch-summary-stat vbatch-stat-warn",
                                            span { class: "vbatch-stat-value", "{warn_label}" }
                                            span { class: "vbatch-stat-label", "{warn_in_msgs}" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ── Error code breakdown ──
                    {
                        let rows = batch_reports.read();
                        if !rows.is_empty() {
                            // Count messages affected per error code
                            let mut map: std::collections::HashMap<&'static str, (usize, Severity)> =
                                std::collections::HashMap::new();
                            for (_, _, rep) in rows.iter() {
                                let mut seen: std::collections::HashSet<&'static str> =
                                    std::collections::HashSet::new();
                                for issue in &rep.issues {
                                    if seen.insert(issue.code) {
                                        let e = map.entry(issue.code).or_insert((0, Severity::Warning));
                                        e.0 += 1;
                                        if issue.severity == Severity::Error {
                                            e.1 = Severity::Error;
                                        }
                                    }
                                }
                            }
                            // Sort: errors first, then by count descending
                            let mut breakdown: Vec<(&'static str, usize, Severity)> = map
                                .into_iter()
                                .map(|(code, (count, sev))| (code, count, sev))
                                .collect();
                            breakdown.sort_by(|a, b| {
                                let sev_ord = |s: &Severity| if *s == Severity::Error { 0 } else { 1 };
                                sev_ord(&a.2).cmp(&sev_ord(&b.2))
                                    .then(b.1.cmp(&a.1))
                            });
                            rsx! {
                                div { class: "vbatch-breakdown",
                                    div { class: "vbatch-breakdown-header",
                                        span { class: "vbd-rule", "Rule" }
                                        span { class: "vbd-code", "Code" }
                                        span { class: "vbd-count", "Msgs" }
                                    }
                                    for (code, count, sev) in breakdown.iter() {
                                        {
                                            let rn = crate::validator::rule_number(code);
                                            let rule_lbl = if *sev == Severity::Error {
                                                format!("Error Rule {rn}")
                                            } else {
                                                format!("Warning Rule {rn}")
                                            };
                                            rsx! {
                                                div { class: "vbd-row",
                                                    span {
                                                        class: if *sev == Severity::Error { "vbd-rule vissue-rule-err" } else { "vbd-rule vissue-rule-warn" },
                                                        "{rule_lbl}"
                                                    }
                                                    span { class: "vbd-code", "{code}" }
                                                    span { class: "vbd-count", "{count}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            rsx! {}
                        }
                    }

                    // ── Issues table ──
                    {
                        let all_rows = batch_reports.read().clone();
                        if !all_rows.is_empty() {
                            // Build filtered rows — each entry carries its first-error string.
                            let needle = filter_text.read().to_lowercase();
                            let filtered: Vec<(usize, String, usize, usize, String)> = all_rows.iter()
                                .map(|(idx, mt, rep)| {
                                    let first = rep.first_error()
                                        .map(|e| e.message.clone())
                                        .or_else(|| rep.issues.first().map(|e| e.message.clone()))
                                        .unwrap_or_default();
                                    (*idx, mt.clone(), rep.error_count(), rep.warning_count(), first)
                                })
                                .filter(|(_, _, _, _, first)| {
                                    needle.is_empty() || first.to_lowercase().contains(&needle)
                                })
                                .collect();

                            let snapshot = all_rows.clone(); // for export closure
                            rsx! {
                                // ── Filter bar + export ──────────────────────
                                div { class: "vbatch-toolbar",
                                    div { class: "vbatch-filter-wrap",
                                        input {
                                            class: "vbatch-filter",
                                            r#type: "text",
                                            placeholder: "Filter by first error…",
                                            value: "{filter_text.read()}",
                                            oninput: move |e| filter_text.set(e.value()),
                                        }
                                        if !filter_text.read().is_empty() {
                                            button {
                                                class: "vbatch-filter-clear",
                                                onclick: move |_| filter_text.set(String::new()),
                                                "×"
                                            }
                                        }
                                    }
                                    {
                                        let count_label = if needle.is_empty() {
                                            format!("{} issues", filtered.len())
                                        } else {
                                            format!("{} / {} match", filtered.len(), all_rows.len())
                                        };
                                        rsx! {
                                            span { class: "vbatch-filter-count", "{count_label}" }
                                        }
                                    }
                                    button {
                                        class: "btn-export-csv",
                                        onclick: move |_| {
                                            let rows_snap = snapshot.clone();
                                            spawn(async move {
                                                let tag = now_tag();
                                                if let Some(file) = rfd::AsyncFileDialog::new()
                                                    .set_file_name(&format!("fix_validation_{tag}.csv"))
                                                    .add_filter("CSV", &["csv"])
                                                    .save_file()
                                                    .await
                                                {
                                                    let csv = build_issues_csv(&rows_snap);
                                                    let _ = std::fs::write(file.path(), csv.as_bytes());
                                                }
                                            });
                                        },
                                        "Export CSV"
                                    }
                                }

                                // ── Table ────────────────────────────────────
                                div { class: "validator-batch-table",
                                    div { class: "vbatch-header vbatch-row",
                                        span { class: "vbatch-idx", "#" }
                                        span { class: "vbatch-type", "MsgType" }
                                        span { class: "vbatch-issues", "Issues" }
                                        span { class: "vbatch-first", "First error" }
                                    }
                                    for (i, mt, errs, warns, first) in filtered.iter() {
                                        {
                                            let i = *i;
                                            let mt = mt.clone();
                                            let errs = *errs;
                                            let warns = *warns;
                                            let first = first.clone();
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
                        } else if !is_validating && *batch_total.read() > 0 {
                            rsx! {
                                div { class: "validator-batch-empty",
                                    "✓ All messages passed validation — no issues found."
                                }
                            }
                        } else {
                            rsx! {}
                        }
                    }
                }
            }
        }
    }
}

// ── CSV export ────────────────────────────────────────────────────────────────

fn build_issues_csv(rows: &[(usize, String, ValidationReport)]) -> String {
    let mut out = String::from("#,MsgType,Errors,Warnings,FirstError,AllIssues\n");
    for (idx, mt, rep) in rows {
        let first = rep.first_error()
            .map(|e| e.message.as_str())
            .or_else(|| rep.issues.first().map(|e| e.message.as_str()))
            .unwrap_or("");
        let all: Vec<String> = rep.issues.iter()
            .map(|i| format!("[{}] {}", i.code, i.message))
            .collect();
        let all_str = all.join(" | ");
        out.push_str(&format!(
            "{},{},{},{},{},{}\n",
            idx + 1,
            csv_escape(mt),
            rep.error_count(),
            rep.warning_count(),
            csv_escape(first),
            csv_escape(&all_str),
        ));
    }
    out
}
