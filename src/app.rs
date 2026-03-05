use std::time::Instant;
use std::mem;

use dioxus::prelude::*;
use dioxus::document::eval;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const VERSION_URL: &str = "https://aifixparser.com/latest-version";
const DOWNLOAD_URL: &str = "https://aifixparser.com/#download";
const GA_ID: &str = "G-Y9J423BNZ0"; // ← replace with your GA4 Measurement ID

#[derive(Clone, PartialEq)]
enum UpdateStatus {
    Idle,
    Checking,
    Available(String), // latest version string
    UpToDate,
}

/// Returns true when `latest` is a higher semver than `current`.
fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse = |s: &str| -> [u32; 3] {
        let mut it = s.split('.').filter_map(|p| p.parse().ok());
        [it.next().unwrap_or(0), it.next().unwrap_or(0), it.next().unwrap_or(0)]
    };
    parse(latest) > parse(current)
}

use crate::components::detail::detail_panel;
use crate::components::timeline::timeline_panel;
use crate::model::FixMessage;
use crate::parser::{parse_all, parse_all_simd, parse_all_simd_bytes};
use crate::sample::{sample_data, FIX_SPECS};
use crate::style::CSS;

/// Replace `signal`'s value with `new_data`, then drop the old `Vec<FixMessage>`
/// on a background thread so the main UI thread isn't blocked by deallocation
/// of potentially millions of structs.
fn offload_replace(signal: &mut Signal<Vec<FixMessage>>, new_data: Vec<FixMessage>) {
    let old = mem::replace(&mut *signal.write(), new_data);
    if !old.is_empty() {
        std::thread::spawn(move || drop(old));
    }
}

/// Root application component.
pub fn app() -> Element {
    let mut input = use_signal(String::new);
    let mut messages: Signal<Vec<FixMessage>> = use_signal(Vec::new);
    let mut selected_idx: Signal<Option<usize>> = use_signal(|| None);
    let skip_heartbeats = use_signal(|| true);
    let skip_common = use_signal(|| false);
    let mut parse_stats: Signal<Option<(usize, u64)>> = use_signal(|| None);
    let loading = use_signal(|| false);
    // Tracks the last file loaded via the file dialog (name only — not the content).
    let mut file_name: Signal<Option<String>> = use_signal(|| None);
    let mut update_status: Signal<UpdateStatus> = use_signal(|| UpdateStatus::Idle);

    let mut process = move || {
        let t = Instant::now();
        let parsed = parse_all(&input.read());
        let ms = t.elapsed().as_micros() as u64;
        parse_stats.set(Some((parsed.len(), ms)));
        offload_replace(&mut messages, parsed);
        selected_idx.set(None);
        file_name.set(None);
    };

    let mut clear = move || {
        input.set(String::new());
        offload_replace(&mut messages, Vec::new());
        selected_idx.set(None);
        parse_stats.set(None);
        file_name.set(None);
    };

    let mut load_sample = move |spec: &str| {
        let s = sample_data(spec);
        let t = Instant::now();
        let parsed = parse_all(&s);
        let ms = t.elapsed().as_micros() as u64;
        parse_stats.set(Some((parsed.len(), ms)));
        let count = parsed.len();
        input.set(s);
        offload_replace(&mut messages, parsed);
        selected_idx.set(None);
        file_name.set(None);
        eval(&format!(
            "window.gtag && window.gtag('event', 'sample_loaded', \
             {{ sample: '{spec}', message_count: {count}, parse_us: {ms} }});"
        ));
    };

    let load_file = move || {
        let mut messages    = messages.clone();
        let mut selected_idx = selected_idx.clone();
        let mut parse_stats = parse_stats.clone();
        let mut loading     = loading.clone();
        let mut file_name   = file_name.clone();
        spawn(async move {
            loading.set(true);
            if let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("FIX log", &["txt", "log", "fix"])
                .add_filter("All files", &["*"])
                .pick_file()
                .await
            {
                let name = file.file_name();
                // Clone the path before any await so `file` stays available as a fallback.
                let path = file.path().to_owned();

                let t = Instant::now();

                // Desktop: mmap the file (zero copy — OS page cache, no Vec<u8> alloc).
                // Falls back to file.read().await if mmap fails (network drive, etc.).
                let (parsed, is_soh) = match std::fs::File::open(&path)
                    .and_then(|f| unsafe { memmap2::Mmap::map(&f) })
                {
                    Ok(mmap) => {
                        let soh = mmap.iter().take(4096).any(|&b| b == 0x01);
                        let msgs = if soh {
                            // Bytes path: inlined AVX2, no normalize alloc, no Vec<u8> copy.
                            parse_all_simd_bytes(&mmap)
                        } else {
                            // Pipe path: Cow::Borrowed fast path (zero extra alloc).
                            let s = String::from_utf8_lossy(&mmap);
                            parse_all(&s)
                        };
                        (msgs, soh)
                    }
                    Err(_) => {
                        // mmap unavailable (network drive, etc.) — read into Vec<u8>.
                        let bytes = file.read().await;
                        let soh = bytes.iter().take(4096).any(|&b| b == 0x01);
                        let s = String::from_utf8_lossy(&bytes);
                        let msgs = if soh { parse_all_simd(&s) } else { parse_all(&s) };
                        (msgs, soh)
                    }
                };

                let ms = t.elapsed().as_micros() as u64;
                let count = parsed.len();
                parse_stats.set(Some((count, ms)));
                offload_replace(&mut messages, parsed);
                selected_idx.set(None);
                file_name.set(Some(name));
                let delimiter = if is_soh { "soh" } else { "pipe" };
                eval(&format!(
                    "window.gtag && window.gtag('event', 'file_parsed', \
                     {{ message_count: {count}, parse_us: {ms}, delimiter: '{delimiter}' }});"
                ));
            }
            loading.set(false);
        });
    };

    // Selected message for the detail panel
    let sel = *selected_idx.read();
    let detail_msg: Option<FixMessage> = sel.and_then(|i| messages.read().get(i).cloned());

    // Show hero landing when nothing is loaded yet.
    let show_hero = messages.read().is_empty() && file_name.read().is_none() && !*loading.read();

    // Inject GA4 once on mount and fire app_open event.
    use_effect(move || {
        let os = std::env::consts::OS;
        eval(&format!(
            r#"(function(id) {{
                if (window._ga_inited) return;
                window._ga_inited = true;
                var s = document.createElement('script');
                s.async = true;
                s.src = 'https://www.googletagmanager.com/gtag/js?id=' + id;
                document.head.appendChild(s);
                s.onload = function() {{
                    window.dataLayer = window.dataLayer || [];
                    function gtag(){{ window.dataLayer.push(arguments); }}
                    window.gtag = gtag;
                    gtag('js', new Date());
                    gtag('config', id, {{ send_page_view: false }});
                    var _ga_start = Date.now();
                    gtag('event', 'app_open', {{ app_version: '{CURRENT_VERSION}', platform: '{os}' }});
                    window.addEventListener('beforeunload', function() {{
                        var sec = Math.round((Date.now() - _ga_start) / 1000);
                        gtag('event', 'session_end', {{
                            session_duration_sec: sec,
                            app_version: '{CURRENT_VERSION}',
                            platform: '{os}',
                            transport_type: 'beacon'
                        }});
                    }});
                }};
            }})('{GA_ID}');"#
        ));
    });

    // Auto-check for updates once on mount (no reactive reads → runs exactly once).
    use_effect(move || {
        update_status.set(UpdateStatus::Checking);
        spawn(async move {
            let mut ev = eval(&format!(
                r#"(async () => {{
                    try {{
                        const r = await fetch('{VERSION_URL}',
                            {{ signal: AbortSignal.timeout(6000) }});
                        window.dioxus.send((await r.text()).trim());
                    }} catch(e) {{
                        window.dioxus.send('');
                    }}
                }})();"#
            ));
            update_status.set(match ev.recv::<String>().await {
                Ok(v) if !v.is_empty() && is_newer_version(&v, CURRENT_VERSION) => {
                    UpdateStatus::Available(v)
                }
                Ok(v) if !v.is_empty() => UpdateStatus::UpToDate,
                _ => UpdateStatus::Idle,
            });
        });
    });

    // Re-run the JS counter animation every time the hero becomes visible
    // (on mount and whenever the user clears back to empty state).
    use_effect(move || {
        let visible = messages.read().is_empty() && file_name.read().is_none() && !*loading.read();
        if visible {
            eval(r#"
                (function() {
                    function animCounter(id, target, dur) {
                        var el = document.getElementById(id);
                        if (!el) return;
                        var t0 = performance.now();
                        (function tick() {
                            var p = Math.min((performance.now() - t0) / dur, 1);
                            var v = Math.round((1 - Math.pow(1 - p, 3)) * target);
                            el.textContent = (p >= 1 ? target : v).toLocaleString('en-US');
                            if (p < 1) requestAnimationFrame(tick);
                        })();
                    }
                    setTimeout(function() {
                        animCounter('hero-msg-count',  1000000, 1600);
                        animCounter('hero-parse-ms',   150,     1200);
                        animCounter('hero-throughput', 631,     1000);
                    }, 150);
                })();
            "#);
        }
    });

    rsx! {
        style { {CSS} }
        div { class: "root",
            // ── Toolbar ──
            div { class: "toolbar",
                // "Process" only makes sense when the user has pasted text manually.
                if file_name.read().is_none() {
                    button { class: "btn btn-process", onclick: move |_| process(), "Process" }
                }
                button { class: "btn btn-clear", onclick: move |_| clear(), "Clear" }
                button { class: "btn btn-load", onclick: move |_| load_file(), "Load file" }
                span { class: "sample-label", "Sample: " }
                {FIX_SPECS.iter().enumerate().map(|(i, spec)| {
                    let sep = if i > 0 { rsx! { span { class: "sample-sep", " | " } } } else { rsx! { } };
                    rsx! {
                        {sep}
                        button {
                            class: "btn btn-sample-inline",
                            onclick: move |_| load_sample(spec),
                            "{spec}"
                        }
                    }
                })}

                // Push update button to the right
                div { class: "toolbar-spacer" }

                // Update indicator / button
                {
                    let status = update_status.read().clone();
                    match status {
                        UpdateStatus::Checking => rsx! {
                            span { class: "update-checking", "Checking for updates…" }
                        },
                        UpdateStatus::Available(v) => rsx! {
                            button {
                                class: "btn btn-update-available",
                                onclick: move |_| {
                                    let url = DOWNLOAD_URL.to_string();
                                    std::thread::spawn(move || { let _ = open::that(url); });
                                },
                                "⬆ v{v} — Update now"
                            }
                        },
                        UpdateStatus::UpToDate => rsx! {
                            span { class: "update-ok", "✓ v{CURRENT_VERSION}" }
                        },
                        UpdateStatus::Idle => rsx! {
                            span { class: "update-version", "v{CURRENT_VERSION}" }
                        },
                    }
                }
            }

            if show_hero {
                // ── Hero landing (replaces input + panels when nothing is loaded) ──
                div { class: "hero",
                    div { class: "hero-title",
                        span { class: "hero-icon", "⚡" }
                        h1 { "AiFIXParser.com" }
                        p { "High-performance FIX protocol parser & inspector" }
                    }
                    div { class: "hero-stats",
                        div { class: "hero-stat hero-stat-green",
                            div { class: "hero-stat-value", id: "hero-msg-count", "0" }
                            div { class: "hero-stat-unit", "messages" }
                            div { class: "hero-stat-label", "parsed in one shot" }
                        }
                        div { class: "hero-stat hero-stat-purple hero-stat-featured",
                            div { class: "hero-stat-value",
                                span { id: "hero-parse-ms", "0" }
                                span { class: "hero-stat-suffix", "ms" }
                            }
                            div { class: "hero-stat-unit", "parse time" }
                            div { class: "hero-stat-label", "for 1,000,000 messages" }
                        }
                        div { class: "hero-stat hero-stat-cyan",
                            div { class: "hero-stat-value",
                                span { id: "hero-throughput", "0" }
                                span { class: "hero-stat-suffix", " MiB/s" }
                            }
                            div { class: "hero-stat-unit", "throughput" }
                            div { class: "hero-stat-label", "streaming parse" }
                        }
                    }
                    div { class: "hero-demo",
                        div { class: "hero-demo-label",
                            span { "Simulating 1,000,000 FIX message parse…" }
                            span { class: "hero-demo-time", "150 ms" }
                        }
                        div { class: "hero-bar-track",
                            div { class: "hero-bar-fill" }
                        }
                    }
                    p { class: "hero-hint",
                        "Paste FIX data and click "
                        span { class: "hero-hint-kbd", "Process" }
                        "  ·  "
                        span { class: "hero-hint-kbd", "Load file" }
                        "  ·  or pick a sample from the toolbar"
                    }
                }
            } else {
                // ── Input area ──
                if *loading.read() {
                    div { class: "fix-loading", "Loading file and parsing messages…" }
                } else if let Some(ref name) = *file_name.read() {
                    // File was loaded via dialog — don't put 135 MB in the textarea.
                    div { class: "fix-file-banner",
                        span { class: "fix-file-icon", "📂" }
                        span { class: "fix-file-name", "{name}" }
                        span { class: "fix-file-hint", "— click Clear to reset" }
                    }
                } else {
                    textarea {
                        class: "fix-input",
                        placeholder: "Paste FIX messages here …",
                        value: "{input.read()}",
                        oninput: move |evt| input.set(evt.value()),
                    }
                }

                // ── Main panels ──
                div { class: "panels",
                    timeline_panel {
                        messages: messages,
                        selected_idx: selected_idx,
                        skip_heartbeats: skip_heartbeats,
                        parse_stats: parse_stats,
                    }
                    detail_panel {
                        detail_msg: detail_msg,
                        skip_common: skip_common,
                    }
                }
            }
        }
    }
}
