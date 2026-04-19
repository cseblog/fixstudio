# UI Flows & Interaction Design

This document covers the application's interaction model, UI component structure, and user flows.
For the underlying data produced by each view, see [data_model.md](data_model.md).
For how messages are parsed before reaching the UI, see [parser.md](parser.md).

---

## Application Shell

The UI is a two-panel desktop layout:

```
┌──────────────────────────────────────────────────────────────────────┐
│  Toolbar: [Open File] [Open Folder] [Export CSV]  [View Mode Tabs]   │
│           [Delimiter badge: SOH/PIPE]  [Parse stats: 1M msgs / 87ms] │
├──────────────────────────────────┬───────────────────────────────────┤
│                                  │                                   │
│        LEFT PANEL                │        RIGHT PANEL                │
│                                  │                                   │
│  Timeline                        │  (one of, per ViewMode):          │
│  ┌──────────────────────────┐    │  • Detail (Table / Raw / JSON)    │
│  │ Filters (6 columns)      │    │  • Lifecycle diagram              │
│  ├──────────────────────────┤    │  • Overview charts                │
│  │ Message row (click → ──)─┼────┤  • Validator report               │
│  │ Message row              │    │                                   │
│  │ Message row              │    │  ← drag handle →                  │
│  │ ...                      │    │                                   │
│  └──────────────────────────┘    │  Premium panel (Pro only):        │
│                                  │  Session Health / Fill Quality    │
│  (infinite scroll)               │  (pinned below right panel)       │
└──────────────────────────────────┴───────────────────────────────────┘
```

The right panel is resizable via a drag handle. Width is synced from JS to Rust via `window.dioxus.send('w:<px>')`. The panel can be collapsed.

---

## State Model

All reactive state lives in `app.rs` as Dioxus `Signal`s:

| Signal | Type | Drives |
|---|---|---|
| `messages` | `Signal<Vec<FixMessage>>` | Timeline list, all analysis panels |
| `selected_idx` | `Signal<Option<usize>>` | Detail panel content |
| `view_mode` | `Signal<ViewMode>` | Which right-panel component renders |
| `skip_heartbeats` | `Signal<bool>` | Timeline filter (heartbeat suppression) |
| `skip_common` | `Signal<bool>` | Detail panel (hide tag 8, 9, 10, 35, 49, 52, 56) |
| `right_panel_width` | `Signal<f64>` | CSS width of right panel |
| `right_panel_collapsed` | `Signal<bool>` | Show/hide right panel |
| `is_pro` | `Signal<bool>` | Show/hide premium panel; enable premium views |
| `parse_stats` | `Signal<Option<ParseStats>>` | Stats badge in toolbar |

---

## User Flows

### Flow 1: Load a File

```
[Toolbar: Open File]
        │
        ▼
rfd::AsyncFileDialog  (native OS file picker)
        │
        ▼ (user picks .log / .fix / .txt)
loader::pick_and_load_file()
  ├─ mmap2::Mmap (zero-copy) — preferred
  └─ file.read() — fallback
        │
        ▼
parse_all_simd_bytes(&[u8])  ← hot path
        │
        ▼
messages.set(parsed_msgs)    ← Signal update triggers re-render
parse_stats.set(Some(stats)) ← "1M msgs parsed in 87ms"
        │
        ▼ (if Pro)
validate_batch + run_health_checks + build_session_summary + build_scorecard
all run in background tokio::spawn → update analysis signals
        │
        ▼
Timeline re-renders (first 1000 rows)
```

### Flow 2: Load a Folder

```
[Toolbar: Open Folder]
        │
        ▼
rfd::AsyncFileDialog (folder picker)
        │
        ▼
loader::pick_and_load_folder()
  DFS traversal of directory tree (max 4096 dirs)
  Filter: .log / .fix / .txt extensions
  Each file: check for "8=FIX" magic bytes
  parse_all_simd_bytes per file → merge into single Vec<FixMessage>
        │
        ▼
messages.set(all_msgs)
```

Messages from different files are merged in discovery order (DFS traversal). They are **not sorted by time** — this is a known limitation (see [issues.md](issues.md)).

### Flow 3: Inspect a Message

```
[Timeline: click any row]
        │
        ▼
selected_idx.set(Some(i))
        │
        ▼
[Right panel: Detail component]
        │
        ├─ [Table tab] — all fields as tag | name | value | description
        ├─ [Raw tab]   — pipe-delimited text with tag annotations
        └─ [JSON tab]  — JSON array of {tag, name, value, description}
                │
                └─ [Copy button] → navigator.clipboard.writeText (JS interop)
```

The detail panel has a "Skip common tags" checkbox (`skip_common` signal). When on, tags 8 (BeginString), 9 (BodyLength), 10 (Checksum), 34 (MsgSeqNum), 35 (MsgType), 49 (SenderCompID), 52 (SendingTime), and 56 (TargetCompID) are hidden to reduce visual noise.

### Flow 4: Filter the Timeline

The timeline has 6 independent column filters. All filters are applied with AND logic. Each filter updates the rendered list immediately (no "Apply" button).

| Filter | Column | Behavior |
|---|---|---|
| `f_time` | Time (tag 52) | Supports `>=HH:MM:SS` and `<=HH:MM:SS` operators, or substring |
| `f_sender` | Sender (tag 49) | Case-insensitive substring |
| `f_target` | Target (tag 56) | Case-insensitive substring |
| `f_msg` | MsgType label | Case-insensitive substring (e.g. "execut") |
| `f_clord` | ClOrdID (tag 11) | Case-insensitive substring |
| `f_detail` | Symbol, Side, Qty, Text | Case-insensitive substring match against combined detail string |

"Skip Heartbeats" is a separate toggle that removes all `35=0` messages.

**Infinite scroll:** The timeline renders the first 1000 filtered rows. As the user scrolls near the bottom, the next 1000 are appended. This is managed by a JS scroll listener (`addEventListener('scroll', ...)`) that calls `window.dioxus.send('scroll')`.

### Flow 5: Validate Messages (Pro)

```
[ViewMode: Validator tab]
        │
        ▼
validator_panel component renders
        │
        ├─ validate_batch(messages) → Vec<ValidationReport>  (computed once on load)
        │
        ├─ Issue summary: total errors, total warnings
        │
        ├─ Issue list (sorted: errors first, then warnings)
        │   Each row: [severity badge] [tag] [code] [message] [fix hint]
        │   Click row → jump to message in timeline (selected_idx.set)
        │
        └─ Single-message debugger:
             [Textarea: paste raw FIX]
             [Validate button]
             → parse_single_for_validation(&bytes)
             → validate_raw(&bytes)   ← also checks checksum + body length
             → show report inline
```

### Flow 6: Session Health (Pro)

```
[Premium panel: Health tab]
        │
        ▼
run_health_checks(messages) → SessionHealthReport  (computed once on load)
        │
        ├─ 7 issue cards (one per rule, collapsed if no issues)
        │   SequenceGap | HeartbeatGap | ExcessiveResends |
        │   Reconnect | MessageRateBurst | LateCancel | RejectedCancel
        │
        ├─ Each card shows: severity badge, technical description, business impact
        │
        └─ Click card → expand to show detail (gaps list, reconnect timeline, etc.)
                      → click message index → jump to timeline
```

### Flow 7: Fill Quality (Pro)

```
[Premium panel: Fill Quality tab]
        │
        ▼
build_scorecard(messages) → FillQualityScorecard
        │
        ├─ Aggregate table: one row per counterparty
        │   Columns: Counterparty | Fill Rate | Slippage (bps) | Partial % |
        │            Reject % | Avg Ack (ms) | Avg Fill (ms) | Cancel Success % | Orders
        │
        ├─ Detail table: one row per (counterparty, symbol)
        │
        └─ Size bucket table: one row per (counterparty, symbol, notional bucket)
             Buckets: <1M | 1M–5M | 5M–10M | 10M–50M | >50M
```

### Flow 8: Export CSV

```
[Toolbar: Export CSV]  (Pro only)
        │
        ▼
Apply current filters (same set as timeline)
        │
        ▼
export::messages_to_csv(filtered_msgs) → String
        │
        ▼
rfd::AsyncFileDialog save dialog
        │
        ▼
Write to chosen path
```

### Flow 9: Activate Pro License

```
[Premium panel: Activation form]
        │
        ▼
User enters Whop license key
[Activate button]
        │
        ▼
tokio::spawn → license::validate_license(key)
  POST https://api.whop.com/api/v2/memberships
  Check: key is active in WHOP_PRODUCT_ID
        │
        ├─ Success → license::save_license(key) → is_pro.set(true)
        │            Premium panel shows (Health, Fill Quality, etc.)
        └─ Failure → show error message inline
```

---

## View Modes

Controlled by `ViewMode` enum and the tab bar in the toolbar:

```
enum ViewMode { Timeline, Lifecycle, Overview, Validator }
```

| Mode | Right Panel Content | Pro Required |
|---|---|---|
| Timeline (default) | Detail panel for selected message | No |
| Lifecycle | ECharts order flow diagram | Yes |
| Overview | ECharts multi-chart dashboard | Yes |
| Validator | Validation issue list + debugger | Yes (batch); No (single) |

---

## Component Hierarchy

```
app()                           ← root; holds all signals
├── hero()                      ← empty state (no messages loaded)
│     "Load a sample file" or "Open file" CTA
│
├── timeline_panel()
│     ├── Filter bar (6 inputs + skip heartbeats toggle)
│     ├── Stats badge (N msgs / Xms)
│     └── Message rows (virtual scroll, first 1000 + lazy load)
│           on_click → selected_idx.set(i)
│
├── (match view_mode)
│   ├── detail_panel()          ← ViewMode::Timeline
│   │     ├── Tab: Table
│   │     ├── Tab: Raw
│   │     └── Tab: JSON
│   │
│   ├── lifecycle_panel()       ← ViewMode::Lifecycle (Pro)
│   │     ECharts flow graph per selected order
│   │
│   ├── overview_panel()        ← ViewMode::Overview (Pro)
│   │     ECharts: msg/sec, orders by symbol, latency histogram, heatmap
│   │
│   └── validator_panel()       ← ViewMode::Validator (Pro/Free)
│         ├── Issue list
│         └── Single-msg debugger
│
└── premium_panel()             ← visible only when is_pro = true
      ├── Tab: Session Summary
      ├── Tab: Health (7 rule cards)
      └── Tab: Fill Quality (3 tables)
```

---

## JavaScript Interop

Dioxus desktop renders in a WebView. Some browser APIs are used via `eval()`:

| Use | JS called |
|---|---|
| Copy to clipboard | `navigator.clipboard.writeText(text)` |
| ECharts init | `echarts.init(document.getElementById('chart-id'))` |
| ECharts update | `chart.setOption({...})` |
| Right-panel resize | `mousemove` listener → `window.dioxus.send('w:<px>')` |
| Infinite scroll | `scroll` listener → `window.dioxus.send('scroll')` |
| Update check | `fetch('https://github.com/...')` |

**CDN dependency:** ECharts is loaded from `cdn.jsdelivr.net`. Without internet access, charts will not render. This affects Lifecycle, Overview, and Health panels.

---

## Sample Data (Hero Screen)

When no file is loaded, the hero component shows sample data options. These call `sample::sample_data()` which returns a small hardcoded FIX session (Logon → NOS → ExecReport → Heartbeat → Logout). Useful for demonstrating the app without a real FIX log file.
