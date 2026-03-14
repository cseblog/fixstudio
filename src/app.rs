use std::time::Instant;
use std::mem;

use dioxus::prelude::*;
use dioxus::document::eval;

use crate::components::detail::detail_panel;
use crate::components::lifecycle::lifecycle_panel;
use crate::components::timeline::timeline_panel;
use crate::export::messages_to_csv;
use crate::license::{clear_license, instance_name, load_license, save_license, StoredLicense};
use crate::model::FixMessage;
use crate::parser::{parse_all, parse_all_simd, parse_all_simd_bytes};
use crate::sample::{sample_data, FIX_SPECS};
use crate::style::CSS;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const VERSION_URL: &str = "https://aifixparser.com/latest-version";
const DOWNLOAD_URL: &str = "https://aifixparser.com/#download";
const GA_ID: &str = "G-Y9J423BNZ0";

/// Replace with your real Lemon Squeezy checkout URL after creating your product.
const LS_CHECKOUT_URL: &str = "https://aifixparser.lemonsqueezy.com/buy/1380467";

/// Set to true during development to bypass all license checks (treat everyone as Pro).
/// REMEMBER: set this back to false before release!
const DEV_MODE: bool = true;

#[derive(Clone, PartialEq)]
enum UpdateStatus {
    Idle,
    Checking,
    Available(String),
    UpToDate,
}

#[derive(Clone, PartialEq)]
enum LicenseStatus {
    Idle,
    Checking,
    Success,
    Error(String),
}

#[derive(Clone, PartialEq)]
enum ViewMode {
    Timeline,
    Lifecycle,
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse = |s: &str| -> [u32; 3] {
        let mut it = s.split('.').filter_map(|p| p.parse().ok());
        [it.next().unwrap_or(0), it.next().unwrap_or(0), it.next().unwrap_or(0)]
    };
    parse(latest) > parse(current)
}

fn offload_replace(signal: &mut Signal<Vec<FixMessage>>, new_data: Vec<FixMessage>) {
    let old = mem::replace(&mut *signal.write(), new_data);
    if !old.is_empty() {
        std::thread::spawn(move || drop(old));
    }
}

/// Root application component.
pub fn app() -> Element {
    // ── Core state ──
    let mut input = use_signal(String::new);
    let mut messages: Signal<Vec<FixMessage>> = use_signal(Vec::new);
    let mut selected_idx: Signal<Option<usize>> = use_signal(|| None);
    let skip_heartbeats = use_signal(|| true);
    let skip_common = use_signal(|| false);
    let mut parse_stats: Signal<Option<(usize, u64)>> = use_signal(|| None);
    let loading = use_signal(|| false);
    let mut file_name: Signal<Option<String>> = use_signal(|| None);
    let mut update_status: Signal<UpdateStatus> = use_signal(|| UpdateStatus::Idle);

    // ── Premium / license state ──
    let mut is_pro = use_signal(|| DEV_MODE);
    let mut show_license_modal = use_signal(|| false);
    let mut license_key_input = use_signal(String::new);
    let mut license_status = use_signal(|| LicenseStatus::Idle);
    let mut view_mode = use_signal(|| ViewMode::Timeline);

    // ── Panel layout state ──
    // right_panel_width controls premium-panel; app-main is flex:1 filling the rest
    let right_panel_width: Signal<f64> = use_signal(|| 300.0_f64);
    let mut left_panel_collapsed  = use_signal(|| false);
    let mut right_panel_collapsed = use_signal(|| false);

    // Initialise right panel to half the window width via JS
    use_effect(move || {
        let mut rpw = right_panel_width;
        spawn(async move {
            if let Ok(v) = eval("window.innerWidth").await {
                if let Some(w) = v.as_f64() {
                    rpw.set((w / 2.0).max(200.0));
                }
            }
        });
    });

    // ── Persistent JS-native resize handler ──
    // Set up once on mount. Uses direct DOM manipulation for smooth 60fps dragging.
    // Checks e.target.closest('button') so collapse buttons are never treated as drag starts.
    use_effect(move || {
        let mut rpw = right_panel_width;
        spawn(async move {
            let mut ev = eval(r#"
(function() {
    if (window._rszCleanup) window._rszCleanup();
    var handle = document.querySelector('.resize-handle');
    if (!handle) return;

    function ondown(e) {
        // Let button clicks pass through untouched
        if (e.target.closest && e.target.closest('button')) return;
        if (e.target.tagName === 'BUTTON') return;

        var panel = document.getElementById('premium-panel-main');
        if (!panel) return;

        // Read current width from inline style; fall back to rendered width
        var sw = parseFloat(panel.style.width);
        if (!sw || sw < 10) {
            var styleAttr = panel.getAttribute('style') || '';
            if (styleAttr.indexOf('flex: 1') >= 0) return; // right fills screen (left collapsed)
            sw = panel.getBoundingClientRect().width;
        }
        if (!sw || sw < 10) return; // panel is collapsed/hidden

        e.preventDefault();
        document.body.style.userSelect = 'none';
        var sx = e.clientX, go = false;

        function onmove(e2) {
            var dx = sx - e2.clientX;
            if (!go) {
                if (Math.abs(dx) < 4) return;
                go = true;
                document.body.style.cursor = 'col-resize';
            }
            var nw = Math.max(200, Math.min(window.innerWidth - 50, sw + dx));
            panel.style.width = nw + 'px'; // direct DOM — no Rust roundtrip
        }

        function onup() {
            document.removeEventListener('mousemove', onmove);
            document.removeEventListener('mouseup', onup);
            document.body.style.cursor = '';
            document.body.style.userSelect = '';
            if (go) window.dioxus.send('w:' + parseFloat(panel.style.width));
        }

        document.addEventListener('mousemove', onmove);
        document.addEventListener('mouseup', onup);
    }

    handle.addEventListener('mousedown', ondown);
    window._rszCleanup = function() {
        handle.removeEventListener('mousedown', ondown);
        window._rszCleanup = null;
    };
})();
            "#);
            // Receive final width once per drag (not per mousemove)
            loop {
                match ev.recv::<String>().await {
                    Ok(s) if s.starts_with("w:") => {
                        if let Ok(w) = s[2..].parse::<f64>() {
                            rpw.set(w);
                        }
                        // keep looping — handler stays active for future drags
                    }
                    _ => break,
                }
            }
        });
    });

    // ── Actions ──
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
        let mut messages     = messages.clone();
        let mut selected_idx = selected_idx.clone();
        let mut parse_stats  = parse_stats.clone();
        let mut loading      = loading.clone();
        let mut file_name    = file_name.clone();
        spawn(async move {
            loading.set(true);
            if let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("FIX log", &["txt", "log", "fix"])
                .add_filter("All files", &["*"])
                .pick_file()
                .await
            {
                let name = file.file_name();
                let path = file.path().to_owned();
                let t = Instant::now();
                let (parsed, is_soh) = match std::fs::File::open(&path)
                    .and_then(|f| unsafe { memmap2::Mmap::map(&f) })
                {
                    Ok(mmap) => {
                        let soh = mmap.iter().take(4096).any(|&b| b == 0x01);
                        let msgs = if soh {
                            parse_all_simd_bytes(&mmap)
                        } else {
                            let s = String::from_utf8_lossy(&mmap);
                            parse_all(&s)
                        };
                        (msgs, soh)
                    }
                    Err(_) => {
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

    // Export CSV (premium feature)
    let export_csv = move || {
        let msgs = messages.read().clone();
        spawn(async move {
            if let Some(file) = rfd::AsyncFileDialog::new()
                .set_file_name("fix_messages.csv")
                .add_filter("CSV", &["csv"])
                .save_file()
                .await
            {
                let csv = messages_to_csv(&msgs);
                let _ = std::fs::write(file.path(), csv.as_bytes());
            }
        });
    };

    // Activate license key via Lemon Squeezy API
    let activate_license = move || {
        let key = license_key_input.read().trim().to_string();
        if key.is_empty() {
            return;
        }
        let name = instance_name();
        let mut is_pro = is_pro.clone();
        let mut license_status = license_status.clone();
        let mut show_license_modal = show_license_modal.clone();
        license_status.set(LicenseStatus::Checking);
        spawn(async move {
            let js = format!(
                r#"(async () => {{
                    try {{
                        const resp = await fetch(
                            'https://api.lemonsqueezy.com/v1/licenses/activate',
                            {{
                                method: 'POST',
                                headers: {{
                                    'Content-Type': 'application/x-www-form-urlencoded',
                                    'Accept': 'application/json'
                                }},
                                body: new URLSearchParams({{
                                    license_key: '{key}',
                                    instance_name: '{name}'
                                }}).toString(),
                                signal: AbortSignal.timeout(10000)
                            }}
                        );
                        const data = await resp.json();
                        if (data.activated) {{
                            window.dioxus.send('ok:' + data.instance.id);
                        }} else {{
                            window.dioxus.send('err:' + (data.error || 'Invalid license key'));
                        }}
                    }} catch(e) {{
                        window.dioxus.send('err:' + e.message);
                    }}
                }})();"#
            );
            let mut ev = eval(&js);
            match ev.recv::<String>().await {
                Ok(s) if s.starts_with("ok:") => {
                    let instance_id = s.trim_start_matches("ok:").to_string();
                    save_license(&StoredLicense {
                        key: key.clone(),
                        instance_id,
                    });
                    is_pro.set(true);
                    license_status.set(LicenseStatus::Success);
                    // Auto-close modal after brief success display
                    spawn(async move {
                        show_license_modal.set(false);
                    });
                }
                Ok(s) => {
                    let err = s.trim_start_matches("err:").to_string();
                    license_status.set(LicenseStatus::Error(err));
                }
                _ => {
                    license_status.set(LicenseStatus::Error("Network error".to_string()));
                }
            }
        });
    };

    // Deactivate / remove license
    let mut deactivate_license = move || {
        clear_license();
        is_pro.set(false);
        view_mode.set(ViewMode::Timeline);
        show_license_modal.set(false);
        license_key_input.set(String::new());
        license_status.set(LicenseStatus::Idle);
    };

    // ── On-mount effects ──

    // 1. Inject GA4 + fire app_open
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

    // 2. Validate stored license on startup
    use_effect(move || {
        if DEV_MODE { return; }
        if let Some(stored) = load_license() {
            let key = stored.key.clone();
            let instance_id = stored.instance_id.clone();
            let mut is_pro = is_pro.clone();
            spawn(async move {
                let js = format!(
                    r#"(async () => {{
                        try {{
                            const resp = await fetch(
                                'https://api.lemonsqueezy.com/v1/licenses/validate',
                                {{
                                    method: 'POST',
                                    headers: {{
                                        'Content-Type': 'application/x-www-form-urlencoded',
                                        'Accept': 'application/json'
                                    }},
                                    body: new URLSearchParams({{
                                        license_key: '{key}',
                                        instance_id: '{instance_id}'
                                    }}).toString(),
                                    signal: AbortSignal.timeout(8000)
                                }}
                            );
                            const data = await resp.json();
                            window.dioxus.send(data.valid ? 'valid' : 'invalid');
                        }} catch(e) {{
                            window.dioxus.send('offline');
                        }}
                    }})();"#
                );
                let mut ev = eval(&js);
                match ev.recv::<String>().await {
                    Ok(s) if s == "valid"   => { is_pro.set(true); }
                    Ok(s) if s == "invalid" => { clear_license(); }
                    // Network error → grant benefit of the doubt (offline-friendly)
                    _                       => { is_pro.set(true); }
                }
            });
        }
    });

    // 3. Auto-check for updates
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

    // 4. Hero counter animation
    use_effect(move || {
        let visible = messages.read().is_empty()
            && file_name.read().is_none()
            && !*loading.read();
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

    // ── Derived state ──
    let sel = *selected_idx.read();
    let detail_msg: Option<FixMessage> = sel.and_then(|i| messages.read().get(i).cloned());
    let show_hero = messages.read().is_empty()
        && file_name.read().is_none()
        && !*loading.read();
    let has_messages = !messages.read().is_empty();
    let pro = *is_pro.read();
    let in_lifecycle = *view_mode.read() == ViewMode::Lifecycle;
    let left_collapsed  = *left_panel_collapsed.read();
    let right_collapsed = *right_panel_collapsed.read();
    let right_w = *right_panel_width.read() as u32;

    rsx! {
        style { {CSS} }
        div { class: "root",

            // ── Toolbar ──
            div { class: "toolbar",
                if file_name.read().is_none() {
                    button { class: "btn btn-process", onclick: move |_| process(), "Process" }
                }
                button { class: "btn btn-clear", onclick: move |_| clear(), "Clear" }
                button { class: "btn btn-load", onclick: move |_| load_file(), "Load file" }

                // CSV export (pro only, visible when messages are loaded)
                if has_messages && pro {
                    button { class: "btn btn-export", onclick: move |_| export_csv(), "Export CSV" }
                }

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

                div { class: "toolbar-spacer" }

                // Update indicator
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

                // Pro badge OR upgrade button
                if pro {
                    span {
                        class: "pro-badge",
                        onclick: move |_| show_license_modal.set(true),
                        "PRO"
                    }
                } else {
                    button {
                        class: "btn btn-upgrade",
                        onclick: move |_| show_license_modal.set(true),
                        "✦ Upgrade to Pro"
                    }
                }
            }

            // ── Two-panel body ──
            div { class: "app-body",

                // ── Left: main content ──
                div {
                    class: "app-main",
                    style: if left_collapsed { "flex: none; width: 0; min-width: 0; overflow: hidden;" } else { "flex: 1; min-width: 0;" },

                    // Panel view tabs (Timeline / Lifecycle)
                    if has_messages {
                        div { class: "panel-tabs",
                            button {
                                class: if !in_lifecycle { "panel-tab panel-tab-active" } else { "panel-tab" },
                                onclick: move |_| view_mode.set(ViewMode::Timeline),
                                "Timeline"
                            }
                            button {
                                class: if in_lifecycle {
                                    "panel-tab panel-tab-pro panel-tab-active"
                                } else {
                                    "panel-tab panel-tab-pro"
                                },
                                onclick: move |_| {
                                    if pro {
                                        view_mode.set(ViewMode::Lifecycle);
                                    } else {
                                        show_license_modal.set(true);
                                    }
                                },
                                if pro { "Trade Latency Analysis" } else { "✦ Trade Latency Analysis" }
                            }
                        }
                    }

                    if show_hero {
                        // ── Hero landing ──
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
                        if in_lifecycle {
                            lifecycle_panel {
                                messages: messages,
                                selected_idx: selected_idx,
                            }
                        } else {
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

                // ── Resize handle (always visible, centered collapse buttons) ──
                // Drag is handled by the persistent native JS handler set up in use_effect above.
                div {
                    class: "resize-handle",
                    // Visual bar only
                    div { class: "resize-handle-bar" }
                    // Centered collapse/expand buttons
                    div { class: "collapse-panel-btns",
                        button {
                            class: "collapse-panel-btn",
                            title: if left_collapsed { "Expand left panel" } else { "Collapse left panel" },
                            onclick: move |_| {
                                let c = *left_panel_collapsed.read();
                                left_panel_collapsed.set(!c);
                            },
                            if left_collapsed { "▶" } else { "◀" }
                        }
                        button {
                            class: "collapse-panel-btn",
                            title: if right_collapsed { "Expand right panel" } else { "Collapse right panel" },
                            onclick: move |_| {
                                let c = *right_panel_collapsed.read();
                                right_panel_collapsed.set(!c);
                            },
                            if right_collapsed { "◀" } else { "▶" }
                        }
                    }
                }

                // ── Right: premium panel ──
                div {
                    id: "premium-panel-main",
                    class: "premium-panel",
                    style: if right_collapsed {
                        "width: 0; min-width: 0; overflow: hidden; border: none;".to_string()
                    } else if left_collapsed {
                        // Left is gone — right fills everything
                        "flex: 1; min-width: 0;".to_string()
                    } else {
                        format!("flex-shrink: 0; width: {right_w}px; min-width: 0;")
                    },

                    // Panel header
                    div { class: "premium-panel-header",
                        span {
                            class: if pro { "premium-panel-title premium-panel-title-pro" } else { "premium-panel-title" },
                            "✦ Pro Features"
                        }
                        button {
                            class: "panel-collapse-btn",
                            title: "Collapse panel",
                            onclick: move |_| right_panel_collapsed.set(true),
                            "›"
                        }
                    }

                        // Upgrade CTA (free users only)
                        if !pro {
                            div { class: "premium-upgrade-cta",
                                p { class: "upgrade-cta-text",
                                    "Unlock powerful analytics for your FIX session data."
                                }
                                button {
                                    class: "btn btn-upgrade-panel",
                                    onclick: move |_| show_license_modal.set(true),
                                    "✦ Upgrade to Pro"
                                }
                                span { class: "upgrade-cta-price", "From $19/month" }
                            }
                        }

                        // Feature cards (scrollable)
                        div { class: "premium-panel-scroll",

                            // ── PRO TIER ──
                            div { class: "feature-tier-label", "PRO TIER" }

                            // Trade Latency Analysis
                            div {
                                class: if pro { "feature-card" } else { "feature-card feature-card-locked" },
                                div { class: "feature-card-top",
                                    span { class: "feature-card-name", "Trade Latency Analysis" }
                                    if pro {
                                        span { class: "badge badge-green feature-badge", "Active" }
                                    } else {
                                        span { class: "badge badge-gray feature-badge", "Pro" }
                                    }
                                }
                                p { class: "feature-card-desc",
                                    "Reconstruct full order chains from RFQ to fill, with latency at each hop."
                                }
                                if pro {
                                    if has_messages {
                                        button {
                                            class: "btn-feature",
                                            onclick: move |_| {
                                                if in_lifecycle {
                                                    view_mode.set(ViewMode::Timeline);
                                                } else {
                                                    view_mode.set(ViewMode::Lifecycle);
                                                }
                                            },
                                            if in_lifecycle { "← Back to Timeline" } else { "View Lifecycle →" }
                                        }
                                    } else {
                                        span { class: "feature-card-hint", "Load data to use" }
                                    }
                                }
                            }

                            // Export CSV
                            div {
                                class: if pro { "feature-card" } else { "feature-card feature-card-locked" },
                                div { class: "feature-card-top",
                                    span { class: "feature-card-name", "Export CSV" }
                                    if pro {
                                        span { class: "badge badge-green feature-badge", "Active" }
                                    } else {
                                        span { class: "badge badge-gray feature-badge", "Pro" }
                                    }
                                }
                                p { class: "feature-card-desc",
                                    "Export all parsed messages to CSV for spreadsheet analysis."
                                }
                                if pro {
                                    span { class: "feature-card-hint", "Use Export CSV button in toolbar" }
                                }
                            }

                            // Fill Quality Scorecard (coming soon)
                            div { class: "feature-card feature-card-soon",
                                div { class: "feature-card-top",
                                    span { class: "feature-card-name", "Fill Quality Score" }
                                    span { class: "badge badge-orange feature-badge", "Soon" }
                                }
                                p { class: "feature-card-desc",
                                    "Per-counterparty fill rate, slippage (bps) & ack latency analytics."
                                }
                            }

                            // Session Health (coming soon)
                            div { class: "feature-card feature-card-soon",
                                div { class: "feature-card-top",
                                    span { class: "feature-card-name", "Session Health" }
                                    span { class: "badge badge-orange feature-badge", "Soon" }
                                }
                                p { class: "feature-card-desc",
                                    "Detect heartbeat gaps, sequence errors, reconnects & latency spikes."
                                }
                            }

                            // Smart Session Summary (coming soon)
                            div { class: "feature-card feature-card-soon",
                                div { class: "feature-card-top",
                                    span { class: "feature-card-name", "Session Summary" }
                                    span { class: "badge badge-orange feature-badge", "Soon" }
                                }
                                p { class: "feature-card-desc",
                                    "AI-generated one-page executive summary of your session."
                                }
                            }

                            // FIX Message Validator (coming soon)
                            div { class: "feature-card feature-card-soon",
                                div { class: "feature-card-top",
                                    span { class: "feature-card-name", "FIX Validator" }
                                    span { class: "badge badge-orange feature-badge", "Soon" }
                                }
                                p { class: "feature-card-desc",
                                    "Validate messages against FIX spec, check required tags & checksums."
                                }
                            }

                            // ── PREMIUM TIER ──
                            div { class: "feature-tier-label feature-tier-label-premium", "PREMIUM TIER" }

                            // AI Chat with Logs
                            div { class: "feature-card feature-card-premium",
                                div { class: "feature-card-top",
                                    span { class: "feature-card-name", "AI Chat with Logs" }
                                    span { class: "badge badge-purple feature-badge", "Premium" }
                                }
                                p { class: "feature-card-desc",
                                    r#""Which fills were worst on AAPL today?" Ask anything about your session."#
                                }
                                if !pro {
                                    button {
                                        class: "btn-feature-upgrade",
                                        onclick: move |_| show_license_modal.set(true),
                                        "Upgrade →"
                                    }
                                }
                            }

                            // Reject Root Cause AI
                            div { class: "feature-card feature-card-premium",
                                div { class: "feature-card-top",
                                    span { class: "feature-card-name", "Reject Root Cause AI" }
                                    span { class: "badge badge-purple feature-badge", "Premium" }
                                }
                                p { class: "feature-card-desc",
                                    "Auto-diagnose order rejections with AI explanation & suggested fix."
                                }
                            }

                            // Order Flow Patterns
                            div { class: "feature-card feature-card-premium",
                                div { class: "feature-card-top",
                                    span { class: "feature-card-name", "Order Flow Patterns" }
                                    span { class: "badge badge-purple feature-badge", "Premium" }
                                }
                                p { class: "feature-card-desc",
                                    "Detect TWAP, VWAP, iceberg & spoofing patterns in order flow."
                                }
                            }

                            // Multi-Session Comparison
                            div { class: "feature-card feature-card-premium",
                                div { class: "feature-card-top",
                                    span { class: "feature-card-name", "Multi-Session Compare" }
                                    span { class: "badge badge-purple feature-badge", "Premium" }
                                }
                                p { class: "feature-card-desc",
                                    "Load two sessions side-by-side and compare fill quality & latency."
                                }
                            }
                        }
                    }
                }
            // ── License Modal ──
            if *show_license_modal.read() {
                div {
                    class: "modal-overlay",
                    onclick: move |_| {
                        show_license_modal.set(false);
                        license_status.set(LicenseStatus::Idle);
                    },
                    div {
                        class: "modal",
                        onclick: move |evt| evt.stop_propagation(),

                        div { class: "modal-header",
                            span { class: "modal-title", "Activate Pro License" }
                            button {
                                class: "modal-close",
                                onclick: move |_| {
                                    show_license_modal.set(false);
                                    license_status.set(LicenseStatus::Idle);
                                },
                                "×"
                            }
                        }

                        if pro {
                            p { class: "modal-desc",
                                "Your Pro license is active. You have access to CSV export, "
                                "Trade Latency Analysis reconstruction, and all future Pro features."
                            }
                            div { class: "modal-deactivate",
                                span { "Want to move to another machine?" }
                                button {
                                    class: "btn-deactivate",
                                    onclick: move |_| deactivate_license(),
                                    "Deactivate"
                                }
                            }
                        } else {
                            p { class: "modal-desc",
                                "Enter your license key to unlock Pro features: "
                                "CSV export, Trade Latency Analysis reconstruction, and more."
                            }
                            label { class: "modal-label", r#for: "license-key-input", "License key" }
                            input {
                                id: "license-key-input",
                                class: "license-input",
                                r#type: "text",
                                placeholder: "XXXX-XXXX-XXXX-XXXX",
                                value: "{license_key_input.read()}",
                                oninput: move |evt| {
                                    license_key_input.set(evt.value());
                                    license_status.set(LicenseStatus::Idle);
                                },
                                onkeydown: move |evt| {
                                    if evt.key() == Key::Enter {
                                        activate_license();
                                    }
                                },
                            }
                            div { class: "modal-actions",
                                button {
                                    class: "btn btn-activate",
                                    disabled: matches!(*license_status.read(), LicenseStatus::Checking),
                                    onclick: move |_| activate_license(),
                                    match *license_status.read() {
                                        LicenseStatus::Checking => "Activating…",
                                        _ => "Activate",
                                    }
                                }
                                button {
                                    class: "btn-buy",
                                    onclick: move |_| {
                                        let url = LS_CHECKOUT_URL.to_string();
                                        std::thread::spawn(move || { let _ = open::that(url); });
                                    },
                                    "Buy Pro →"
                                }
                            }
                            div { class: "activate-status",
                                match license_status.read().clone() {
                                    LicenseStatus::Success => rsx! {
                                        span { class: "activate-ok", "✓ License activated! Welcome to Pro." }
                                    },
                                    LicenseStatus::Error(e) => rsx! {
                                        span { class: "activate-err", "✗ {e}" }
                                    },
                                    LicenseStatus::Checking => rsx! {
                                        span { class: "activate-wait", "Verifying with Lemon Squeezy…" }
                                    },
                                    LicenseStatus::Idle => rsx! { },
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
