/// Global CSS — Paper / Editorial theme.
/// Cream paper background, ink-black serif headers, ledger rules, financial-newspaper accents.
/// Inspired by printed protocol specs and broadsheet stock tables.
pub const CSS: &str = r#"
* { margin: 0; padding: 0; box-sizing: border-box; }
html, body {
    height: 100%;
    overflow: hidden;
    background: #f5efe2;
    /* Subtle paper grain via layered radial gradients — very low opacity. */
    background-image:
        radial-gradient(circle at 20% 30%, rgba(28,26,23,0.02) 0, transparent 60%),
        radial-gradient(circle at 80% 70%, rgba(28,26,23,0.02) 0, transparent 60%);
}

::selection { background: #fff39a; color: #1c1a17; }
::-webkit-scrollbar { width: 8px; height: 8px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: #c9bfa9; border-radius: 4px; }
::-webkit-scrollbar-thumb:hover { background: #8a8071; }

.root {
    font-family: 'Iowan Old Style', 'Charter', Georgia, serif;
    color: #1c1a17;
    padding: 14px 20px 12px;
    background: transparent;
    height: 100vh;
    font-size: 13px;
    display: flex;
    flex-direction: column;
}
/* Body text uses sans for readability; serif reserved for headlines + numerals. */
.root, .root input, .root textarea, .root select, .root button {
    font-family: -apple-system, 'Helvetica Neue', system-ui, sans-serif;
}
.hero-title-text, .panel-header h2, h1, h2, h3 {
    font-family: 'Iowan Old Style', 'Charter', Georgia, serif !important;
    letter-spacing: -0.2px;
}
/* Tabular data uses monospace for clean column alignment. */
.cell-time, .cell-time-clock, .cell-time-date,
.tag-num, .raw-text, .stat-pill-num {
    font-family: ui-monospace, 'SF Mono', Menlo, monospace;
    font-variant-numeric: tabular-nums;
}

/* Toolbar */
.toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
}
.btn {
    padding: 7px 16px;
    border-radius: 4px;
    border: none;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.12s;
}
.btn-process { background: #2f6b2f; color: #f5efe2; }
.btn-process:hover { opacity: 0.85; }
.btn-clear { background: #ede4cf; color: #6b6356; border: 1px solid #c9bfa9; }
.btn-clear:hover { color: #1c1a17; border-color: #8a8071; }
.btn-load { background: #ede4cf; color: #6b6356; border: 1px solid #c9bfa9; }
.btn-load:hover { color: #1c1a17; border-color: #8a8071; }
.btn-sample { background: transparent; color: #2f6b2f; border: 1px solid rgba(47,107,47,0.4); }
.btn-sample:hover { background: rgba(47,107,47,0.08); }
.btn-sample-inline {
    background: transparent;
    color: #2f6b2f;
    padding: 4px 8px;
    font-size: 12px;
    border: none;
}
.btn-sample-inline:hover { background: rgba(47,107,47,0.08); }
.sample-label { color: #8a8071; font-size: 12px; margin-right: 4px; }
.sample-sep { color: #c9bfa9; font-size: 12px; }
.toolbar-spacer { flex: 1; }
.update-version { color: #8a8071; font-size: 11px; align-self: center; }
.update-ok { color: #2f6b2f; font-size: 11px; align-self: center; }
.update-checking { color: #8a8071; font-size: 11px; align-self: center; font-style: italic; }
.btn-update-available {
    background: #2f6b2f;
    color: #f5efe2;
    padding: 5px 14px;
    font-size: 12px;
}
.btn-update-available:hover { opacity: 0.85; }

/* ── Command bar (single top row: file menu + tabs + actions) ─────────── */
.cmdbar {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 0;
    padding: 2px 0 0;
    border-bottom: 1px solid #c9bfa9;
    min-height: 34px;
}
.cmdbar-spacer { flex: 1; min-width: 12px; }
.cmdbar-icon {
    background: transparent;
    border: 1px solid transparent;
    color: #6b6356;
    font-size: 13px;
    padding: 5px 10px;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.12s;
    line-height: 1;
}
.cmdbar-icon:hover { background: #ede4cf; color: #1c1a17; border-color: #c9bfa9; }
.cmdbar-icon-on   { color: #2f6b2f; border-color: rgba(47,107,47,0.35); background: rgba(47,107,47,0.06); }
.cmdbar-search {
    background: #ede4cf;
    border: 1px solid #c9bfa9;
    color: #1c1a17;
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 12px;
    min-width: 240px;
    outline: none;
}
.cmdbar-search:focus { border-color: #2f6b2f; }

/* ── File menu (≡ dropdown) ───────────────────────────────────────────── */
.file-menu-wrap { position: relative; }
.file-menu-btn {
    background: transparent;
    border: 1px solid #c9bfa9;
    color: #1c1a17;
    font-size: 13px;
    font-weight: 600;
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.12s;
}
.file-menu-btn:hover { border-color: #2f6b2f; color: #2f6b2f; }
.file-menu-btn-open { border-color: #2f6b2f; color: #2f6b2f; background: rgba(47,107,47,0.06); }

.file-menu-overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
}
.file-menu {
    position: absolute;
    top: 38px;
    left: 0;
    min-width: 240px;
    background: #faf6ec;
    border: 1px solid #c9bfa9;
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(28,26,23,0.18);
    padding: 4px 0;
    z-index: 51;
}
.file-menu-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 7px 14px;
    color: #1c1a17;
    font-size: 13px;
    cursor: pointer;
    gap: 16px;
}
.file-menu-item:hover { background: #ede4cf; }
.file-menu-item-sm { font-size: 12px; padding: 5px 14px; color: #3a342c; }
.file-menu-item-disabled { color: #8a8071; cursor: default; pointer-events: none; }
.file-menu-item-indent { padding-left: 28px; font-size: 12px; color: #2f6b2f; }
.file-menu-item-sub { color: #6b6356; }
.file-menu-hint { color: #8a8071; font-size: 11px; }
.file-menu-label {
    padding: 4px 14px 2px;
    font-size: 10px;
    color: #8a8071;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-weight: 700;
}
.file-menu-sep { height: 1px; background: #c9bfa9; margin: 4px 0; }
.file-menu-trunc {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 220px;
}

/* ── Tab strip (inside command bar) ───────────────────────────────────── */
.tab-strip {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-wrap: nowrap;
    overflow-x: auto;
    overflow-y: hidden;
    max-width: calc(100vw - 600px);
    padding-bottom: 0;
}
.tab-strip::-webkit-scrollbar { height: 3px; }
.tab-strip::-webkit-scrollbar-thumb { background: #c9bfa9; }

.tab-chip {
    display: inline-flex;
    align-items: center;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    overflow: hidden;
    height: 28px;
    padding: 0 4px 0 10px;
    cursor: pointer;
    transition: background 0.12s, border-color 0.12s;
    gap: 6px;
    user-select: none;
}
.tab-chip:hover { background: #ede4cf; }
.tab-chip-active {
    background: #ede4cf;
    border-bottom-color: #2f6b2f;
}
.tab-chip-compare { border-bottom-color: #b78427; }
.tab-chip-label {
    color: #6b6356;
    font-size: 12px;
    font-weight: 600;
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.tab-chip-active .tab-chip-label { color: #1c1a17; }
.tab-chip-cmp-badge {
    background: #b78427;
    color: #f5efe2;
    font-size: 10px;
    font-weight: 700;
    padding: 1px 5px;
    border-radius: 4px;
    margin-right: 6px;
}
.tab-chip-close {
    background: transparent;
    border: none;
    color: #8a8071;
    font-size: 14px;
    width: 20px;
    height: 20px;
    cursor: pointer;
    border-radius: 4px;
    margin-right: 4px;
    line-height: 1;
}
.tab-chip-close:hover { background: #c9bfa9; color: #1c1a17; }

.tab-add {
    background: transparent;
    border: none;
    color: #8a8071;
    font-size: 18px;
    width: 28px;
    height: 28px;
    cursor: pointer;
    border-radius: 4px;
    line-height: 1;
    padding: 0;
    margin-left: 2px;
}
.tab-add:hover { background: #ede4cf; color: #2f6b2f; }

.compare-picker {
    display: inline-flex;
    align-items: center;
}
.compare-select {
    background: #faf6ec;
    border: 1px solid #c9bfa9;
    color: #1c1a17;
    padding: 5px 10px;
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
}
.compare-select:hover { border-color: #b78427; }
.compare-btn {
    background: transparent;
    border: 1px solid #b78427;
    color: #b78427;
    font-size: 12px;
    font-weight: 600;
    padding: 5px 12px;
    border-radius: 4px;
    cursor: pointer;
}
.compare-btn:hover { background: rgba(183,132,39,0.08); }
.compare-btn-active { background: rgba(183,132,39,0.12); }

/* Tab body panes */
.tab-pane {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}
.tab-pane-compare {
    border-left: 1px solid #c9bfa9;
    padding-left: 12px;
}
.compare-divider { width: 12px; flex-shrink: 0; }
.compare-empty {
    color: #8a8071;
    font-size: 12px;
    font-style: italic;
    padding: 8px 4px;
}
.app-body-compare { gap: 0; }

/* Compare-mode header bar (diff stats + shared view tabs) */
.compare-bar {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 16px;
    margin: 8px 0;
    padding: 6px 12px;
    background: #faf6ec;
    border: 1px solid #c9bfa9;
    border-radius: 6px;
}
.diff-stats {
    display: flex;
    align-items: center;
    gap: 8px;
    justify-self: start;
}
.panel-tabs-shared { justify-self: center; }
.diff-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    font-weight: 600;
    color: #1c1a17;
    padding: 3px 9px;
    background: #ede4cf;
    border: 1px solid #c9bfa9;
    border-radius: 999px;
    letter-spacing: 0.2px;
}
.diff-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: inline-block;
}
.diff-dot-match { background: #2f6b2f; }
.diff-dot-onlya { background: #b78427; }
.diff-dot-onlyb { background: #15467a; }
.diff-chip-match  { border-color: rgba(47,107,47,0.45); }
.diff-chip-onlya  { border-color: rgba(183,132,39,0.45); }
.diff-chip-onlyb  { border-color: rgba(21,70,122,0.45); }

.panel-tabs-shared { margin-bottom: 0; }

/* Diff highlighting on timeline rows: 4px left border + soft tint */
.tbl-row.row-match {
    box-shadow: inset 4px 0 0 0 #2f6b2f;
    background: rgba(47,107,47,0.04);
}
.tbl-row.row-match:hover { background: rgba(47,107,47,0.10); }
.tbl-row.row-diverge {
    box-shadow: inset 4px 0 0 0 #b78427;
    background: rgba(183,132,39,0.04);
}
.tbl-row.row-diverge:hover { background: rgba(183,132,39,0.12); }
.tbl-row.row-selected.row-match,
.tbl-row.row-selected.row-diverge {
    background: rgba(47,107,47,0.18);
}

/* Textarea / loading placeholder */
.fix-loading {
    width: 100%;
    min-height: 110px;
    padding: 10px 12px;
    border: 1px solid #c9bfa9;
    border-radius: 4px;
    background: #faf6ec;
    color: #6b6356;
    font-size: 13px;
    font-weight: 600;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
}
.fix-file-banner {
    width: 100%;
    padding: 4px 10px;
    border: 1px solid #c9bfa9;
    border-radius: 4px;
    background: #faf6ec;
    color: #6b6356;
    font-size: 11px;
    margin-bottom: 8px;
    display: flex;
    align-items: center;
    gap: 8px;
}
.fix-file-icon { font-size: 12px; opacity: 0.7; }
.fix-file-name {
    font-weight: 600;
    color: #1c1a17;
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
}
.fix-file-toggle {
    background: #ede4cf;
    border: 1px solid #c9bfa9;
    border-radius: 4px;
    color: #6b6356;
    font-size: 11px;
    padding: 2px 8px;
    cursor: pointer;
    white-space: nowrap;
}
.fix-file-toggle:hover { border-color: #8a8071; color: #1c1a17; }
.fix-file-toggle-on {
    background: rgba(47,107,47,0.10);
    border-color: #2f6b2f;
    color: #2f6b2f;
    position: relative;
}
.fix-file-toggle-on:hover { color: #1e4a1e; border-color: #1e4a1e; }
/* Pulsing dot replaces the ○/● glyph state — visual proof to the user
   that the watcher is alive and polling the file every 1.5s. */
.fix-file-toggle-on::before {
    content: "";
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #2f6b2f;
    margin-right: 6px;
    vertical-align: middle;
    animation: live-pulse 1.5s ease-in-out infinite;
}
@keyframes live-pulse {
    0%, 100% { opacity: 1.0;  transform: scale(1); }
    50%      { opacity: 0.35; transform: scale(1.3); }
}

/* ── Anomaly banner ────────────────────────────────────────────────────
   Sits above the view-tabs after a parse. Chips list each fired alert
   with severity-coloured borders. Silent on healthy logs. */
.anomaly-banner {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 4px 0 8px;
}
.anom-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    background: #faf6ec;
    border: 1px solid #c9bfa9;
}
.anom-icon { font-size: 12px; line-height: 1; }
.anom-warn {
    border-color: rgba(183,132,39,0.55);
    background:   rgba(183,132,39,0.10);
    color: #7d5712;
}
.anom-crit {
    border-color: rgba(178,34,34,0.60);
    background:   rgba(178,34,34,0.10);
    color: #8a1818;
}
.fix-file-list {
    width: 100%;
    background: #faf6ec;
    border: 1px solid #c9bfa9;
    border-radius: 4px;
    margin-bottom: 16px;
    max-height: 200px;
    overflow-y: auto;
    padding: 6px 0;
}
.fix-file-list-item {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 11px;
    color: #6b6356;
    padding: 3px 14px;
    border-bottom: 1px solid #ede4cf;
}
.fix-file-list-item:last-child { border-bottom: none; }

.fix-input {
    width: 100%;
    min-height: 110px;
    padding: 10px 12px;
    border: 1px solid #c9bfa9;
    border-radius: 4px;
    background: #faf6ec;
    color: #1c1a17;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
    resize: vertical;
    margin-bottom: 16px;
    transition: border-color 0.12s;
}
.fix-input::placeholder { color: #8a8071; }
.fix-input:focus { outline: none; border-color: #2f6b2f; }
.fix-input-compact {
    min-height: 70px;
    max-height: 140px;
    margin-top: 4px;
    margin-bottom: 0;
    font-size: 11px;
    line-height: 1.45;
    resize: vertical;
}

/* Textarea + Parse-button cluster — makes the "how do I run this?" step
   visible instead of buried behind a keyboard shortcut. */
.input-with-action {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0;
    margin-bottom: 12px;
}
.input-with-action-compact { margin-bottom: 8px; }
.input-parse-btn {
    align-self: flex-end;
    margin-top: 6px;
    padding: 6px 18px;
    font-size: 12px;
    border-radius: 4px;
}
.input-parse-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
}

/* Panels */
.panels {
    display: flex;
    gap: 20px;
    flex: 1;
    min-height: 0;
}
.panel-timeline {
    flex: 1.3;
    min-width: 0;
    display: flex;
    flex-direction: column;
}
.panel-detail {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
}
.panel-detail .table-wrap {
    flex: 1;
    min-height: 0;
}
.panel-detail .raw-text-wrap {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
}
.panel-detail .raw-text {
    flex: 1;
    min-height: 0;
}

.panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
    flex-shrink: 0;
}
/* Panels that manage their own spacing via flex gap don't need extra margin.
   latency-panel gets a tighter gap after its header only. */
.latency-panel > .panel-header,
.overview-panel > .panel-header,
.validator-panel > .panel-header { margin-bottom: 0; }
.latency-panel > .panel-header { padding-bottom: 4px; border-bottom: 1px solid #c9bfa9; }
.panel-title {
    display: flex;
    align-items: baseline;
    gap: 10px;
}
.panel-header h2 {
    font-size: 11px;
    font-weight: 700;
    color: #8a8071;
    text-transform: uppercase;
    letter-spacing: 1px;
}
.parse-stats {
    font-size: 11px;
    color: #8a8071;
}
.filter-count {
    font-size: 11px;
    color: #6b6356;
}
.cap-notice {
    padding: 10px 20px;
    text-align: center;
    font-size: 11px;
    color: #6b6356;
    border-top: 1px solid #c9bfa9;
}

.check-label {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: #8a8071;
    cursor: pointer;
    transition: color 0.12s;
}
.check-label:hover { color: #1c1a17; }
.check-label input {
    cursor: pointer;
    accent-color: #2f6b2f;
}

/* Tables */
.table-wrap {
    border: 1px solid #c9bfa9;
    border-radius: 4px;
    overflow: hidden;
    background: #faf6ec;
    display: flex;
    flex-direction: column;
}
.panel-timeline .table-wrap {
    flex: 1;
    min-height: 0;
}

.tbl-header {
    background: #ede4cf;
    font-weight: 600;
    color: #6b6356;
    font-size: 12px;
}

.tbl-body {
    overflow-y: auto;
    flex: 1;
    min-height: 0;
}
.panel-detail .tbl-body {
    flex: 1;
    min-height: 0;
}

.tbl-row {
    border-top: 1px solid #ede4cf;
    cursor: pointer;
    transition: background 0.1s;
}
.tbl-row:hover { background: rgba(28,26,23,0.05); }
/* Selection uses highlighter yellow tint + bolder text; the ClOrdID rail (inset
   box-shadow on the row) still shows through because we don't override shadow. */
.row-selected {
    background: rgba(255,243,154,0.55) !important;
}
.row-selected .cell-time,
.row-selected .cell-detail,
.row-selected span { color: #1c1a17 !important; font-weight: 600; }
.row-group-cont .cell-time-date { display: none; }

/* Faded date prefix so the eye locks on the time when scrolling. */
.cell-time-date  { color: #8a8071; }
.cell-time-clock { color: #3a342c; font-variant-numeric: tabular-nums; }
.row-selected .cell-time-clock { color: #1c1a17; }

.tbl-timeline-row {
    display: grid;
    grid-template-columns: 130px 80px 80px 150px 1fr 170px;
    gap: 6px;
    padding: 5px 10px 5px 14px;
    align-items: center;
    font-size: 12px;
}
.tbl-timeline-row > span:nth-child(2),
.tbl-timeline-row > span:nth-child(3) {
    color: #1c1a17;
    font-weight: 600;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 11px;
    letter-spacing: 0.3px;
}

.tbl-detail-row {
    display: grid;
    grid-template-columns: 44px 140px 1fr 160px;
    gap: 6px;
    padding: 6px 10px;
    align-items: center;
    font-size: 12px;
}
.detail-filter-row {
    padding: 6px 10px 4px;
    border-bottom: 1px solid #c9bfa9;
}
.detail-filter-input {
    width: 100%;
    box-sizing: border-box;
    background: #faf6ec;
    border: 1px solid #c9bfa9;
    border-radius: 4px;
    padding: 5px 8px;
    font-size: 11px;
    color: #1c1a17;
    outline: none;
}
.detail-filter-input::placeholder { color: #8a8071; }
.detail-filter-input:focus { border-color: #8a8071; }

.cell-time { font-variant-numeric: tabular-nums; color: #8a8071; }
.cell-detail { color: #3a342c; font-size: 12px; }
.tag-num { color: #2f6b2f; font-variant-numeric: tabular-nums; text-align: right; }

/* Column filters */
.tbl-filter { background: #faf6ec; border-bottom: 1px solid #ede4cf; }

.time-filter-wrap {
    display: flex;
    align-items: center;
    gap: 3px;
    width: 100%;
}
.time-op-select {
    flex-shrink: 0;
    width: 34px;
    background: #ede4cf;
    border: 1px solid #c9bfa9;
    border-radius: 4px;
    color: #6b6356;
    font-size: 11px;
    font-weight: 700;
    font-family: inherit;
    padding: 1px 2px;
    outline: none;
    cursor: pointer;
    text-align: center;
}
.time-op-select:focus { border-color: #2f6b2f; }
.col-filter {
    width: 100%;
    background: transparent;
    border: none;
    border-bottom: 1px solid transparent;
    color: #1c1a17;
    font-size: 11px;
    font-family: inherit;
    padding: 2px 2px;
    outline: none;
}
.col-filter::placeholder { color: #c9bfa9; }
.col-filter:focus { border-bottom-color: #2f6b2f; }
.btn-clear-filter {
    padding: 3px 10px;
    border-radius: 4px;
    border: 1px solid #b22222;
    background: transparent;
    color: #b22222;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.12s;
}
.btn-clear-filter:hover { background: rgba(178,34,34,0.1); }

/* ── Unified empty state ──────────────────────────────────────────────
   Used everywhere an area has no data to show: timeline filtered out,
   detail panel with no selection, batch validator with all rows clean,
   etc. One pattern → consistent visual rhythm. */
.empty-state {
    padding: 32px 24px;
    text-align: center;
    color: #6b6356;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    line-height: 1.5;
}
.empty-state-icon  { font-size: 28px; opacity: 0.65; margin-bottom: 4px; }
.empty-state-title { font-size: 14px; font-weight: 600; color: #3a342c; }
.empty-state-hint  { font-size: 12px; color: #8a8071; max-width: 380px; }

/* Badges */
.badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
}
.badge-green   { background: rgba(47,107,47,0.14);  color: #1e4a1e; border: 1px solid rgba(47,107,47,0.45); }
.badge-red     { background: rgba(178,34,34,0.12);   color: #8a1818; border: 1px solid rgba(178,34,34,0.40); }
.badge-amber   { background: rgba(183,132,39,0.14);  color: #7d5712; border: 1px solid rgba(183,132,39,0.45); }
.badge-orange  { background: rgba(122,58,138,0.14);  color: #4f2459; border: 1px solid rgba(122,58,138,0.40); }
.badge-gray    { background: rgba(107,99,86,0.10);   color: #4a4337; border: 1px solid rgba(107,99,86,0.30); }
.badge-blue    { background: rgba(21,70,122,0.12);   color: #0d2e54; border: 1px solid rgba(21,70,122,0.40); }
.badge-teal    { background: rgba(21,70,122,0.15);  color: #15467a; }
.badge-purple  { background: rgba(47,107,47,0.15);  color: #2f6b2f; }
.badge-yellow  { background: rgba(47,107,47,0.15);  color: #2f6b2f; }
.badge-slate   { background: rgba(201,191,169,0.8);     color: #1c1a17; }

/* Header actions */
.header-actions {
    display: flex;
    align-items: center;
    gap: 12px;
}

/* View tabs (Table / Raw Text toggle) */
.view-tabs {
    display: flex;
    gap: 4px;
    margin-bottom: 8px;
}
.tab-btn {
    padding: 5px 14px;
    border-radius: 4px;
    border: 1px solid #c9bfa9;
    background: transparent;
    color: #8a8071;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: color 0.12s, border-color 0.12s;
}
.tab-btn:hover { color: #1c1a17; border-color: #8a8071; }
.tab-active {
    background: #ede4cf !important;
    color: #1c1a17 !important;
    border-color: #8a8071 !important;
}

/* Raw text view */
.raw-text-wrap {
    border: 1px solid #c9bfa9;
    border-radius: 4px;
    background: #faf6ec;
    overflow: hidden;
}
.raw-text-toolbar {
    display: flex;
    justify-content: flex-end;
    padding: 6px 10px;
    border-bottom: 1px solid #c9bfa9;
    background: #ede4cf;
}
.btn-copy {
    background: #2f6b2f;
    color: #f5efe2;
    padding: 4px 14px;
    font-size: 12px;
}
.btn-copy:hover { opacity: 0.85; }
.btn-copied {
    background: #2f6b2f;
    color: #f5efe2;
    padding: 4px 14px;
    font-size: 12px;
}
.raw-text {
    padding: 12px 14px;
    margin: 0;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
    line-height: 1.6;
    color: #1c1a17;
    white-space: pre-wrap;
    word-break: break-all;
    overflow-y: auto;
    user-select: text;
    -webkit-user-select: text;
}

/* Scrollbar */
.tbl-body::-webkit-scrollbar, .raw-text::-webkit-scrollbar { width: 6px; }
.tbl-body::-webkit-scrollbar-track, .raw-text::-webkit-scrollbar-track { background: #faf6ec; }
.tbl-body::-webkit-scrollbar-thumb, .raw-text::-webkit-scrollbar-thumb { background: #c9bfa9; border-radius: 4px; }
.tbl-body::-webkit-scrollbar-thumb:hover, .raw-text::-webkit-scrollbar-thumb:hover { background: #8a8071; }

/* ── Hero / Landing ─────────────────────────────────────────────────────── */
.hero {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 28px;
    padding: 20px;
}

/* Title block */
.hero-title { text-align: center; animation: hero-fade-up 0.35s ease both; }
.hero-icon {
    font-size: 46px;
    display: block;
    margin-bottom: 10px;
}
.hero-title h1 {
    font-size: 28px;
    font-weight: 700;
    color: #1c1a17;
    letter-spacing: -0.5px;
    margin-bottom: 7px;
}
.hero-title p { font-size: 13px; color: #8a8071; letter-spacing: 0.2px; }

/* Stat cards */
.hero-stats { display: flex; gap: 16px; align-items: stretch; }
.hero-stat {
    border: 1px solid #c9bfa9;
    border-radius: 6px;
    padding: 18px 26px;
    text-align: center;
    min-width: 140px;
    background: #faf6ec;
    animation: hero-fade-up 0.35s ease both;
}
.hero-stat-a { animation-delay: 0.08s; }
.hero-stat-b { animation-delay: 0.20s; }
.hero-stat-featured {
    padding: 22px 34px;
    border-color: rgba(47,107,47,0.35);
}
.hero-stat-value {
    font-size: 34px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    letter-spacing: -1.5px;
    line-height: 1;
    margin-bottom: 10px;
    color: #1c1a17;
}
.hero-stat-featured .hero-stat-value { font-size: 40px; color: #2f6b2f; }
.hero-stat-a .hero-stat-value { color: #1c1a17; }
.hero-stat-b .hero-stat-value { color: #1c1a17; }
.hero-stat-suffix { font-size: 16px; font-weight: 700; opacity: 0.6; letter-spacing: 0; }
.hero-stat-featured .hero-stat-suffix { font-size: 20px; opacity: 0.75; }
.hero-stat-unit  { font-size: 12px; font-weight: 600; color: #8a8071; margin-bottom: 3px; }
.hero-stat-label { font-size: 11px; color: #c9bfa9; }

/* Parse bar */
.hero-demo {
    width: 100%;
    max-width: 480px;
    animation: hero-fade-up 0.35s 0.28s ease both;
}
.hero-demo-label {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: #8a8071;
    margin-bottom: 8px;
}
.hero-demo-time { color: #2f6b2f; font-weight: 700; }
.hero-bar-track {
    height: 4px;
    border-radius: 2px;
    background: #ede4cf;
    border: 1px solid #c9bfa9;
    overflow: hidden;
}
.hero-bar-fill {
    height: 100%;
    border-radius: 2px;
    background: linear-gradient(to right, #b22222, #2f6b2f);
    transform: scaleX(0);
    transform-origin: left;
    animation: hero-bar-grow 1.4s 0.45s cubic-bezier(0.22, 1, 0.36, 1) forwards;
}
@keyframes hero-bar-grow { to { transform: scaleX(1); } }

/* Hint line */
.hero-hint {
    font-size: 12px;
    color: #8a8071;
    text-align: center;
    animation: hero-fade-up 0.35s 0.35s ease both;
}
.hero-hint-kbd {
    display: inline-block;
    background: #ede4cf;
    color: #6b6356;
    padding: 1px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    border: 1px solid #c9bfa9;
}

@keyframes hero-fade-up {
    from { opacity: 0; transform: translateY(12px); }
    to   { opacity: 1; transform: translateY(0); }
}

/* Export CSV button */
.btn-export-csv {
    padding: 3px 10px;
    border-radius: 4px;
    border: 1px solid #c9bfa9;
    background: #ede4cf;
    color: #6b6356;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: border-color 0.12s, color 0.12s;
}
.btn-export-csv:hover { border-color: #8a8071; color: #1c1a17; }

/* Panel view tabs — ghost style with underline (mirrors top tab strip) */
.panel-tabs {
    display: flex;
    gap: 2px;
    margin-bottom: 8px;
    border-bottom: 1px solid #c9bfa9;
}
.panel-tab {
    padding: 6px 14px;
    border: none;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: #8a8071;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: color 0.12s, border-color 0.12s, background 0.12s;
    border-radius: 0;
    margin-bottom: -1px;
}
.panel-tab:hover { color: #1c1a17; background: #faf6ec; }
.panel-tab-active {
    color: #1c1a17 !important;
    border-bottom-color: #2f6b2f !important;
    background: transparent !important;
}

/* Modal overlay */
.modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(28,26,23,0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(3px);
    animation: modal-fade-in 0.15s ease both;
}
@keyframes modal-fade-in {
    from { opacity: 0; }
    to   { opacity: 1; }
}
.modal {
    background: #faf6ec;
    border: 1px solid #c9bfa9;
    border-radius: 6px;
    padding: 28px 32px;
    width: 460px;
    max-width: 90vw;
    animation: modal-slide-up 0.18s cubic-bezier(0.22,1,0.36,1) both;
}
@keyframes modal-slide-up {
    from { opacity: 0; transform: translateY(16px); }
    to   { opacity: 1; transform: translateY(0); }
}
.modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 20px;
}
.modal-title {
    font-size: 16px;
    font-weight: 700;
    color: #1c1a17;
}
.modal-close {
    background: transparent;
    border: none;
    color: #8a8071;
    font-size: 20px;
    cursor: pointer;
    padding: 0 4px;
    line-height: 1;
}
.modal-close:hover { color: #1c1a17; }
.modal-desc {
    font-size: 13px;
    color: #6b6356;
    margin-bottom: 18px;
    line-height: 1.6;
}
.modal-label {
    display: block;
    font-size: 12px;
    font-weight: 600;
    color: #6b6356;
    margin-bottom: 6px;
}
.modal-actions {
    display: flex;
    gap: 10px;
    align-items: center;
}
/* ── Two-panel body layout ────────────────────────────────────────────── */

.app-body {
    display: flex;
    flex-direction: row;
    flex: 1;
    min-height: 0;
}

.app-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

/* Resize handle */
.resize-handle {
    width: 18px;
    flex-shrink: 0;
    position: relative;
    margin: 0 1px;
    cursor: col-resize;
    user-select: none;
    -webkit-user-select: none;
}
.resize-handle-bar {
    position: absolute;
    inset: 0;
    z-index: 0;
    pointer-events: none;
}
.resize-handle-bar::after {
    content: '';
    display: block;
    position: absolute;
    top: 0; bottom: 0;
    left: 50%;
    transform: translateX(-50%);
    width: 4px;
    background: #c9bfa9;
    border-radius: 2px;
    transition: background 0.12s;
}
.resize-handle:hover .resize-handle-bar::after { background: #2f6b2f; }

.collapse-panel-btns {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    display: flex;
    flex-direction: column;
    gap: 3px;
    z-index: 1;
}
.collapse-panel-btn {
    width: 18px;
    height: 20px;
    background: #faf6ec;
    border: 1px solid #c9bfa9;
    color: #8a8071;
    font-size: 9px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    transition: all 0.12s;
    padding: 0;
    line-height: 1;
}
.collapse-panel-btn:hover { background: #ede4cf; color: #1c1a17; border-color: #8a8071; }

/* Features panel */
.premium-panel {
    flex-shrink: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: #faf6ec;
    border: 1px solid #c9bfa9;
    border-radius: 6px;
    overflow: hidden;
    min-height: 0;
    transition: width 0.15s ease;
}

.panel-collapse-btn {
    background: transparent;
    border: none;
    color: #8a8071;
    font-size: 18px;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    line-height: 1;
    transition: all 0.12s;
}
.panel-collapse-btn:hover {
    color: #1c1a17;
    background: #ede4cf;
}

.premium-panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px 9px;
    border-bottom: 1px solid #c9bfa9;
    flex-shrink: 0;
    background: #faf6ec;
}
.premium-panel-title {
    font-size: 11px;
    font-weight: 700;
    color: #8a8071;
    letter-spacing: 0.5px;
    text-transform: uppercase;
}

.premium-panel-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 10px 10px 16px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
}
.premium-panel-scroll::-webkit-scrollbar { width: 4px; }
.premium-panel-scroll::-webkit-scrollbar-track { background: #faf6ec; }
.premium-panel-scroll::-webkit-scrollbar-thumb { background: #c9bfa9; border-radius: 2px; }
.premium-panel-scroll::-webkit-scrollbar-thumb:hover { background: #8a8071; }

.feature-tier-label {
    font-size: 10px;
    font-weight: 700;
    color: #c9bfa9;
    letter-spacing: 1px;
    padding: 6px 4px 2px;
    margin-top: 2px;
}

.feature-card {
    background: #faf6ec;
    border: 1px solid #c9bfa9;
    border-radius: 6px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 5px;
}
.feature-card-soon {
    border-color: rgba(178,34,34,0.6);
}

.feature-card-top {
    display: flex;
    align-items: center;
    gap: 6px;
}
.feature-card-name {
    font-size: 12px;
    font-weight: 600;
    color: #1c1a17;
    flex: 1;
}
.feature-badge {
    flex-shrink: 0;
    font-size: 10px;
    padding: 1px 6px;
}
.feature-card-desc {
    font-size: 11px;
    color: #8a8071;
    line-height: 1.5;
}
.feature-card-hint {
    font-size: 10px;
    color: #c9bfa9;
    font-style: italic;
}

.btn-feature {
    padding: 4px 10px;
    border-radius: 4px;
    border: 1px solid rgba(47,107,47,0.35);
    background: transparent;
    color: #2f6b2f;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.12s;
    align-self: flex-start;
    margin-top: 2px;
}
.btn-feature:hover { background: rgba(47,107,47,0.08); }
.btn-feature:disabled { border-color: #c9bfa9; color: #8a8071; cursor: not-allowed; }


/* ── Lifecycle panel ──────────────────────────────────────────────────── */

.tbl-lc-row {
    display: grid;
    grid-template-columns: 160px 80px 50px 70px 120px 36px 1fr;
    gap: 6px;
    padding: 6px 10px;
    align-items: center;
    font-size: 12px;
}
.tbl-lc-chain-row {
    display: grid;
    grid-template-columns: 32px 160px 130px 1fr;
    gap: 6px;
    padding: 6px 10px;
    align-items: center;
    font-size: 12px;
}
.lc-clordid { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; color: #15467a; }
/* Stacked-id cell — each id on its own line so multi-id rows
   (Quote linking RFQ→QuoteID, ER linking ClOrdID+OrigClOrdID) are
   scannable. Labels (C / O / Q / QR) sit flush-left. */
.cell-id-stack {
    display: flex;
    flex-direction: column;
    gap: 1px;
    line-height: 1.2;
    min-width: 0;
}
.id-line {
    display: flex;
    align-items: baseline;
    gap: 4px;
    min-width: 0;
}
.id-line > span:last-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.id-clordid    { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; color: #1c1a17; font-weight: 600; }
.id-orig       { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; color: #6b6356; font-style: italic; }
.id-quoteid    { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; color: #15467a; }
.id-quotereqid { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; color: #7a3a8a; }
.id-label {
    color: #8a8071;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.4px;
    min-width: 18px;
    text-align: right;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
/* Inline "jump to latency chain" button per id line. Invisible by default to
   keep the timeline dense; fades in on row hover, click switches view_mode
   to Lifecycle with the chain filter pre-populated. */
.id-jump {
    background: transparent;
    border: none;
    color: #15467a;
    font-size: 11px;
    line-height: 1;
    cursor: pointer;
    padding: 0 3px;
    margin-left: 2px;
    opacity: 0;
    transition: opacity 0.1s, color 0.1s, background 0.1s;
    border-radius: 2px;
}
.tbl-row:hover .id-jump { opacity: 0.7; }
.id-jump:hover { opacity: 1 !important; color: #2f6b2f; background: rgba(47,107,47,0.10); }
.lc-symbol  { color: #1c1a17; font-weight: 600; }
.lc-side    { color: #1c1a17; font-weight: 600; }
.lc-qty     { color: #1c1a17; font-variant-numeric: tabular-nums; }
.lc-count   { color: #8a8071; font-weight: 700; text-align: center; }
.lc-time    { color: #8a8071; font-size: 11px; font-variant-numeric: tabular-nums; }
.lc-seq     { color: #8a8071; text-align: right; font-variant-numeric: tabular-nums; }
.lc-info    { color: #8a8071; font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.lc-selected-id { font-size: 12px; color: #2f6b2f; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
.lc-empty   { padding: 30px 20px; text-align: center; color: #8a8071; font-size: 13px; }

/* ── Trade Latency Analysis panel ─────────────────────────────────────────── */
.latency-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
    box-sizing: border-box;
    background: #faf6ec;
}
.latency-header { display: flex; align-items: baseline; gap: 12px; flex-wrap: wrap; }
.latency-header-left { display: flex; flex-direction: column; gap: 3px; }
.latency-title { margin: 0; font-size: 15px; font-weight: 600; color: #1c1a17; letter-spacing: 0.1px; }
.latency-anel-headerheader-meta { font-size: 11px; color: #c9bfa9; }
.latency-section { display: flex; flex-direction: column; gap: 8px; }
.latency-section-title {
    font-size: 10px; font-weight: 600; letter-spacing: 0.8px;
    color: #c9bfa9; text-transform: uppercase;
}
.latency-section-sub { font-size: 10px; color: #c9bfa9; margin-top: -4px; }
.latency-chart-wrap { background: #f5efe2; border-radius: 4px; padding: 6px; overflow: hidden; }
.latency-stat-row { display: flex; gap: 8px; flex-wrap: wrap; }
.latency-stat-item {
    flex: 1; min-width: 70px;
    background: #f5efe2; border-radius: 4px; padding: 8px 10px;
    display: flex; flex-direction: column; align-items: center; gap: 2px;
    border-top: 2px solid #ede4cf;
}
.latency-stat-val { font-size: 15px; font-weight: 700; font-variant-numeric: tabular-nums; color: #3a342c; }
.latency-stat-lbl { font-size: 10px; color: #c9bfa9; text-transform: uppercase; letter-spacing: 0.5px; }
.latency-stat-green  { border-color: #ede4cf; } .latency-stat-green  .latency-stat-val { color: #3a342c; }
.latency-stat-cyan   { border-color: #ede4cf; } .latency-stat-cyan   .latency-stat-val { color: #3a342c; }
.latency-stat-yellow { border-color: #b22222; } .latency-stat-yellow .latency-stat-val { color: #b22222; }
.latency-stat-orange { border-color: #7a3a8a; } .latency-stat-orange .latency-stat-val { color: #7a3a8a; }
.latency-stat-red    { border-color: #b22222; } .latency-stat-red    .latency-stat-val { color: #b22222; }
.tbl-sym-row, .tbl-slow-row {
    display: grid; align-items: center; font-size: 12px; padding: 5px 8px; gap: 6px;
}
.tbl-sym-row  { grid-template-columns: 80px 50px 48px 70px 70px 60px 60px; }
.tbl-slow-row { grid-template-columns: 130px 70px 38px 68px 68px 40px 1fr; }
.latency-tbl-body .tbl-row:nth-child(even) { background: #efe8d2; }
.latency-tbl-body .tbl-row:hover { background: #ede4cf; }
.latency-cell-mean { color: #3a342c; font-variant-numeric: tabular-nums; }
.latency-cell-p95  { color: #b22222; font-variant-numeric: tabular-nums; }
.latency-cell-min  { color: #3a342c; font-variant-numeric: tabular-nums; }
.latency-cell-max  { color: #b22222; font-variant-numeric: tabular-nums; }
.latency-empty {
    display: flex; flex-direction: column; align-items: center;
    justify-content: center; padding: 40px 20px; gap: 8px;
    color: #c9bfa9; text-align: center;
}
.latency-empty-icon  { font-size: 40px; }
.latency-empty-title { font-size: 15px; font-weight: 600; color: #1c1a17; margin: 0; }
.latency-empty-hint  { font-size: 12px; margin: 0; }
.latency-empty-list  { font-size: 12px; text-align: left; padding-left: 20px; }

/* Flow chart */
.flow-chart-viewport {
    position: relative;
    overflow: hidden;
    height: 220px;
    background: #faf6ec;
    border-radius: 6px;
    cursor: grab;
    border: 1px solid #c9bfa9;
}
.flow-chart-viewport:active { cursor: grabbing; }
#flow-wrap {
    position: absolute;
    top: 0; left: 0;
    will-change: transform;
    user-select: none;
}
.flow-row-clickable {
    cursor: pointer;
    transition: background 0.08s;
}
.flow-row-clickable:hover { background: #ede4cf !important; }
.flow-row-selected {
    background: #c9bfa9 !important;
    outline: 1px solid #8a8071;
    cursor: pointer;
}

/* ── Phase overview ────────────────────────────────────────────────────── */
.phase-light {
    background: #faf6ec;
    border-radius: 6px;
    padding: 14px 16px;
    border: 1px solid #c9bfa9;
}
.phase-light .phase-card {
    background: #ede4cf;
    border-color: #c9bfa9;
}
.phase-light .phase-card:hover { background: #ede4cf; border-color: #8a8071; }
.phase-light .phase-card-active {
    background: #fff39a !important;
    border-color: #8a8071 !important;
    border-bottom-color: transparent !important;
}
.phase-light .phase-card-label { color: #8a8071; }
.phase-light .phase-card-p50   { color: #15467a; }
.phase-light .phase-card-sub   { color: #c9bfa9; }
.phase-light .phase-card-caret { color: #c9bfa9; }
.phase-light .health-green  { color: #15467a; }
.phase-light .health-yellow { color: #2f6b2f; }
.phase-light .health-orange { color: #b78427; }
.phase-light .health-red    { color: #b22222; }
.phase-light .health-none   { color: #c9bfa9; }
.phase-light .phase-detail {
    background: #ede4cf;
    border-color: #8a8071;
    border-top: none;
}
.phase-light .phase-detail-count { color: #15467a; }
.phase-light .phase-detail-hint  { color: #c9bfa9; }
.phase-light .latency-chart-wrap { background: #faf6ec; }
.phase-light .phase-stat-cell { background: #faf6ec; border-color: #ede4cf; }
.phase-light .phase-stat-val  { color: #15467a; }
.phase-light .phase-stat-lbl  { color: #c9bfa9; }
.phase-light .phase-stat-green  { border-color: #ede4cf; } .phase-light .phase-stat-green  .phase-stat-val { color: #15467a; }
.phase-light .phase-stat-cyan   { border-color: #ede4cf; } .phase-light .phase-stat-cyan   .phase-stat-val { color: #15467a; }
.phase-light .phase-stat-yellow { border-color: #2f6b2f; } .phase-light .phase-stat-yellow .phase-stat-val { color: #2f6b2f; }
.phase-light .phase-stat-orange { border-color: #b78427; } .phase-light .phase-stat-orange .phase-stat-val { color: #b78427; }
.phase-light .phase-stat-red    { border-color: #b22222; } .phase-light .phase-stat-red    .phase-stat-val { color: #b22222; }
.phase-light .phase-stat-drilling {
    border-color: #2f6b2f;
    border-top-width: 3px;
}
.phase-light .phase-no-data { color: #c9bfa9; }

/* ── Lifecycle Reconstructor table layout ────────────────── */
.tbl-chain-row {
    display: grid;
    grid-template-columns: 9rem 6rem 3.5rem 4rem 6rem 6rem 6rem 6rem 6rem 6rem 3.5rem;
    gap: 0;
    align-items: center;
    padding: 0 8px;
}
/* Lifecycle / latency filter bar — visually identical to the timeline
   filter row (.tbl-filter + .col-filter) so the two views feel like one
   tool. Borderless inputs that get an ink-green underline on focus, sitting
   in a faint paper card. Status pills are tab-style chips. */
.recon-filter-bar {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    margin-bottom: 8px;
    padding: 6px 12px;
    background: #faf6ec;
    border: 1px solid #ede4cf;
    border-radius: 4px;
}
/* Reuse .col-filter for inputs (defined above) — alias kept for legacy
   callers that may still hand-pick the wider standalone treatment. */
.recon-filter-input {
    background: transparent;
    border: none;
    border-bottom: 1px solid transparent;
    color: #1c1a17;
    font-size: 12px;
    font-family: inherit;
    padding: 3px 4px;
    width: 200px;
    outline: none;
}
.recon-filter-input::placeholder { color: #c9bfa9; }
.recon-filter-input:focus { border-bottom-color: #2f6b2f; }
.recon-filter-btn {
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: #8a8071;
    font-size: 11px;
    font-weight: 600;
    padding: 3px 10px;
    cursor: pointer;
    transition: all 0.1s;
    border-radius: 0;
}
.recon-filter-btn:hover { color: #1c1a17; }
.recon-filter-btn-active {
    color: #1c1a17;
    border-bottom-color: #2f6b2f;
}
.recon-more {
    color: #8a8071;
    font-size: 11px;
    text-align: center;
    padding: 8px 0 4px;
}
.status-filled    { color: #2f6b2f; font-weight: 600; font-size: 11px; }
.status-partial   { color: #b22222; font-weight: 600; font-size: 11px; }
.status-cancelled { color: #8a8071; font-size: 11px; }
.status-rejected  { color: #b22222; font-weight: 600; font-size: 11px; }
.status-expired   { color: #c9bfa9; font-size: 11px; }
.status-open      { color: #15467a; font-size: 11px; }
.status-unknown   { color: #c9bfa9; font-size: 11px; }
.chain-type-rfq    { color: #15467a; font-size: 11px; font-weight: 600; }
.chain-type-direct { color: #8a8071; font-size: 11px; }
.lc-sort-hdr { cursor: pointer; user-select: none; transition: color 0.08s; }
.lc-sort-hdr:hover { color: #2f6b2f; }
.lc-sort-hdr-active { color: #2f6b2f; font-weight: 700; }

/* ── Phase overview: cards + detail ─────────────────────────────────────── */
.phase-overview-wrap { gap: 0; }

.phase-cards-row {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
}
.phase-card {
    background: #f5efe2;
    border: 1px solid #ede4cf;
    border-radius: 4px;
    padding: 12px 14px 10px;
    cursor: pointer;
    transition: border-color 0.12s, background 0.12s;
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 3px;
}
.phase-card:hover { border-color: #c9bfa9; background: #efe8d2; }
.phase-card-active { border-color: #c9bfa9 !important; background: #faf6ec !important; border-bottom-left-radius: 0 !important; border-bottom-right-radius: 0 !important; border-bottom-color: transparent !important; }
.phase-card-label { font-size: 10px; font-weight: 600; letter-spacing: 0.8px; text-transform: uppercase; color: #c9bfa9; }
.phase-card-p50 { font-size: 22px; font-weight: 700; font-variant-numeric: tabular-nums; letter-spacing: -0.5px; line-height: 1.1; color: #3a342c; }
.phase-card-sub { font-size: 11px; color: #ede4cf; }
.phase-card-caret { position: absolute; top: 10px; right: 12px; font-size: 9px; color: #ede4cf; }
.health-green  { color: #3a342c; }
.health-yellow { color: #b22222; }
.health-orange { color: #7a3a8a; }
.health-red    { color: #b22222; }
.health-none   { color: #ede4cf; }
.phase-card:has(.health-yellow) { border-left: 3px solid #b22222; }
.phase-card:has(.health-orange) { border-left: 3px solid #7a3a8a; }
.phase-card:has(.health-red)    { border-left: 3px solid #b22222; }

.phase-detail {
    background: #f5efe2;
    border: 1px solid #ede4cf;
    border-top: none;
    border-radius: 0 0 5px 5px;
    padding: 12px 12px 10px;
    display: flex;
    flex-direction: column;
    gap: 10px;
}
.phase-detail-meta { display: flex; align-items: center; gap: 12px; }
.phase-detail-count { font-size: 12px; color: #6b6356; font-weight: 600; }
.phase-detail-hint  { font-size: 11px; color: #ede4cf; font-style: italic; }
.phase-hist-full { width: 100%; }

.phase-stats-table { display: grid; grid-template-columns: repeat(6, 1fr); gap: 6px; }
.phase-stat-cell {
    background: #faf6ec; border-radius: 4px; padding: 8px 10px;
    display: flex; flex-direction: column; align-items: center; gap: 2px;
    border-top: 2px solid #ede4cf;
}
.phase-stat-val { font-size: 14px; font-weight: 700; font-variant-numeric: tabular-nums; color: #3a342c; }
.phase-stat-lbl { font-size: 10px; color: #ede4cf; text-transform: uppercase; letter-spacing: 0.5px; }
.phase-stat-green  { border-color: #ede4cf; } .phase-stat-green  .phase-stat-val { color: #3a342c; }
.phase-stat-cyan   { border-color: #ede4cf; } .phase-stat-cyan   .phase-stat-val { color: #3a342c; }
.phase-stat-yellow { border-color: #b22222; } .phase-stat-yellow .phase-stat-val { color: #b22222; }
.phase-stat-orange { border-color: #7a3a8a; } .phase-stat-orange .phase-stat-val { color: #7a3a8a; }
.phase-stat-red    { border-color: #b22222; } .phase-stat-red    .phase-stat-val { color: #b22222; }

.phase-stat-drill {
    cursor: pointer;
    transition: border-color 0.08s;
}
.phase-stat-drill:hover {
    border-top-width: 3px;
    border-color: #8a8071;
}
.phase-stat-drilling {
    border-color: #2f6b2f;
    border-top-width: 3px;
}

.drill-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    background: rgba(201,191,169,0.6);
    border: 1px solid #c9bfa9;
    border-radius: 4px;
    padding: 6px 12px;
    font-size: 12px;
    color: #6b6356;
    margin-bottom: 8px;
    font-variant-numeric: tabular-nums;
}
.drill-banner-clear {
    margin-left: auto;
    cursor: pointer;
    color: #8a8071;
    font-size: 16px;
    line-height: 1;
    padding: 0 3px;
    transition: color 0.08s;
}
.drill-banner-clear:hover { color: #b22222; }

.phase-no-data { font-size: 11px; color: #c9bfa9; padding: 12px 0; }

/* ── Inline chain timeline expansion ────────────────────────────────────── */
.chain-inline-expand {
    background: #f5efe2;
    border-left: 2px solid #c9bfa9;
    padding: 10px 12px 10px 16px;
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 11.5px;
    font-family: 'JetBrains Mono', 'Fira Mono', 'Consolas', monospace;
    overflow-x: auto;
}
.cit-line {
    display: flex;
    align-items: center;
    white-space: nowrap;
    gap: 0;
}
.cit-arrow {
    color: #8a8071;
    padding: 0 2px;
    user-select: none;
}
.cit-node {
    display: inline-block;
    padding: 2px 7px;
    border-radius: 4px;
    border: 1px solid currentColor;
    font-weight: 600;
    font-size: 11px;
    background: rgba(28,26,23,0.08);
}

/* ── Overview / Session Analysis ──────────────────────────────────────────── */
.overview-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: #faf6ec;
}
.overview-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 14px 16px 10px;
    border-bottom: 1px solid #c9bfa9;
    flex-shrink: 0;
    gap: 12px;
}
.overview-header-left { flex: 1; min-width: 0; }
.overview-title {
    font-size: 15px;
    font-weight: 700;
    color: #1c1a17;
    margin-bottom: 4px;
}
.overview-meta {
    font-size: 11px;
    color: #8a8071;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.overview-header-actions { flex-shrink: 0; }

/* Tab bar */
.overview-tab-bar {
    display: flex;
    gap: 2px;
    padding: 8px 12px 0;
    border-bottom: 1px solid #c9bfa9;
    flex-shrink: 0;
    background: #faf6ec;
}
.overview-tab {
    padding: 6px 14px;
    border: none;
    border-radius: 4px 5px 0 0;
    background: transparent;
    color: #8a8071;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: color 0.12s, border-color 0.12s;
}
.overview-tab:hover { color: #1c1a17; }
.overview-tab-active {
    color: #2f6b2f;
    border-bottom-color: #2f6b2f;
    background: rgba(183,132,39,0.06);
}
.tab-badge-warn {
    display: inline-block;
    background: #b22222;
    color: #1c1a17;
    font-size: 10px;
    font-weight: 700;
    border-radius: 6px;
    padding: 0 5px;
    margin-left: 4px;
    vertical-align: middle;
}

/* Content area */
.overview-content {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
}

/* ── Summary tab ──────────────────────────────────────────────────────────── */
.summary-layout {
    display: flex;
    gap: 24px;
    align-items: flex-start;
}
.summary-body { flex: 1; min-width: 0; max-width: 700px; }
.summary-charts {
    flex-shrink: 0;
    width: 240px;
    display: flex;
    flex-direction: column;
    gap: 16px;
}
.summary-chart-block { display: flex; flex-direction: column; gap: 6px; }
.summary-chart-label {
    font-size: 11px;
    font-weight: 600;
    color: #8a8071;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.summary-pie { height: 200px; width: 100%; }
.summary-section { padding: 4px 0; }
.summary-divider {
    height: 1px;
    background: #c9bfa9;
    margin: 12px 0;
}
.summary-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 3px 0;
    font-size: 13px;
}
.summary-sub { padding-left: 10px; }
.summary-label {
    min-width: 170px;
    color: #8a8071;
    font-size: 12px;
    flex-shrink: 0;
}
.summary-value { color: #1c1a17; }
.summary-bold { font-weight: 700; }
.summary-mono { font-family: ui-monospace, 'SF Mono', monospace; }
.summary-session-label { color: #2f6b2f; font-weight: 600; }
.summary-duration { color: #8a8071; font-size: 12px; }
.summary-pct { color: #8a8071; font-size: 11px; }
.summary-pct-green { color: #2f6b2f; }
.summary-pct-warn  { color: #b22222; }
.summary-warn      { color: #b22222; }
.summary-spike-meta { color: #8a8071; font-size: 11px; font-weight: 400; }
.summary-symbol {
    display: inline-block;
    font-family: ui-monospace, 'SF Mono', monospace;
    font-size: 12px;
    color: #6b6356;
}
.summary-symbol-count { color: #8a8071; font-size: 11px; }
.summary-events-header {
    font-size: 12px;
    font-weight: 700;
    color: #8a8071;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 8px;
}
.summary-event {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 3px 0;
    font-size: 12px;
}
.event-icon { font-size: 13px; flex-shrink: 0; }
.event-warn    { color: #b22222; }
.event-info    { color: #15467a; }
.event-ok      { color: #2f6b2f; }
.event-time { color: #8a8071; font-family: ui-monospace, 'SF Mono', monospace; font-size: 11px; }
.event-desc { color: #1c1a17; }

/* ── Fill Quality tab ────────────────────────────────────────────────────── */
.scorecard-wrap { overflow: hidden; }
.scorecard-breadcrumb {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: #8a8071;
    margin-bottom: 10px;
}
.scorecard-back-btn {
    background: transparent;
    border: none;
    color: #2f6b2f;
    font-size: 12px;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
}
.scorecard-back-btn:hover { background: rgba(47,107,47,0.08); }
.scorecard-breadcrumb-sep { color: #c9bfa9; }
.scorecard-table-wrap { overflow-x: auto; }
.scorecard-table {
    display: grid;
    grid-template-columns: 140px repeat(7, 1fr);
    min-width: 680px;
    font-size: 12px;
}
.scorecard-row {
    display: contents;
}
.scorecard-row-clickable > .sc-cell:first-child { cursor: pointer; }
.scorecard-row-clickable:hover > .sc-cell { background: rgba(201,191,169,0.5); }
.scorecard-header > .sc-cell {
    background: #faf6ec;
    border-bottom: 1px solid #c9bfa9;
    padding: 6px 8px;
    font-weight: 700;
    color: #8a8071;
    text-transform: uppercase;
    font-size: 10px;
    letter-spacing: 0.4px;
    position: sticky;
    top: 0;
    z-index: 1;
}
.sc-cell {
    padding: 5px 8px;
    border-bottom: 1px solid rgba(201,191,169,0.5);
    color: #1c1a17;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.sc-header-cell { cursor: pointer; user-select: none; }
.sc-header-cell:hover { color: #1c1a17; }
.sc-sorted { color: #2f6b2f !important; }
.sc-num  { font-family: ui-monospace, 'SF Mono', monospace; text-align: right; }
.sc-cp   { color: #6b6356; font-weight: 600; }
.sc-sym  { color: #2f6b2f; }
.sc-good { color: #2f6b2f; font-family: ui-monospace, 'SF Mono', monospace; text-align: right; }
.sc-ok   { color: #b22222; font-family: ui-monospace, 'SF Mono', monospace; text-align: right; }
.sc-bad  { color: #b22222; font-family: ui-monospace, 'SF Mono', monospace; text-align: right; }

/* ── Health tab ──────────────────────────────────────────────────────────── */
.health-empty {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 24px 0;
    font-size: 14px;
    color: #8a8071;
}
.health-ok-icon { font-size: 20px; color: #2f6b2f; }
.health-list { display: flex; flex-direction: column; gap: 14px; }

/* Card */
.health-card {
    background: #faf6ec;
    border: 1px solid #c9bfa9;
    border-radius: 6px;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
}
.health-card-header {
    display: flex;
    align-items: baseline;
    gap: 8px;
}
.health-icon { font-size: 13px; flex-shrink: 0; }
.health-critical { color: #b22222; }
.health-warning  { color: #b22222; }
.health-info     { color: #15467a; }
.health-kind {
    font-weight: 700;
    font-size: 13px;
    color: #1c1a17;
    flex-shrink: 0;
}
.health-tech-desc {
    font-size: 12px;
    color: #b22222;
    font-family: ui-monospace, 'SF Mono', monospace;
    flex: 1;
    min-width: 0;
}
.health-impact {
    font-size: 12px;
    color: #8a8071;
    line-height: 1.5;
}

/* Detail rows (per-event text list) */
.health-detail-lines {
    display: flex;
    flex-direction: column;
    gap: 2px;
    border-left: 2px solid #c9bfa9;
    padding-left: 10px;
}
.health-detail-line {
    font-size: 11px;
    font-family: ui-monospace, 'SF Mono', monospace;
    color: #6b6356;
}

/* Chart container */
.health-chart {
    width: 100%;
    height: 200px;
    margin-top: 4px;
}

/* ── Trade Latency ECharts histogram ─────────────────────────────────────── */
.latency-hist-echarts {
    width: 100%;
    height: 230px;
}

/* ── Fill Quality view toggle ─────────────────────────────────────────────── */
.fq-view-toggle {
    display: flex;
    gap: 4px;
    margin-bottom: 12px;
}
.fq-view-btn {
    padding: 4px 14px;
    border: 1px solid #c9bfa9;
    border-radius: 4px;
    background: transparent;
    color: #8a8071;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: color 0.12s, border-color 0.12s;
}
.fq-view-btn:hover { color: #1c1a17; border-color: #8a8071; }
.fq-view-btn-active {
    background: rgba(183,132,39,0.1);
    border-color: #2f6b2f;
    color: #2f6b2f;
}

/* ── Fill Quality charts ──────────────────────────────────────────────────── */
.fq-charts-wrap {
    display: flex;
    flex-direction: column;
    gap: 24px;
}
.fq-chart-section { display: flex; flex-direction: column; gap: 6px; }
.fq-chart-label {
    font-size: 11px;
    font-weight: 700;
    color: #8a8071;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.fq-chart   { width: 100%; height: 340px; }
.fq-treemap { width: 100%; height: 420px; }

/* ── Overview loading state ───────────────────────────────────────────────── */
.overview-loading {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 32px 16px;
    font-size: 14px;
    color: #8a8071;
}

/* ── Validator panel ──────────────────────────────────────────────────────── */
.validator-panel {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    overflow: hidden;
    background: #faf6ec;
    padding: 0 0 16px;
}
.validator-tabs {
    display: flex;
    gap: 4px;
    padding: 8px 16px 0;
    border-bottom: 1px solid #c9bfa9;
    flex-shrink: 0;
    background: #faf6ec;
    margin-bottom: 12px;
}
.validator-msg-count {
    color: #8a8071;
    font-weight: 400;
}

/* Single-message debugger */
.validator-single {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    padding: 0 16px;
    gap: 10px;
    overflow-y: auto;
}
.validator-input-row {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    flex-shrink: 0;
}
.validator-input {
    flex: 1;
    min-height: 90px;
    max-height: 160px;
    padding: 9px 12px;
    border: 1px solid #c9bfa9;
    border-radius: 4px;
    background: #faf6ec;
    color: #1c1a17;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
    resize: vertical;
    transition: border-color 0.12s;
    line-height: 1.5;
}
.validator-input::placeholder { color: #c9bfa9; font-size: 11px; }
.validator-input:focus { outline: none; border-color: #2f6b2f; }
.validator-validate-btn { flex-shrink: 0; align-self: flex-start; }

/* Summary bar */
.validator-summary {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 7px 12px;
    background: #faf6ec;
    border: 1px solid #c9bfa9;
    border-radius: 4px;
    font-size: 12px;
    flex-shrink: 0;
    flex-wrap: wrap;
}
.vsummary-ok   { color: #2f6b2f; font-weight: 600; }
.vsummary-err  { color: #b22222; font-weight: 700; }
.vsummary-warn { color: #b22222; font-weight: 600; }
.vsummary-chk-ok  { color: #2f6b2f; font-size: 11px; }
.vsummary-chk-err { color: #b22222; font-size: 11px; font-weight: 600; }

/* Field table */
.validator-field-table {
    border: 1px solid #c9bfa9;
    border-radius: 4px;
    overflow: hidden;
    background: #faf6ec;
    font-size: 12px;
}
.vfield-header {
    background: #ede4cf;
    font-weight: 600;
    color: #6b6356;
    font-size: 11px;
    border-bottom: 1px solid #c9bfa9;
}
.vfield-row {
    display: grid;
    grid-template-columns: 48px 160px 1fr 52px;
    gap: 6px;
    padding: 5px 10px;
    align-items: center;
    border-top: 1px solid #ede4cf;
}
.vfield-header.vfield-row { border-top: none; }
.vfield-ok   { }
.vfield-error { background: rgba(178,34,34,0.07); }
.vfield-warn  { background: rgba(183,132,39,0.07); }
.vfield-tag-num {
    color: #2f6b2f;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
}
.vfield-name  { color: #6b6356; }
.vfield-value {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    color: #1c1a17;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.vfield-status { text-align: center; }
.vstatus-ok   { color: #2f6b2f; font-weight: 700; }
.vstatus-err  { color: #b22222; font-weight: 700; }
.vstatus-warn { color: #b78427; font-weight: 700; }

/* Issue detail rows */
.vfield-issue {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 3px 10px 4px 68px;
    background: #faf6ec;
    border-top: 1px solid #ede4cf;
    font-size: 11px;
}
.vissue-rule-err  {
    display: inline-block;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.03em;
    color: #b22222;
    background: rgba(178,34,34, 0.12);
    border: 1px solid rgba(178,34,34, 0.3);
    border-radius: 4px;
    padding: 1px 5px;
    margin-right: 6px;
    white-space: nowrap;
    flex-shrink: 0;
}
.vissue-rule-warn {
    display: inline-block;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.03em;
    color: #b22222;
    background: rgba(183,132,39, 0.12);
    border: 1px solid rgba(183,132,39, 0.3);
    border-radius: 4px;
    padding: 1px 5px;
    margin-right: 6px;
    white-space: nowrap;
    flex-shrink: 0;
}
.vissue-err  { color: #b22222; }
.vissue-warn { color: #b22222; }
.vissue-hint { color: #8a8071; font-style: italic; }

/* Structural issues */
.validator-structural {
    border: 1px solid rgba(178,34,34,0.25);
    border-radius: 4px;
    background: rgba(178,34,34,0.04);
    flex-shrink: 0;
}

/* Batch view */
.validator-batch {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    padding: 0 16px;
    gap: 10px;
}
.validator-batch-toolbar {
    display: flex;
    align-items: center;
    gap: 16px;
    flex-shrink: 0;
}

/* Batch auto-summary bar */
.vbatch-summary {
    display: flex;
    align-items: center;
    gap: 20px;
    padding: 10px 14px;
    border: 1px solid #c9bfa9;
    border-radius: 6px;
    background: #faf6ec;
    flex-shrink: 0;
}
.vbatch-summary-running { font-size: 11px; color: #8a8071; font-style: italic; }
.vbatch-summary-empty   { font-size: 11px; color: #c9bfa9; }
.vbatch-summary-stat {
    display: flex;
    flex-direction: column;
    gap: 1px;
}
.vbatch-stat-value { font-size: 15px; font-weight: 700; font-variant-numeric: tabular-nums; }
.vbatch-stat-label { font-size: 10px; color: #8a8071; text-transform: uppercase; letter-spacing: 0.4px; }
.vbatch-stat-ok   .vbatch-stat-value { color: #2f6b2f; }
.vbatch-stat-err  .vbatch-stat-value { color: #b22222; }
.vbatch-stat-warn .vbatch-stat-value { color: #b78427; }

/* Error code breakdown table */
.vbatch-breakdown {
    border: 1px solid #c9bfa9;
    border-radius: 6px;
    overflow: hidden;
    flex-shrink: 0;
}
.vbatch-breakdown-header,
.vbd-row {
    display: grid;
    grid-template-columns: 110px 1fr 44px;
    align-items: center;
    gap: 8px;
    padding: 5px 10px;
}
.vbatch-breakdown-header {
    background: #faf6ec;
    border-bottom: 1px solid #c9bfa9;
    font-size: 10px;
    font-weight: 700;
    color: #8a8071;
    text-transform: uppercase;
    letter-spacing: 0.4px;
}
.vbd-row {
    border-bottom: 1px solid #c9bfa9;
    font-size: 11px;
}
.vbd-row:last-child { border-bottom: none; }
.vbd-row:hover { background: #efe8d2; }
.vbd-rule { }
.vbd-code { color: #6b6356; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 10px; }
.vbd-count { text-align: right; font-weight: 700; font-variant-numeric: tabular-nums; color: #1c1a17; }
.validator-batch-empty {
    padding: 24px;
    text-align: center;
    color: #2f6b2f;
    font-size: 13px;
    border: 1px solid rgba(47,107,47,0.2);
    border-radius: 4px;
    background: rgba(47,107,47,0.04);
}
/* Issues filter bar */
.vbatch-toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 8px;
    flex-shrink: 0;
}
.vbatch-filter-wrap {
    position: relative;
    flex: 1;
    max-width: 320px;
}
.vbatch-filter {
    width: 100%;
    background: #faf6ec;
    border: 1px solid #c9bfa9;
    border-radius: 4px;
    color: #1c1a17;
    font-size: 12px;
    padding: 5px 28px 5px 10px;
    outline: none;
}
.vbatch-filter:focus { border-color: #8a8071; }
.vbatch-filter-clear {
    position: absolute;
    right: 6px;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    color: #8a8071;
    font-size: 14px;
    cursor: pointer;
    line-height: 1;
    padding: 0 2px;
}
.vbatch-filter-clear:hover { color: #6b6356; }
.vbatch-filter-count {
    font-size: 11px;
    color: #8a8071;
    white-space: nowrap;
    margin-right: auto;
}

.validator-batch-table {
    border: 1px solid #c9bfa9;
    border-radius: 4px;
    overflow-y: auto;
    background: #faf6ec;
    flex: 1;
    min-height: 0;
}
.vbatch-header {
    background: #ede4cf;
    font-weight: 700;
    color: #3a342c;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.6px;
    border-bottom: 1px solid #c9bfa9;
    position: sticky;
    top: 0;
    z-index: 1;
}
.vbatch-row {
    display: grid;
    grid-template-columns: 70px 90px 110px 1fr;
    gap: 12px;
    padding: 7px 12px;
    align-items: center;
    font-size: 12px;
    border-top: 1px solid #ede4cf;
    border-left: 4px solid transparent;
    cursor: pointer;
    transition: background 0.1s;
    color: #1c1a17;
    min-height: 32px;
    box-sizing: border-box;
}
.vbatch-header.vbatch-row {
    border-top: none;
    border-left: 4px solid transparent;
    cursor: default;
    padding: 8px 12px;
}
.vbatch-error {
    background: rgba(178,34,34,0.08);
    border-left-color: #b22222;
}
.vbatch-warn {
    background: rgba(183,132,39,0.09);
    border-left-color: #b78427;
}
.vbatch-error:hover { background: rgba(178,34,34,0.16); }
.vbatch-warn:hover  { background: rgba(183,132,39,0.18); }
.vbatch-idx    { color: #8a8071; font-variant-numeric: tabular-nums; font-family: ui-monospace, Menlo, monospace; }
.vbatch-type   { color: #1c1a17; font-weight: 600; }
.vbatch-issues {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
}
.vbatch-issues .vstatus-err  { color: #b22222; }
.vbatch-issues .vstatus-warn { color: #b78427; }
.vbatch-first {
    color: #3a342c;
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

/* ── Stats strip (shared by timeline + detail panels) ─────────────────── */
.stats-strip {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    min-height: 36px;
    box-sizing: border-box;
    flex-wrap: wrap;          /* allow wrap when pane is narrow (compare mode) */
    flex-shrink: 0;
    min-width: 0;             /* permit child shrinking inside flex parents */
    overflow: hidden;         /* safety: clip any final overflow rather than bleed */
}
/* Long-form labels (e.g. "Skip heartbeats") shrink first so the fixed pills
   never get pushed out of the column. */
.stats-strip .check-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.stats-spacer { flex: 1; min-width: 16px; }
.stat-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.2px;
    background: rgba(47,107,47,0.10);
    color: #2f6b2f;
    border: 1px solid rgba(47,107,47,0.30);
}
.stat-pill-good { color: #2f6b2f; }
.stat-pill-num  { color: #1c1a17; }
.stat-pill-unit { color: #6b6356; font-weight: 500; }
.stat-pill-sep  { color: #8a8071; font-weight: 400; }
.stat-meta {
    color: #6b6356;
    font-size: 11px;
    letter-spacing: 0.2px;
}
.btn-icon {
    background: transparent;
    border: 1px solid #c9bfa9;
    color: #6b6356;
    font-size: 11px;
    padding: 4px 10px;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.12s;
}
.btn-icon:hover { border-color: #8a8071; color: #1c1a17; }
.btn-icon-on    { color: #2f6b2f; border-color: rgba(47,107,47,0.45); background: rgba(47,107,47,0.06); }
.btn-icon-clear:hover { color: #b22222; border-color: rgba(178,34,34,0.45); }

.stat-pill-meta {
    background: rgba(28,26,23,0.05);
    color: #6b6356;
    border-color: #c9bfa9;
}
.stat-pill-meta .stat-pill-num { color: #3a342c; }

/* ── Hero overhaul ────────────────────────────────────────────────────── */
.hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 32px 24px 8px;
    gap: 18px;
}
.hero-headline { max-width: 720px; }
.hero-title-text {
    font-size: 28px;
    font-weight: 700;
    color: #1c1a17;
    letter-spacing: -0.5px;
}
.hero-title-text span { color: #2f6b2f; font-variant-numeric: tabular-nums; }
.hero-sub {
    margin-top: 6px;
    color: #6b6356;
    font-size: 13px;
}
.hero-cta-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(180px, 240px));
    gap: 10px;
    width: 100%;
    max-width: 520px;
}
.hero-cta {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 18px 14px;
    background: #faf6ec;
    border: 1px solid #c9bfa9;
    border-radius: 6px;
    color: #1c1a17;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
}
.hero-cta:hover { border-color: #2f6b2f; transform: translateY(-1px); }
.hero-cta-primary {
    border-color: #2f6b2f;
    background: rgba(47,107,47,0.06);
}
.hero-cta-info {
    cursor: default;
    opacity: 0.7;
}
.hero-cta-info:hover { transform: none; border-color: #c9bfa9; }
.hero-cta-icon { font-size: 22px; }
.hero-cta-label { color: #1c1a17; }
.hero-cta-hint  { color: #8a8071; font-size: 11px; font-weight: 500; }

.hero-section {
    width: 100%;
    max-width: 720px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    align-items: center;
}
.hero-section-label {
    font-size: 10px;
    color: #8a8071;
    text-transform: uppercase;
    letter-spacing: 1px;
    font-weight: 700;
}
.hero-recent-list,
.hero-sample-list {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    justify-content: center;
}
.hero-recent-chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 11px;
    background: #faf6ec;
    border: 1px solid #c9bfa9;
    color: #3a342c;
    border-radius: 999px;
    font-size: 12px;
    cursor: pointer;
    transition: all 0.12s;
    max-width: 240px;
}
.hero-recent-chip:hover { border-color: #15467a; color: #15467a; }
.hero-recent-icon { font-size: 11px; }
.hero-recent-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.hero-sample-chip {
    padding: 5px 12px;
    background: transparent;
    border: 1px solid rgba(47,107,47,0.35);
    color: #2f6b2f;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.12s;
}
.hero-sample-chip:hover { background: rgba(47,107,47,0.10); border-color: #2f6b2f; }
.hero-footnote {
    color: #8a8071;
    font-size: 11px;
    margin-top: 4px;
}
.hero-footnote span { color: #6b6356; font-variant-numeric: tabular-nums; }
.fix-input-hero {
    margin-top: 18px;
    max-width: 720px;
    width: 100%;
    min-height: 80px;
    align-self: center;
}

/* ── Command palette ──────────────────────────────────────────────────── */
.palette-overlay {
    position: fixed;
    inset: 0;
    background: rgba(28,26,23,0.18);
    backdrop-filter: blur(2px);
    z-index: 200;
    display: flex;
    justify-content: center;
    padding-top: 14vh;
}
.palette-modal {
    width: min(560px, calc(100vw - 80px));
    height: fit-content;
    max-height: 60vh;
    background: #faf6ec;
    border: 1px solid #c9bfa9;
    border-radius: 6px;
    box-shadow: 0 16px 48px rgba(28,26,23,0.22);
    overflow: hidden;
    display: flex;
    flex-direction: column;
}
.palette-input {
    background: transparent;
    border: none;
    border-bottom: 1px solid #c9bfa9;
    color: #1c1a17;
    font-size: 14px;
    padding: 14px 18px;
    outline: none;
}
.palette-list {
    overflow-y: auto;
    max-height: 50vh;
    padding: 6px 0;
}
.palette-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 18px;
    color: #1c1a17;
    font-size: 13px;
    cursor: pointer;
    gap: 16px;
}
.palette-item-active { background: rgba(47,107,47,0.10); }
.palette-item-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.palette-item-hint {
    color: #8a8071;
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 220px;
}
.palette-empty {
    padding: 18px;
    color: #8a8071;
    text-align: center;
    font-size: 12px;
}

/* ── Compare picker (dropdown when 3+ tabs) ───────────────────────────── */
.compare-picker-wrap { position: relative; display: inline-flex; }
.cmdbar-vs-name { color: #b78427; font-weight: 600; }
.compare-picker-overlay {
    position: fixed;
    inset: 0;
    z-index: 90;
}
.compare-picker-menu {
    position: absolute;
    top: 38px;
    right: 0;
    min-width: 260px;
    background: #faf6ec;
    border: 1px solid #c9bfa9;
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(28,26,23,0.18);
    padding: 6px 0;
    z-index: 91;
}
.compare-picker-label {
    padding: 6px 14px 4px;
    font-size: 10px;
    color: #8a8071;
    text-transform: uppercase;
    letter-spacing: 1px;
    font-weight: 700;
}
.compare-picker-active,
.compare-picker-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 14px;
    font-size: 13px;
}
.compare-picker-active { color: #6b6356; cursor: default; }
.compare-picker-item   { color: #1c1a17; cursor: pointer; }
.compare-picker-item:hover { background: #ede4cf; }
.compare-picker-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border-radius: 4px;
    background: #2f6b2f;
    color: #f5efe2;
    font-size: 10px;
    font-weight: 700;
}
.compare-picker-badge-b { background: #b78427; }
.compare-picker-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 220px;
}
.compare-picker-sep { height: 1px; background: #c9bfa9; margin: 4px 0; }

/* ── Tab context menu ─────────────────────────────────────────────────── */
.tab-menu-overlay {
    position: fixed;
    inset: 0;
    z-index: 80;
}
.tab-menu {
    position: absolute;
    min-width: 200px;
    background: #faf6ec;
    border: 1px solid #c9bfa9;
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(28,26,23,0.18);
    padding: 4px 0;
    z-index: 81;
}
.tab-menu-item {
    padding: 7px 14px;
    color: #1c1a17;
    font-size: 13px;
    cursor: pointer;
}
.tab-menu-item:hover { background: #ede4cf; }
.tab-menu-item-disabled { color: #8a8071; cursor: default; pointer-events: none; }
.tab-menu-sep { height: 1px; background: #c9bfa9; margin: 4px 0; }

/* ── Pane visibility toggles ──────────────────────────────────────────── */
.panels-no-detail .panel-detail { display: none; }
.panels-no-detail .panel-timeline { flex: 1; }
.panels-no-timeline .panel-timeline { display: none; }
.panels-no-timeline .panel-detail { flex: 1; }
.empty-state-ghost {
    color: #8a8071;
    font-size: 12px;
    padding: 24px 16px;
    text-align: center;
    border: 1px dashed #c9bfa9;
    border-radius: 6px;
    margin: 12px 0;
}

/* ── Detail strip + segmented view tabs (native macOS feel) ───────────── */
.detail-strip {
    /* Inherits padding + min-height from .stats-strip so the timeline and detail
       top rows align pixel-for-pixel side-by-side. */
}
.panel-tag {
    font-size: 10px;
    font-weight: 700;
    color: #8a8071;
    text-transform: uppercase;
    letter-spacing: 1.5px;
    margin-right: 4px;
}
.seg-tabs {
    display: inline-flex;
    background: #ede4cf;
    border: 1px solid #c9bfa9;
    border-radius: 6px;
    padding: 2px;
    gap: 0;
}
.seg-tab {
    background: transparent;
    border: none;
    color: #6b6356;
    font-size: 11px;
    font-weight: 600;
    padding: 3px 12px;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.1s;
    line-height: 1.4;
}
.seg-tab:hover { color: #1c1a17; }
.seg-tab-active {
    background: #faf6ec;
    color: #1c1a17;
    box-shadow: 0 1px 2px rgba(28,26,23,0.10);
}

/* ── Pasted-FIX banner (sample / paste with no file) ──────────────────── */
.fix-file-banner-pasted {
    background: rgba(21,70,122,0.04);
    border-color: rgba(21,70,122,0.30);
}
.fix-file-banner-pasted .fix-file-icon { color: #15467a; opacity: 1; }
.fix-file-meta { color: #8a8071; font-size: 11px; margin-left: 4px; }

/* ── LP Scorecard (last-look analyzer) ────────────────────────────────── */
.lp-scorecard {
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding: 12px 16px;
    overflow-y: auto;
}
.lp-section { display: flex; flex-direction: column; gap: 8px; }
.lp-section-label {
    font-size: 10px;
    font-weight: 700;
    color: #6b6356;
    text-transform: uppercase;
    letter-spacing: 1px;
}
.lp-table {
    display: flex;
    flex-direction: column;
    border: 1px solid #c9bfa9;
    border-radius: 4px;
    background: #faf6ec;
    overflow: hidden;
}
.lp-row {
    display: grid;
    grid-template-columns: 1.5fr 80px 90px 90px 90px 90px 90px 40px;
    align-items: center;
    padding: 6px 12px;
    border-top: 1px solid #ede4cf;
    font-size: 12px;
    gap: 8px;
}
.lp-row:first-child { border-top: none; }
.lp-row-header {
    background: #ede4cf;
    color: #6b6356;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.lp-row-flagged {
    background: rgba(178,34,34,0.06);
    border-left: 4px solid #b22222;
    padding-left: 8px;
}
.lp-name {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-weight: 600;
    color: #1c1a17;
}
.lp-num {
    font-variant-numeric: tabular-nums;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    color: #3a342c;
    text-align: right;
}
.lp-num-bad { color: #b22222; font-weight: 700; }
.lp-flag    { text-align: center; }
.lp-flag-on { color: #b22222; font-size: 14px; font-weight: 700; }

.lp-grid {
    display: flex;
    flex-direction: column;
    border: 1px solid #c9bfa9;
    border-radius: 4px;
    background: #faf6ec;
    overflow-x: auto;
}
.lp-grid-row {
    display: flex;
    border-top: 1px solid #ede4cf;
    align-items: stretch;
}
.lp-grid-row:first-child { border-top: none; }
.lp-grid-header { background: #ede4cf; font-weight: 700; color: #6b6356; }
.lp-grid-corner, .lp-grid-rowlabel {
    width: 140px;
    flex-shrink: 0;
    padding: 6px 10px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 11px;
    color: #1c1a17;
    font-weight: 600;
    border-right: 1px solid #ede4cf;
}
.lp-grid-cell, .lp-grid-cell-h {
    flex: 1;
    min-width: 70px;
    padding: 6px 8px;
    text-align: center;
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}
.lp-grid-cell-h {
    color: #6b6356;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.lp-grid-cell {
    color: #1c1a17;
    border-left: 1px solid rgba(201,191,169,0.5);
}

"#;
