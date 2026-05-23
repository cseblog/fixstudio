use std::mem;
use std::time::Instant;

use dioxus::prelude::*;
use dioxus::document::eval;

use crate::components::command_palette::{command_palette, CommandItem};
use crate::components::tab_bar::{tab_bar, TabMenuPos};
use crate::components::tab_menu::tab_menu;
use crate::components::tab_view::tab_view;
use crate::loader::{load_file_at, load_file_tail, pick_and_load_file, pick_and_load_folder, FileLoadResult};
use crate::model::FixMessage;
use crate::parser::parse_all;
use crate::recents::{self, RecentEntry};
use crate::sample::{sample_data, FIX_SPECS};
use crate::style::CSS;
use crate::tab::Tab;
use crate::types::{is_newer_version, UpdateStatus, ViewMode};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const VERSION_URL: &str     = "https://aifixparser.com/latest-version";
const DOWNLOAD_URL: &str    = "https://aifixparser.com/#download";
const GA_ID: &str           = "G-Y9J423BNZ0";

fn offload_replace(signal: &mut Signal<Vec<FixMessage>>, new_data: Vec<FixMessage>) {
    let old = mem::replace(&mut *signal.write(), new_data);
    if !old.is_empty() {
        std::thread::spawn(move || drop(old));
    }
}

fn active_tab(tabs: &Signal<Vec<Tab>>, active_id: &Signal<u64>) -> Option<Tab> {
    let aid = *active_id.read();
    tabs.read().iter().copied().find(|t| t.id == aid)
}

/// Read the file's current modification time as unix-millis, or 0 on error.
/// Used to detect on-disk updates between auto-watch polls.
fn file_mtime_ms(path: &str) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Read the current file size in bytes, or 0 on error. Used as the
/// starting tail offset after a full load so the next mtime tick can
/// stream just the appended bytes.
fn file_size_bytes(path: &str) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// Pump a `FileLoadResult` into the given tab's signals.
fn apply_file_result(t: Tab, r: FileLoadResult, is_soh: &str) {
    let mut messages         = t.messages;
    let mut selected_idx     = t.selected_idx;
    let mut parse_stats      = t.parse_stats;
    let mut file_name        = t.file_name;
    let mut file_path        = t.file_path;
    let mut file_mtime       = t.file_mtime_ms;
    let mut file_tail_offset = t.file_tail_offset;
    let mut label            = t.label;
    let count = r.messages.len();
    let ms    = r.parse_us as u64;
    let path  = r.path.clone();
    parse_stats.set(Some((count, ms)));
    offload_replace(&mut messages, r.messages);
    selected_idx.set(None);
    let name = r.name.clone();
    file_name.set(Some(r.name));
    file_path.set(Some(path.clone()));
    file_mtime.set(file_mtime_ms(&path));
    file_tail_offset.set(file_size_bytes(&path));
    label.set(name);
    eval(&format!(
        "window.gtag && window.gtag('event', 'file_parsed', \
         {{ message_count: {count}, parse_us: {ms}, delimiter: '{is_soh}' }});"
    ));
}

pub fn app() -> Element {
    // ── Top-level state ─────────────────────────────────────────────────────
    let mut tabs:       Signal<Vec<Tab>>    = use_signal(|| vec![Tab::new(0, "Untitled")]);
    let mut next_id:    Signal<u64>         = use_signal(|| 1);
    let mut active_id:  Signal<u64>         = use_signal(|| 0);
    let mut compare_id: Signal<Option<u64>> = use_signal(|| None);

    let mut update_status: Signal<UpdateStatus> = use_signal(|| UpdateStatus::Idle);

    // UI state
    let mut file_menu_open:    Signal<bool> = use_signal(|| false);
    let mut samples_sub_open:  Signal<bool> = use_signal(|| false);
    let mut workspaces_sub_open: Signal<bool> = use_signal(|| false);
    let mut workspaces_state:  Signal<Vec<crate::workspaces::Workspace>>
        = use_signal(crate::workspaces::load);
    let mut palette_open:        Signal<bool> = use_signal(|| false);
    let mut compare_picker_open: Signal<bool> = use_signal(|| false);
    let mut detail_visible:      Signal<bool> = use_signal(|| true);
    let mut timeline_visible:  Signal<bool> = use_signal(|| true);
    let mut tab_menu_pos:      Signal<Option<TabMenuPos>> = use_signal(|| None);
    let mut recents_state:     Signal<Vec<RecentEntry>>   = use_signal(recents::load);

    // ── Per-tab process: parse a Tab's textarea content into its messages.
    //    Same behaviour whether triggered by the Parse button, ⌘↩, the command
    //    palette, or auto-fire on input change.
    let process_tab = move |t: Tab| {
        let mut messages     = t.messages;
        let mut selected_idx = t.selected_idx;
        let mut parse_stats  = t.parse_stats;
        let mut file_name    = t.file_name;
        let mut file_path    = t.file_path;
        let mut file_mtime   = t.file_mtime_ms;
        let mut file_auto    = t.file_auto_watch;
        let input            = t.input;

        let s = input.read().clone();
        if s.is_empty() { return; }
        let started = Instant::now();
        let parsed  = parse_all(&s);
        let ms      = started.elapsed().as_micros() as u64;
        parse_stats.set(Some((parsed.len(), ms)));
        offload_replace(&mut messages, parsed);
        selected_idx.set(None);
        file_name.set(None);
        file_path.set(None);
        file_mtime.set(0);
        file_auto.set(false);
        let mut tail_offset = t.file_tail_offset;
        tail_offset.set(0);
        let mut follow = t.file_follow_tail;
        follow.set(false);
    };

    let process_active = move || {
        let Some(t) = active_tab(&tabs, &active_id) else { return };
        process_tab(t);
    };

    let clear_active = move || {
        let Some(t) = active_tab(&tabs, &active_id) else { return };
        let mut input          = t.input;
        let mut messages       = t.messages;
        let mut selected_idx   = t.selected_idx;
        let mut parse_stats    = t.parse_stats;
        let mut file_name      = t.file_name;
        let mut file_path      = t.file_path;
        let mut file_mtime     = t.file_mtime_ms;
        let mut file_auto      = t.file_auto_watch;
        let mut loaded_files   = t.loaded_files;
        let mut show_file_list = t.show_file_list;
        let mut view_mode      = t.view_mode;
        let mut label          = t.label;

        // Also reset every per-tab filter so the timeline returns to a clean
        // unfiltered state — otherwise stale column-filter values from the
        // previous data set silently hide rows after Clear.
        let mut f_time     = t.f_time;
        let mut f_time_op  = t.f_time_op;
        let mut f_sender   = t.f_sender;
        let mut f_target   = t.f_target;
        let mut f_msg      = t.f_msg;
        let mut f_clord    = t.f_clord;
        let mut f_detail   = t.f_detail;
        let mut filters_open = t.timeline_filters_open;
        let mut display_limit = t.display_limit;

        input.set(String::new());
        offload_replace(&mut messages, Vec::new());
        selected_idx.set(None);
        parse_stats.set(None);
        file_name.set(None);
        file_path.set(None);
        file_mtime.set(0);
        file_auto.set(false);
        let mut tail_offset = t.file_tail_offset;
        tail_offset.set(0);
        let mut follow = t.file_follow_tail;
        follow.set(false);
        loaded_files.set(Vec::new());
        show_file_list.set(false);
        view_mode.set(ViewMode::Timeline);
        f_time.set(String::new());
        f_time_op.set("=".to_string());
        f_sender.set(String::new());
        f_target.set(String::new());
        f_msg.set(String::new());
        f_clord.set(String::new());
        f_detail.set(String::new());
        filters_open.set(true);   // keep filter row visible by default
        display_limit.set(1000);
        label.set("Untitled".to_string());
    };

    let load_sample = move |spec: String| {
        let Some(t) = active_tab(&tabs, &active_id) else { return };
        let mut input        = t.input;
        let mut messages     = t.messages;
        let mut selected_idx = t.selected_idx;
        let mut parse_stats  = t.parse_stats;
        let mut file_name    = t.file_name;
        let mut file_path    = t.file_path;
        let mut file_mtime   = t.file_mtime_ms;
        let mut file_auto    = t.file_auto_watch;
        let mut label        = t.label;

        let s       = sample_data(&spec);
        let started = Instant::now();
        let parsed  = parse_all(&s);
        let ms      = started.elapsed().as_micros() as u64;
        let count   = parsed.len();
        parse_stats.set(Some((count, ms)));
        input.set(s);
        offload_replace(&mut messages, parsed);
        selected_idx.set(None);
        file_name.set(None);
        file_path.set(None);
        file_mtime.set(0);
        file_auto.set(false);
        label.set(format!("Sample {spec}"));
        eval(&format!(
            "window.gtag && window.gtag('event', 'sample_loaded', \
             {{ sample: '{spec}', message_count: {count}, parse_us: {ms} }});"
        ));
    };

    let load_file = move || {
        let Some(t) = active_tab(&tabs, &active_id) else { return };
        let mut loading = t.loading;
        spawn(async move {
            loading.set(true);
            if let Some(r) = pick_and_load_file().await {
                let delim = if r.is_soh { "soh" } else { "pipe" };
                let path  = r.path.clone();
                let name  = r.name.clone();
                apply_file_result(t, r, delim);
                let list = recents::push(&path, &name);
                recents_state.set(list);
            }
            loading.set(false);
        });
    };

    let load_folder = move || {
        let Some(t) = active_tab(&tabs, &active_id) else { return };
        let mut messages     = t.messages;
        let mut selected_idx = t.selected_idx;
        let mut parse_stats  = t.parse_stats;
        let mut loading      = t.loading;
        let mut file_name    = t.file_name;
        let mut loaded_files = t.loaded_files;
        let mut label        = t.label;
        spawn(async move {
            loading.set(true);
            if let Some(r) = pick_and_load_folder().await {
                let count = r.messages.len();
                let ms    = r.parse_us as u64;
                parse_stats.set(Some((count, ms)));
                offload_replace(&mut messages, r.messages);
                selected_idx.set(None);
                let name = r.folder_name.clone();
                file_name.set(Some(r.folder_name));
                loaded_files.set(r.file_names);
                label.set(name);
                eval(&format!(
                    "window.gtag && window.gtag('event', 'folder_parsed', \
                     {{ message_count: {count}, parse_us: {ms} }});"
                ));
            }
            loading.set(false);
        });
    };

    let open_recent = move |path: String| {
        let Some(t) = active_tab(&tabs, &active_id) else { return };
        let mut loading = t.loading;
        spawn(async move {
            loading.set(true);
            if let Some(r) = load_file_at(&path).await {
                let delim = if r.is_soh { "soh" } else { "pipe" };
                let p = r.path.clone();
                let n = r.name.clone();
                apply_file_result(t, r, delim);
                let list = recents::push(&p, &n);
                recents_state.set(list);
            }
            loading.set(false);
        });
    };

    // Save the active tab's current state (file + filters + view mode) as a
    // named workspace. Auto-derives the name from the file label so the
    // first save is one click — user can rename / delete via the menu later.
    let mut save_workspace = move || {
        let Some(t) = active_tab(&tabs, &active_id) else { return };
        let Some(path) = t.file_path.peek().clone() else { return };
        let label = t.label.peek().clone();
        let vm = t.view_mode.peek().clone();
        let view_tag = match vm {
            ViewMode::Now       => "Now",
            ViewMode::Timeline  => "Timeline",
            ViewMode::Lifecycle => "Latency",
            ViewMode::Overview  => "Session",
            ViewMode::Validator => "Validator",
        };
        let ws = crate::workspaces::Workspace {
            name:         format!("{label} · {view_tag}"),
            file_path:    path,
            view_mode:    vm.to_u8(),
            f_sender:     t.f_sender.peek().clone(),
            f_target:     t.f_target.peek().clone(),
            f_msg:        t.f_msg.peek().clone(),
            f_clord:      t.f_clord.peek().clone(),
            f_detail:     t.f_detail.peek().clone(),
            f_time:       t.f_time.peek().clone(),
            f_time_op:    t.f_time_op.peek().clone(),
            selected_lp:  String::new(),
            chain_filter: t.lifecycle_filter_id.peek().clone(),
            auto_watch:   *t.file_auto_watch.peek(),
            follow_tail:  *t.file_follow_tail.peek(),
        };
        let list = crate::workspaces::save(ws);
        workspaces_state.set(list);
    };

    let load_workspace = move |ws: crate::workspaces::Workspace| {
        let Some(t) = active_tab(&tabs, &active_id) else { return };
        let mut loading = t.loading;
        let mut view_mode = t.view_mode;
        let mut f_sender   = t.f_sender;
        let mut f_target   = t.f_target;
        let mut f_msg      = t.f_msg;
        let mut f_clord    = t.f_clord;
        let mut f_detail   = t.f_detail;
        let mut f_time     = t.f_time;
        let mut f_time_op  = t.f_time_op;
        let mut chain_id   = t.lifecycle_filter_id;
        let mut auto_watch = t.file_auto_watch;
        let mut follow     = t.file_follow_tail;
        spawn(async move {
            loading.set(true);
            if let Some(r) = load_file_at(&ws.file_path).await {
                let delim = if r.is_soh { "soh" } else { "pipe" };
                apply_file_result(t, r, delim);
                // Apply filters + view AFTER load so the freshly-set
                // messages signal triggers downstream effects with the
                // workspace's view mode in place from the first paint.
                view_mode.set(ViewMode::from_u8(ws.view_mode));
                f_sender.set(ws.f_sender);
                f_target.set(ws.f_target);
                f_msg.set(ws.f_msg);
                f_clord.set(ws.f_clord);
                f_detail.set(ws.f_detail);
                f_time.set(ws.f_time);
                f_time_op.set(if ws.f_time_op.is_empty() { "=".into() } else { ws.f_time_op });
                chain_id.set(ws.chain_filter);
                auto_watch.set(ws.auto_watch);
                follow.set(ws.follow_tail);
            }
            loading.set(false);
        });
    };

    let mut delete_workspace = move |name: String| {
        let list = crate::workspaces::delete(&name);
        workspaces_state.set(list);
    };

    // Re-read the current `file_path` of any tab and apply the result. Used by
    // the Reload button + the auto-watch poller. Silently no-ops on tabs that
    // were loaded from paste / sample (no file_path).
    let reload_tab = move |t: Tab| {
        let path = match t.file_path.peek().clone() { Some(p) => p, None => return };
        let mut loading = t.loading;
        spawn(async move {
            loading.set(true);
            if let Some(r) = load_file_at(&path).await {
                let delim = if r.is_soh { "soh" } else { "pipe" };
                apply_file_result(t, r, delim);
            }
            loading.set(false);
        });
    };

    // Auto-watch poller: every 1.5s, check every tab whose `file_auto_watch`
    // is on. Prefers incremental tail-load (read only the bytes appended
    // since the last poll); falls back to a full reload when the file
    // shrank (rotation) or no tail offset has been recorded yet.
    use_effect(move || {
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                let snapshot: Vec<Tab> = tabs.peek().clone();
                for t in snapshot {
                    if !*t.file_auto_watch.peek() { continue; }
                    let Some(path) = t.file_path.peek().clone() else { continue };
                    let cur_mtime = file_mtime_ms(&path);
                    if cur_mtime == 0 { continue; }
                    let last_mtime = *t.file_mtime_ms.peek();
                    if cur_mtime <= last_mtime { continue; }

                    let last_offset = *t.file_tail_offset.peek();
                    let cur_size    = file_size_bytes(&path);

                    if last_offset == 0 || cur_size < last_offset {
                        // Either first run after the watch was switched on
                        // without a clean offset, or the file was truncated
                        // / rotated. Either way, safest path is a full reload.
                        reload_tab(t);
                        continue;
                    }

                    // Incremental path: read [last_offset..cur_size], parse,
                    // append. Heavier paths (offload_replace) intentionally
                    // skipped — Vec::extend on the existing Signal works fine.
                    let mut messages    = t.messages;
                    let mut tail_offset = t.file_tail_offset;
                    let mut file_mtime  = t.file_mtime_ms;
                    let follow          = *t.file_follow_tail.peek();
                    spawn(async move {
                        if let Some(tail) = load_file_tail(&path, last_offset).await {
                            if !tail.messages.is_empty() {
                                messages.with_mut(|v| v.extend(tail.messages));
                            }
                            tail_offset.set(tail.new_offset);
                            file_mtime.set(cur_mtime);
                            if follow {
                                // Scroll Timeline + Validator panes to the
                                // bottom once the new rows are committed.
                                let _ = eval(
                                    "requestAnimationFrame(() => {\
                                        document.querySelectorAll('.tbl-body, .latency-tbl-body')\
                                            .forEach(el => el.scrollTop = el.scrollHeight);\
                                    });"
                                );
                            }
                        }
                    });
                }
            }
        });
    });

    let mut add_tab = move || {
        let id = *next_id.read();
        next_id.set(id + 1);
        let new_tab = Tab::new(id, "Untitled");
        tabs.with_mut(|v| v.push(new_tab));
        active_id.set(id);
    };

    let mut duplicate_tab = move |src_id: u64| {
        let Some(src) = tabs.read().iter().copied().find(|t| t.id == src_id) else { return };
        let id = *next_id.read();
        next_id.set(id + 1);
        let label = format!("{} (copy)", src.label.read().clone());
        let dst = Tab::new(id, label);
        // Shallow data copy (message Vec cloned). Acceptable cost for the rare action.
        *dst.input.clone().write()         = src.input.read().clone();
        *dst.messages.clone().write()      = src.messages.read().clone();
        *dst.selected_idx.clone().write()  = *src.selected_idx.read();
        *dst.parse_stats.clone().write()   = *src.parse_stats.read();
        *dst.file_name.clone().write()     = src.file_name.read().clone();
        *dst.loaded_files.clone().write()  = src.loaded_files.read().clone();
        *dst.view_mode.clone().write()     = src.view_mode.read().clone();
        tabs.with_mut(|v| v.push(dst));
        active_id.set(id);
    };

    let mut close_tab = move |id: u64| {
        if tabs.read().len() <= 1 { return; }
        let closing_active  = *active_id.read() == id;
        let closing_compare = *compare_id.read() == Some(id);
        tabs.with_mut(|v| v.retain(|t| t.id != id));
        if closing_compare { compare_id.set(None); }
        if closing_active {
            // Pick the first remaining tab as the new active.
            if let Some(first) = tabs.read().first().map(|t| t.id) {
                active_id.set(first);
                // If that lands on the existing compare target, comparing a tab
                // with itself makes no sense — clear compare.
                if *compare_id.read() == Some(first) { compare_id.set(None); }
            }
        }
    };

    let mut close_others = move |keep_id: u64| {
        tabs.with_mut(|v| v.retain(|t| t.id == keep_id));
        if *compare_id.read() != Some(keep_id) { compare_id.set(None); }
        active_id.set(keep_id);
    };

    let mut cycle_tab = move |dir: i32| {
        let list = tabs.read();
        if list.len() <= 1 { return; }
        let idx = list.iter().position(|t| t.id == *active_id.read()).unwrap_or(0);
        let n   = list.len() as i32;
        let nx  = (((idx as i32 + dir) % n) + n) % n;
        let new_id = list[nx as usize].id;
        drop(list);
        if compare_id.peek().is_some() { compare_id.set(None); }
        active_id.set(new_id);
    };

    // Cancel any in-flight heavy job (Validator, Latency) on the *previous*
    // active tab when the user switches away — otherwise background work keeps
    // spinning on a tab the user has left.
    let mut prev_active: Signal<u64> = use_signal(|| *active_id.peek());
    use_effect(move || {
        let cur = *active_id.read();
        let prev = *prev_active.peek();
        if cur != prev {
            if let Some(old) = tabs.peek().iter().copied().find(|t| t.id == prev) {
                old.cancel_validator();
                old.cancel_lifecycle();
            }
            prev_active.set(cur);
        }
    });

    // Entering compare mode (once, on the None → Some transition) we force
    // both panes to a clean Timeline view + Timeline & Detail visible. We do
    // NOT keep re-forcing on every render — that would override the user's
    // click on the shared "Latency / Session / Validator" tabs because the
    // effect would re-read view_mode, refire, and snap back to Timeline.
    //
    // `peek()` reads without registering a reactive dep, so writes to
    // view_mode / timeline_visible / detail_visible inside the body don't
    // refire the effect. Only the explicit `compare_id.read()` is a tracked
    // dep, which is exactly what we want.
    let mut last_compare: Signal<Option<u64>> = use_signal(|| None);
    use_effect(move || {
        let cur = *compare_id.read();
        let prev = *last_compare.peek();
        if cur == prev { return; }
        last_compare.set(cur);

        let Some(cid) = cur else { return };
        if !*timeline_visible.peek() { timeline_visible.set(true); }
        if !*detail_visible.peek()   { detail_visible.set(true); }

        let aid = *active_id.peek();
        let list = tabs.peek();
        let a_vm = list.iter().find(|t| t.id == aid).map(|t| t.view_mode);
        let c_vm = list.iter().find(|t| t.id == cid).map(|t| t.view_mode);
        if let (Some(mut av), Some(mut cv)) = (a_vm, c_vm) {
            if av.peek().clone() != ViewMode::Timeline { av.set(ViewMode::Timeline); }
            if cv.peek().clone() != ViewMode::Timeline { cv.set(ViewMode::Timeline); }
        }
    });

    // ── On-mount effects ────────────────────────────────────────────────────
    // Anonymous telemetry (Google Analytics): emits app_open + per-load counters
    // (message_count, parse_us, delimiter). NO filenames, NO message content.
    // Users opt out with environment variable AIFIXPARSER_NO_TELEMETRY=1.
    let telemetry_disabled = std::env::var_os("AIFIXPARSER_NO_TELEMETRY")
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false);
    use_effect(move || {
        if telemetry_disabled { return; }
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
                    gtag('event', 'app_open', {{ app_version: '{CURRENT_VERSION}', platform: '{os}' }});
                }};
            }})('{GA_ID}');"#
        ));
    });

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
                Ok(v) if !v.is_empty() && is_newer_version(&v, CURRENT_VERSION) => UpdateStatus::Available(v),
                Ok(v) if !v.is_empty() => UpdateStatus::UpToDate,
                _ => UpdateStatus::Idle,
            });
        });
    });

    // ── Global keyboard shortcuts: install a single JS listener that ships
    //    each accelerator back as a tagged string. Keeps shortcuts working
    //    even when focus is inside a textarea.
    use_effect(move || {
        spawn(async move {
            let mut ev = eval(r#"
                (function() {
                    function send(tag) { window.dioxus.send(tag); }
                    document.addEventListener('keydown', function(e) {
                        var mod = e.metaKey || e.ctrlKey;
                        if (!mod) {
                            if (e.key === 'Escape') send('esc');
                            return;
                        }
                        var k = e.key.toLowerCase();
                        if (k === 'o' && e.shiftKey)      { e.preventDefault(); send('load-folder'); }
                        else if (k === 'o')               { e.preventDefault(); send('load-file'); }
                        else if (k === 't')               { e.preventDefault(); send('new-tab'); }
                        else if (k === 'w')               { e.preventDefault(); send('close-tab'); }
                        else if (k === 'k')               { e.preventDefault(); send('palette'); }
                        else if (k === 'b')               { e.preventDefault(); send('toggle-detail'); }
                        else if (k === 'l')               { e.preventDefault(); send('toggle-timeline'); }
                        else if (k === '\\')              { e.preventDefault(); send('toggle-compare'); }
                        else if (k === 'enter')           { e.preventDefault(); send('process'); }
                        else if (k === 'tab' && e.shiftKey){ e.preventDefault(); send('prev-tab'); }
                        else if (k === 'tab')             { e.preventDefault(); send('next-tab'); }
                        else if (k >= '1' && k <= '9')    { e.preventDefault(); send('tab:' + k); }
                    });
                })();
            "#);
            loop {
                match ev.recv::<String>().await {
                    Ok(tag) => {
                        match tag.as_str() {
                            "load-file"      => load_file(),
                            "load-folder"    => load_folder(),
                            "new-tab"        => add_tab(),
                            "close-tab"      => {
                                let id = *active_id.read();
                                close_tab(id);
                            }
                            "palette"        => {
                                let v = !*palette_open.read();
                                palette_open.set(v);
                            }
                            "toggle-detail"  => {
                                let v = !*detail_visible.read();
                                detail_visible.set(v);
                            }
                            "toggle-timeline" => {
                                let v = !*timeline_visible.read();
                                timeline_visible.set(v);
                            }
                            "toggle-compare" => {
                                if compare_id.read().is_some() {
                                    compare_id.set(None);
                                } else {
                                    let total  = tabs.read().len();
                                    let active = *active_id.read();
                                    if total == 2 {
                                        if let Some(other) = tabs.read().iter()
                                            .find(|t| t.id != active)
                                            .map(|t| t.id)
                                        { compare_id.set(Some(other)); }
                                    } else if total > 2 {
                                        let v = !*compare_picker_open.read();
                                        compare_picker_open.set(v);
                                    }
                                }
                            }
                            "process"        => process_active(),
                            "next-tab"       => cycle_tab(1),
                            "prev-tab"       => cycle_tab(-1),
                            "esc"            => {
                                palette_open.set(false);
                                file_menu_open.set(false);
                                samples_sub_open.set(false);
                                tab_menu_pos.set(None);
                                compare_picker_open.set(false);
                            }
                            s if s.starts_with("tab:") => {
                                if let Ok(n) = s[4..].parse::<usize>() {
                                    let new_id = {
                                        let list = tabs.read();
                                        if n >= 1 && n <= list.len() {
                                            Some(list[n - 1].id)
                                        } else { None }
                                    };
                                    if let Some(id) = new_id {
                                        if compare_id.peek().is_some() { compare_id.set(None); }
                                        active_id.set(id);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    });

    // ── Derived ─────────────────────────────────────────────────────────────
    let active  = active_tab(&tabs, &active_id);
    // Defensive guard: never render compare-with-self even if some code path
    // briefly leaves compare_id == active_id. Prevents the "click the other
    // pane and see the same tab twice" rendering glitch.
    let cmp_id  = compare_id.read()
        .filter(|c| active.map(|t| t.id) != Some(*c));
    let compare = cmp_id.and_then(|cid| tabs.read().iter().copied().find(|t| t.id == cid));
    let multi   = tabs.read().len() > 1;

    // Palette commands — assembled fresh each render so labels reflect current state.
    let palette_items: Vec<CommandItem> = {
        let mut v = vec![
            CommandItem { id: "load-file".into(),       label: "Load file…".into(),         hint: "⌘O".into() },
            CommandItem { id: "load-folder".into(),     label: "Load folder…".into(),       hint: "⌘⇧O".into() },
            CommandItem { id: "new-tab".into(),         label: "Untitled".into(),            hint: "⌘T".into() },
            CommandItem { id: "close-tab".into(),       label: "Close tab".into(),          hint: "⌘W".into() },
            CommandItem { id: "clear".into(),           label: "Clear current tab".into(),  hint: "".into() },
            CommandItem { id: "process".into(),         label: "Process pasted FIX".into(), hint: "⌘↩".into() },
            CommandItem { id: "toggle-detail".into(),   label: "Toggle Detail panel".into(),hint: "⌘B".into() },
            CommandItem { id: "toggle-timeline".into(), label: "Toggle Timeline panel".into(), hint: "⌘L".into() },
            CommandItem { id: "toggle-compare".into(),  label: "Toggle compare".into(),     hint: "⌘\\".into() },
        ];
        for spec in FIX_SPECS.iter() {
            v.push(CommandItem {
                id:    format!("sample:{spec}"),
                label: format!("Load sample · {spec}"),
                hint:  "".into(),
            });
        }
        for r in recents_state.read().iter() {
            v.push(CommandItem {
                id:    format!("recent:{}", r.path),
                label: format!("Open recent · {}", r.name),
                hint:  r.path.clone(),
            });
        }
        v
    };

    let palette_pick = move |id: String| {
        match id.as_str() {
            "load-file"      => load_file(),
            "load-folder"    => load_folder(),
            "new-tab"        => add_tab(),
            "close-tab"      => { let id = *active_id.read(); close_tab(id); }
            "clear"          => clear_active(),
            "process"        => process_active(),
            "toggle-detail"  => { let v = !*detail_visible.read(); detail_visible.set(v); }
            "toggle-timeline" => { let v = !*timeline_visible.read(); timeline_visible.set(v); }
            "toggle-compare" => {
                if compare_id.read().is_some() {
                    compare_id.set(None);
                } else {
                    let total  = tabs.read().len();
                    let active = *active_id.read();
                    if total == 2 {
                        if let Some(other) = tabs.read().iter()
                            .find(|t| t.id != active)
                            .map(|t| t.id)
                        { compare_id.set(Some(other)); }
                    } else if total > 2 {
                        let v = !*compare_picker_open.read();
                        compare_picker_open.set(v);
                    }
                }
            }
            s if s.starts_with("sample:") => load_sample(s[7..].to_string()),
            s if s.starts_with("recent:") => open_recent(s[7..].to_string()),
            _ => {}
        }
    };

    let recent_for_hero = recents_state.read().clone();

    rsx! {
        style { {CSS} }
        div { class: "root",

            // ── Command bar ─────────────────────────────────────────────────
            div { class: "cmdbar",
                // File menu
                div { class: "file-menu-wrap",
                    button {
                        class: if *file_menu_open.read() { "file-menu-btn file-menu-btn-open" } else { "file-menu-btn" },
                        title: "File menu — Load ⌘O · Load folder ⌘⇧O · Samples · Clear",
                        onclick: move |e| {
                            e.stop_propagation();
                            let v = !*file_menu_open.read();
                            file_menu_open.set(v);
                            if !v { samples_sub_open.set(false); }
                        },
                        "≡ File"
                    }
                    if *file_menu_open.read() {
                        div {
                            class: "file-menu-overlay",
                            onclick: move |_| { file_menu_open.set(false); samples_sub_open.set(false); },
                            div {
                                class: "file-menu",
                                onclick: move |e| e.stop_propagation(),
                                div {
                                    class: "file-menu-item",
                                    onclick: move |_| { load_file(); file_menu_open.set(false); },
                                    span { "Load file…" }
                                    span { class: "file-menu-hint", "⌘O" }
                                }
                                div {
                                    class: "file-menu-item",
                                    onclick: move |_| { load_folder(); file_menu_open.set(false); },
                                    span { "Load folder…" }
                                    span { class: "file-menu-hint", "⌘⇧O" }
                                }
                                div { class: "file-menu-sep" }
                                {
                                    let recents = recents_state.read().clone();
                                    if recents.is_empty() {
                                        rsx! {
                                            div { class: "file-menu-item file-menu-item-disabled",
                                                span { "Recent (empty)" }
                                            }
                                        }
                                    } else {
                                        rsx! {
                                            div { class: "file-menu-label", "Recent" }
                                            for r in recents.iter() {
                                                {
                                                    let p = r.path.clone();
                                                    let n = r.name.clone();
                                                    rsx! {
                                                        div {
                                                            class: "file-menu-item file-menu-item-sm",
                                                            title: "{p}",
                                                            onclick: move |_| {
                                                                let path = p.clone();
                                                                open_recent(path);
                                                                file_menu_open.set(false);
                                                            },
                                                            span { class: "file-menu-trunc", "{n}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                div { class: "file-menu-sep" }
                                div {
                                    class: "file-menu-item file-menu-item-sub",
                                    onclick: move |_| {
                                        let v = !*samples_sub_open.read();
                                        samples_sub_open.set(v);
                                    },
                                    span { "Samples" }
                                    span { class: "file-menu-hint", "▸" }
                                }
                                if *samples_sub_open.read() {
                                    for spec in FIX_SPECS.iter() {
                                        {
                                            let s = spec.to_string();
                                            rsx! {
                                                div {
                                                    class: "file-menu-item file-menu-item-indent",
                                                    onclick: move |_| {
                                                        load_sample(s.clone());
                                                        file_menu_open.set(false);
                                                        samples_sub_open.set(false);
                                                    },
                                                    "{spec}"
                                                }
                                            }
                                        }
                                    }
                                }
                                div { class: "file-menu-sep" }
                                div {
                                    class: "file-menu-item",
                                    title: "Save current file + filters + view as a workspace",
                                    onclick: move |_| { save_workspace(); file_menu_open.set(false); },
                                    span { "Save current as workspace" }
                                }
                                div {
                                    class: "file-menu-item file-menu-item-sub",
                                    onclick: move |_| {
                                        let v = !*workspaces_sub_open.read();
                                        workspaces_sub_open.set(v);
                                    },
                                    span { "Workspaces" }
                                    span { class: "file-menu-hint", "▸" }
                                }
                                if *workspaces_sub_open.read() {
                                    {
                                        let list = workspaces_state.read().clone();
                                        if list.is_empty() {
                                            rsx! {
                                                div {
                                                    class: "file-menu-item file-menu-item-indent file-menu-item-disabled",
                                                    span { "(none saved yet)" }
                                                }
                                            }
                                        } else {
                                            rsx! {
                                                for ws in list.iter() {
                                                    {
                                                        let ws_for_load = ws.clone();
                                                        let ws_name_for_del = ws.name.clone();
                                                        let display = ws.name.clone();
                                                        let tip = format!("{} → {}", ws.name, ws.file_path);
                                                        rsx! {
                                                            div {
                                                                class: "file-menu-item file-menu-item-indent file-menu-item-ws",
                                                                title: "{tip}",
                                                                onclick: move |_| {
                                                                    load_workspace(ws_for_load.clone());
                                                                    file_menu_open.set(false);
                                                                    workspaces_sub_open.set(false);
                                                                },
                                                                span { class: "file-menu-trunc", "{display}" }
                                                                button {
                                                                    class: "file-menu-ws-del",
                                                                    title: "Delete workspace",
                                                                    onclick: move |e: MouseEvent| {
                                                                        e.stop_propagation();
                                                                        delete_workspace(ws_name_for_del.clone());
                                                                    },
                                                                    "×"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                div { class: "file-menu-sep" }
                                div {
                                    class: "file-menu-item",
                                    onclick: move |_| { clear_active(); file_menu_open.set(false); },
                                    span { "Clear current tab" }
                                }
                            }
                        }
                    }
                }

                // Tab strip
                tab_bar {
                    tabs:            tabs,
                    active_id:       active_id,
                    compare_id:      compare_id,
                    on_add:          move |_| add_tab(),
                    on_context_menu: move |pos: TabMenuPos| tab_menu_pos.set(Some(pos)),
                }

                div { class: "cmdbar-spacer" }

                // Compare toggle / picker
                if multi {
                    div { class: "compare-picker-wrap",
                        {
                            let total = tabs.read().len();
                            let cmp_label = compare
                                .map(|c| c.label.read().clone());
                            let title = if cmp_label.is_some() {
                                "Stop comparing (⌘\\)".to_string()
                            } else if total == 2 {
                                "Compare with the other tab (⌘\\)".to_string()
                            } else {
                                "Pick a tab to compare against (⌘\\)".to_string()
                            };
                            rsx! {
                                button {
                                    class: if compare.is_some() { "cmdbar-icon cmdbar-icon-on" } else { "cmdbar-icon" },
                                    title: "{title}",
                                    onclick: move |_| {
                                        if compare_id.read().is_some() {
                                            compare_id.set(None);
                                            return;
                                        }
                                        let total = tabs.read().len();
                                        let active = *active_id.read();
                                        if total == 2 {
                                            if let Some(other) = tabs.read().iter()
                                                .find(|t| t.id != active)
                                                .map(|t| t.id)
                                            { compare_id.set(Some(other)); }
                                        } else {
                                            // 3+ tabs: surface a picker so the user chooses.
                                            let v = !*compare_picker_open.read();
                                            compare_picker_open.set(v);
                                        }
                                    },
                                    if let Some(name) = cmp_label {
                                        span { "⇆ vs " span { class: "cmdbar-vs-name", "{name}" } " ✕" }
                                    } else {
                                        span { "⇆" }
                                    }
                                }
                                if *compare_picker_open.read() {
                                    div {
                                        class: "compare-picker-overlay",
                                        onclick: move |_| compare_picker_open.set(false),
                                        div {
                                            class: "compare-picker-menu",
                                            onclick: move |e| e.stop_propagation(),
                                            div { class: "compare-picker-label", "Compare active tab with…" }
                                            {
                                                let active = *active_id.read();
                                                let active_name = tabs.read().iter()
                                                    .find(|t| t.id == active)
                                                    .map(|t| t.label.read().clone())
                                                    .unwrap_or_default();
                                                let others: Vec<(u64, String)> = tabs.read().iter()
                                                    .filter(|t| t.id != active)
                                                    .map(|t| (t.id, t.label.read().clone()))
                                                    .collect();
                                                rsx! {
                                                    div { class: "compare-picker-active",
                                                        span { class: "compare-picker-badge", "A" }
                                                        span { class: "compare-picker-name", "{active_name}" }
                                                    }
                                                    div { class: "compare-picker-sep" }
                                                    for (id, name) in others.into_iter() {
                                                        div {
                                                            class: "compare-picker-item",
                                                            onclick: move |_| {
                                                                compare_id.set(Some(id));
                                                                compare_picker_open.set(false);
                                                            },
                                                            span { class: "compare-picker-badge compare-picker-badge-b", "B" }
                                                            span { class: "compare-picker-name", "{name}" }
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

                // Timeline panel toggle
                button {
                    class: if *timeline_visible.read() { "cmdbar-icon cmdbar-icon-on" } else { "cmdbar-icon" },
                    title: "Toggle Timeline panel (⌘L)",
                    onclick: move |_| { let v = !*timeline_visible.read(); timeline_visible.set(v); },
                    "▥"
                }
                // Detail panel toggle
                button {
                    class: if *detail_visible.read() { "cmdbar-icon cmdbar-icon-on" } else { "cmdbar-icon" },
                    title: "Toggle Detail panel (⌘B) · ⌘K palette",
                    onclick: move |_| { let v = !*detail_visible.read(); detail_visible.set(v); },
                    "▤"
                }

                // Update status / version
                {
                    let status = update_status.read().clone();
                    match status {
                        UpdateStatus::Checking => rsx! {
                            span { class: "update-checking", "…" }
                        },
                        UpdateStatus::Available(v) => rsx! {
                            button {
                                class: "btn btn-update-available",
                                onclick: move |_| {
                                    let url = DOWNLOAD_URL.to_string();
                                    std::thread::spawn(move || { let _ = open::that(url); });
                                },
                                "⬆ v{v}"
                            }
                        },
                        UpdateStatus::UpToDate => rsx! {
                            span { class: "update-ok", "v{CURRENT_VERSION}" }
                        },
                        UpdateStatus::Idle => rsx! {
                            span { class: "update-version", "v{CURRENT_VERSION}" }
                        },
                    }
                }
            }

            // ── Compare-mode shared controls ────────────────────────────────
            if let (Some(a), Some(c)) = (active, compare) {
                {
                    let a_msgs = a.messages;
                    let c_msgs = c.messages;
                    let diff_stats = use_memo(move || {
                        use ahash::AHashSet as HashSet;
                        use crate::tab::message_key;
                        let a_keys: HashSet<String> = a_msgs.read().iter().filter_map(message_key).collect();
                        let b_keys: HashSet<String> = c_msgs.read().iter().filter_map(message_key).collect();
                        let m  = a_keys.intersection(&b_keys).count();
                        let oa = a_keys.difference(&b_keys).count();
                        let ob = b_keys.difference(&a_keys).count();
                        (m, oa, ob)
                    });
                    let (m, oa, ob) = *diff_stats.read();

                    let mut a_vm = a.view_mode;
                    let mut c_vm = c.view_mode;
                    let cur = a_vm.read().clone();
                    let set_both = move |mode: ViewMode| {
                        a_vm.set(mode.clone());
                        c_vm.set(mode);
                    };
                    let in_now = cur == ViewMode::Now;
                    let in_lc  = cur == ViewMode::Lifecycle;
                    let in_ov  = cur == ViewMode::Overview;
                    let in_val = cur == ViewMode::Validator;
                    let in_tl  = cur == ViewMode::Timeline;

                    rsx! {
                        div { class: "compare-bar",
                            div { class: "diff-stats",
                                span { class: "diff-chip diff-chip-match",
                                    span { class: "diff-dot diff-dot-match" }
                                    "{m} match"
                                }
                                span { class: "diff-chip diff-chip-onlya",
                                    span { class: "diff-dot diff-dot-onlya" }
                                    "{oa} only A"
                                }
                                span { class: "diff-chip diff-chip-onlyb",
                                    span { class: "diff-dot diff-dot-onlyb" }
                                    "{ob} only B"
                                }
                            }
                            div { class: "panel-tabs panel-tabs-shared",
                                button {
                                    class: if in_now { "panel-tab panel-tab-active" } else { "panel-tab" },
                                    onclick: { let mut sb = set_both; move |_| sb(ViewMode::Now) },
                                    "Now"
                                }
                                button {
                                    class: if in_tl { "panel-tab panel-tab-active" } else { "panel-tab" },
                                    onclick: { let mut sb = set_both; move |_| sb(ViewMode::Timeline) },
                                    "Timeline"
                                }
                                button {
                                    class: if in_lc { "panel-tab panel-tab-active" } else { "panel-tab" },
                                    onclick: { let mut sb = set_both; move |_| sb(ViewMode::Lifecycle) },
                                    "Latency"
                                }
                                button {
                                    class: if in_ov { "panel-tab panel-tab-active" } else { "panel-tab" },
                                    onclick: { let mut sb = set_both; move |_| sb(ViewMode::Overview) },
                                    "Session"
                                }
                                button {
                                    class: if in_val { "panel-tab panel-tab-active" } else { "panel-tab" },
                                    onclick: { let mut sb = set_both; move |_| sb(ViewMode::Validator) },
                                    "Validator"
                                }
                            }
                        }
                    }
                }
            }

            // ── Body ────────────────────────────────────────────────────────
            div {
                class: if compare.is_some() { "app-body app-body-compare" } else { "app-body" },

                if let Some(t) = active {
                    {
                        let active_input = t.input;
                        let recents_clone = recent_for_hero.clone();
                        rsx! {
                            tab_view {
                                tab: t,
                                detail_visible: detail_visible,
                                timeline_visible: timeline_visible,
                                recent_files: recents_clone,
                                compare_messages: compare.map(|c| c.messages),
                                is_compare_pane: false,
                                hide_view_tabs: compare.is_some(),
                                on_input:       move |v: String| {
                                    active_input.clone().set(v);
                                    process_active();
                                },
                                on_load_file:   move |_| load_file(),
                                on_load_folder: move |_| load_folder(),
                                on_load_sample: move |s: String| load_sample(s),
                                on_open_recent: move |p: String| open_recent(p),
                                on_parse:       move |_| process_active(),
                                on_reload:      move |_| reload_tab(t),
                            }
                        }
                    }
                }

                if let Some(c) = compare {
                    div { class: "compare-divider" }
                    {
                        let cmp_input = c.input;
                        rsx! {
                            tab_view {
                                tab: c,
                                detail_visible: detail_visible,
                                timeline_visible: timeline_visible,
                                recent_files: Vec::new(),
                                compare_messages: active.map(|a| a.messages),
                                is_compare_pane: true,
                                hide_view_tabs: true,
                                on_input:       move |v: String| {
                                    cmp_input.clone().set(v);
                                    process_tab(c);
                                },
                                on_load_file:   move |_| load_file(),
                                on_load_folder: move |_| load_folder(),
                                on_load_sample: move |s: String| load_sample(s),
                                on_open_recent: move |p: String| open_recent(p),
                                on_parse:       move |_| process_tab(c),
                                on_reload:      move |_| reload_tab(c),
                            }
                        }
                    }
                }
            }

            // ── Bottom status bar (heartbeat health) ────────────────────────
            // Always visible while a tab has messages. Shows per-session
            // heartbeat freshness so the operator can see at a glance which
            // counterparty is silent. Worst sessions sort left.
            {
                let bar_messages = active.map(|t| t.messages);
                rsx! {
                    if let Some(msgs_sig) = bar_messages {
                        {
                            let rows = use_memo(move || {
                                crate::live_health::compute(&msgs_sig.read())
                            });
                            let list = rows.read().clone();
                            if !list.is_empty() {
                                rsx! {
                                    div { class: "status-bar",
                                        span { class: "status-bar-label", "Sessions" }
                                        for r in list.iter() {
                                            {
                                                use crate::live_health::HbStatus;
                                                let dot_cls = match r.status {
                                                    HbStatus::Fresh => "status-dot status-dot-fresh",
                                                    HbStatus::Stale => "status-dot status-dot-stale",
                                                    HbStatus::Dead  => "status-dot status-dot-dead",
                                                };
                                                let chip_cls = match r.status {
                                                    HbStatus::Fresh => "status-chip status-chip-fresh",
                                                    HbStatus::Stale => "status-chip status-chip-stale",
                                                    HbStatus::Dead  => "status-chip status-chip-dead",
                                                };
                                                let age = crate::live_health::fmt_age(r.last_msg_age_us);
                                                let tooltip = format!(
                                                    "{}→{} · HB {}s · last msg {} ago{}",
                                                    r.sender, r.target, r.interval_secs, age,
                                                    if r.closed { " · logged out" } else { "" },
                                                );
                                                rsx! {
                                                    span {
                                                        class: "{chip_cls}",
                                                        title: "{tooltip}",
                                                        span { class: "{dot_cls}" }
                                                        span { class: "status-chip-name", "{r.sender}→{r.target}" }
                                                        span { class: "status-chip-age", "{age}" }
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
                    }
                }
            }

            // ── Tab right-click menu ────────────────────────────────────────
            if let Some(pos) = *tab_menu_pos.read() {
                {
                    let active = *active_id.read();
                    let total  = tabs.read().len();
                    let is_pair = *compare_id.read() == Some(pos.tab_id);
                    let can_compare = total > 1 && pos.tab_id != active;
                    rsx! {
                        tab_menu {
                            x: pos.x,
                            y: pos.y,
                            tab_id: pos.tab_id,
                            can_compare: can_compare,
                            is_compare_pair: is_pair,
                            can_close: total > 1,
                            on_close:        move |_| tab_menu_pos.set(None),
                            on_compare:      move |id: u64| compare_id.set(Some(id)),
                            on_stop_compare: move |_| compare_id.set(None),
                            on_duplicate:    move |id: u64| duplicate_tab(id),
                            on_close_tab:    move |id: u64| close_tab(id),
                            on_close_others: move |id: u64| close_others(id),
                        }
                    }
                }
            }

            // ── Command palette ─────────────────────────────────────────────
            command_palette {
                open:    palette_open,
                items:   palette_items,
                on_pick: palette_pick,
            }
        }
    }
}
