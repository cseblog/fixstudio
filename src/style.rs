/// Global CSS — clean charcoal + amber theme.
/// Philosophy: no gradients, no glows, one accent color, pure utility.
pub const CSS: &str = r#"
* { margin: 0; padding: 0; box-sizing: border-box; }
html, body { height: 100%; overflow: hidden; background: #0f0f11; }

.root {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    color: #dddde3;
    padding: 16px 20px;
    background: #0f0f11;
    height: 100vh;
    font-size: 13px;
    display: flex;
    flex-direction: column;
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
    border-radius: 5px;
    border: none;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.12s;
}
.btn-process { background: #b8922a; color: #0f0f11; }
.btn-process:hover { opacity: 0.85; }
.btn-clear { background: #1d1d20; color: #888890; border: 1px solid #2c2c30; }
.btn-clear:hover { color: #dddde3; border-color: #444448; }
.btn-load { background: #1d1d20; color: #888890; border: 1px solid #2c2c30; }
.btn-load:hover { color: #dddde3; border-color: #444448; }
.btn-sample { background: transparent; color: #b8922a; border: 1px solid rgba(184,146,42,0.4); }
.btn-sample:hover { background: rgba(184,146,42,0.08); }
.btn-sample-inline {
    background: transparent;
    color: #b8922a;
    padding: 4px 8px;
    font-size: 12px;
    border: none;
}
.btn-sample-inline:hover { background: rgba(184,146,42,0.08); }
.sample-label { color: #444448; font-size: 12px; margin-right: 4px; }
.sample-sep { color: #2c2c30; font-size: 12px; }
.toolbar-spacer { flex: 1; }
.update-version { color: #444448; font-size: 11px; align-self: center; }
.update-ok { color: #3d8f5a; font-size: 11px; align-self: center; }
.update-checking { color: #444448; font-size: 11px; align-self: center; font-style: italic; }
.btn-update-available {
    background: #3d8f5a;
    color: #0f0f11;
    padding: 5px 14px;
    font-size: 12px;
}
.btn-update-available:hover { opacity: 0.85; }

/* Textarea / loading placeholder */
.fix-loading {
    width: 100%;
    min-height: 110px;
    padding: 10px 12px;
    border: 1px solid #2c2c30;
    border-radius: 5px;
    background: #161618;
    color: #888890;
    font-size: 13px;
    font-weight: 600;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
}
.fix-file-banner {
    width: 100%;
    min-height: 60px;
    padding: 10px 16px;
    border: 1px solid #2c2c30;
    border-radius: 5px;
    background: #161618;
    color: #888890;
    font-size: 13px;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
    gap: 10px;
}
.fix-file-icon { font-size: 18px; }
.fix-file-name { font-weight: 700; color: #dddde3; }
.fix-file-hint { color: #444448; font-size: 12px; }
.fix-file-toggle {
    background: #1d1d20;
    border: 1px solid #2c2c30;
    border-radius: 4px;
    color: #888890;
    font-size: 11px;
    padding: 2px 8px;
    cursor: pointer;
    white-space: nowrap;
}
.fix-file-toggle:hover { border-color: #444448; color: #dddde3; }
.fix-file-list {
    width: 100%;
    background: #111113;
    border: 1px solid #2c2c30;
    border-radius: 5px;
    margin-bottom: 16px;
    max-height: 200px;
    overflow-y: auto;
    padding: 6px 0;
}
.fix-file-list-item {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 11px;
    color: #888890;
    padding: 3px 14px;
    border-bottom: 1px solid #1d1d20;
}
.fix-file-list-item:last-child { border-bottom: none; }

.fix-input {
    width: 100%;
    min-height: 110px;
    padding: 10px 12px;
    border: 1px solid #2c2c30;
    border-radius: 5px;
    background: #161618;
    color: #dddde3;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
    resize: vertical;
    margin-bottom: 16px;
    transition: border-color 0.12s;
}
.fix-input::placeholder { color: #444448; }
.fix-input:focus { outline: none; border-color: #b8922a; }

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
.latency-panel > .panel-header { padding-bottom: 4px; border-bottom: 1px solid #1e1e22; }
.panel-title {
    display: flex;
    align-items: baseline;
    gap: 10px;
}
.panel-header h2 {
    font-size: 16px;
    font-weight: 700;
    color: #dddde3;
}
.parse-stats {
    font-size: 11px;
    color: #444448;
}
.filter-count {
    font-size: 11px;
    color: #888890;
}
.cap-notice {
    padding: 10px 20px;
    text-align: center;
    font-size: 11px;
    color: #888890;
    border-top: 1px solid #2c2c30;
}

.check-label {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: #444448;
    cursor: pointer;
    transition: color 0.12s;
}
.check-label:hover { color: #dddde3; }
.check-label input {
    cursor: pointer;
    accent-color: #b8922a;
}

/* Tables */
.table-wrap {
    border: 1px solid #2c2c30;
    border-radius: 5px;
    overflow: hidden;
    background: #161618;
    display: flex;
    flex-direction: column;
}
.panel-timeline .table-wrap {
    flex: 1;
    min-height: 0;
}

.tbl-header {
    background: #1d1d20;
    font-weight: 600;
    color: #888890;
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
    border-top: 1px solid #1d1d20;
    cursor: pointer;
    transition: background 0.1s;
}
.tbl-row:hover { background: #1d1d20; }
.row-selected { background: #252528 !important; }

.tbl-timeline-row {
    display: grid;
    grid-template-columns: 200px 72px 72px 150px 1fr 160px;
    gap: 6px;
    padding: 6px 10px;
    align-items: center;
    font-size: 12px;
}

.tbl-detail-row {
    display: grid;
    grid-template-columns: 44px 140px 1fr 160px;
    gap: 6px;
    padding: 6px 10px;
    align-items: center;
    font-size: 12px;
}

.cell-time { font-variant-numeric: tabular-nums; color: #444448; }
.cell-detail { color: #444448; font-size: 11px; }
.tag-num { color: #b8922a; font-variant-numeric: tabular-nums; text-align: right; }

/* Column filters */
.tbl-filter { background: #111113; border-bottom: 1px solid #1d1d20; }

.time-filter-wrap {
    display: flex;
    align-items: center;
    gap: 3px;
    width: 100%;
}
.time-op-select {
    flex-shrink: 0;
    width: 34px;
    background: #1d1d20;
    border: 1px solid #2c2c30;
    border-radius: 3px;
    color: #888890;
    font-size: 11px;
    font-weight: 700;
    font-family: inherit;
    padding: 1px 2px;
    outline: none;
    cursor: pointer;
    text-align: center;
}
.time-op-select:focus { border-color: #b8922a; }
.col-filter {
    width: 100%;
    background: transparent;
    border: none;
    border-bottom: 1px solid transparent;
    color: #dddde3;
    font-size: 11px;
    font-family: inherit;
    padding: 2px 2px;
    outline: none;
}
.col-filter::placeholder { color: #2c2c30; }
.col-filter:focus { border-bottom-color: #b8922a; }
.btn-clear-filter {
    padding: 3px 10px;
    border-radius: 4px;
    border: 1px solid #963232;
    background: transparent;
    color: #963232;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.12s;
}
.btn-clear-filter:hover { background: rgba(150,50,50,0.1); }

.empty-state { padding: 20px; text-align: center; color: #444448; }

/* Badges */
.badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 3px;
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
}
.badge-green   { background: rgba(61,143,90,0.15);   color: #3d8f5a; }
.badge-red     { background: rgba(150,50,50,0.15);   color: #963232; }
.badge-orange  { background: rgba(176,114,48,0.15);  color: #b07230; }
.badge-gray    { background: rgba(136,136,144,0.12); color: #888890; }
.badge-blue    { background: rgba(90,143,168,0.15);  color: #5a8fa8; }
.badge-teal    { background: rgba(90,143,168,0.15);  color: #5a8fa8; }
.badge-purple  { background: rgba(184,146,42,0.15);  color: #b8922a; }
.badge-yellow  { background: rgba(184,146,42,0.15);  color: #b8922a; }
.badge-slate   { background: rgba(40,40,44,0.8);     color: #dddde3; }

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
    border-radius: 5px;
    border: 1px solid #2c2c30;
    background: transparent;
    color: #444448;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: color 0.12s, border-color 0.12s;
}
.tab-btn:hover { color: #dddde3; border-color: #444448; }
.tab-active {
    background: #1d1d20 !important;
    color: #dddde3 !important;
    border-color: #444448 !important;
}

/* Raw text view */
.raw-text-wrap {
    border: 1px solid #2c2c30;
    border-radius: 5px;
    background: #161618;
    overflow: hidden;
}
.raw-text-toolbar {
    display: flex;
    justify-content: flex-end;
    padding: 6px 10px;
    border-bottom: 1px solid #2c2c30;
    background: #1d1d20;
}
.btn-copy {
    background: #b8922a;
    color: #0f0f11;
    padding: 4px 14px;
    font-size: 12px;
}
.btn-copy:hover { opacity: 0.85; }
.btn-copied {
    background: #3d8f5a;
    color: #0f0f11;
    padding: 4px 14px;
    font-size: 12px;
}
.raw-text {
    padding: 12px 14px;
    margin: 0;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
    line-height: 1.6;
    color: #dddde3;
    white-space: pre-wrap;
    word-break: break-all;
    overflow-y: auto;
    user-select: text;
    -webkit-user-select: text;
}

/* Scrollbar */
.tbl-body::-webkit-scrollbar, .raw-text::-webkit-scrollbar { width: 6px; }
.tbl-body::-webkit-scrollbar-track, .raw-text::-webkit-scrollbar-track { background: #161618; }
.tbl-body::-webkit-scrollbar-thumb, .raw-text::-webkit-scrollbar-thumb { background: #2c2c30; border-radius: 3px; }
.tbl-body::-webkit-scrollbar-thumb:hover, .raw-text::-webkit-scrollbar-thumb:hover { background: #444448; }

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
    color: #dddde3;
    letter-spacing: -0.5px;
    margin-bottom: 7px;
}
.hero-title p { font-size: 13px; color: #444448; letter-spacing: 0.2px; }

/* Stat cards */
.hero-stats { display: flex; gap: 16px; align-items: stretch; }
.hero-stat {
    border: 1px solid #2c2c30;
    border-radius: 8px;
    padding: 18px 26px;
    text-align: center;
    min-width: 140px;
    background: #161618;
    animation: hero-fade-up 0.35s ease both;
}
.hero-stat-a { animation-delay: 0.08s; }
.hero-stat-b { animation-delay: 0.20s; }
.hero-stat-featured {
    padding: 22px 34px;
    border-color: rgba(184,146,42,0.35);
}
.hero-stat-value {
    font-size: 34px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    letter-spacing: -1.5px;
    line-height: 1;
    margin-bottom: 10px;
    color: #dddde3;
}
.hero-stat-featured .hero-stat-value { font-size: 40px; color: #b8922a; }
.hero-stat-a .hero-stat-value { color: #dddde3; }
.hero-stat-b .hero-stat-value { color: #dddde3; }
.hero-stat-suffix { font-size: 16px; font-weight: 700; opacity: 0.6; letter-spacing: 0; }
.hero-stat-featured .hero-stat-suffix { font-size: 20px; opacity: 0.75; }
.hero-stat-unit  { font-size: 12px; font-weight: 600; color: #444448; margin-bottom: 3px; }
.hero-stat-label { font-size: 11px; color: #2c2c30; }

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
    color: #444448;
    margin-bottom: 8px;
}
.hero-demo-time { color: #b8922a; font-weight: 700; }
.hero-bar-track {
    height: 4px;
    border-radius: 2px;
    background: #1d1d20;
    border: 1px solid #2c2c30;
    overflow: hidden;
}
.hero-bar-fill {
    height: 100%;
    border-radius: 2px;
    background: #b8922a;
    transform: scaleX(0);
    transform-origin: left;
    animation: hero-bar-grow 1.4s 0.45s cubic-bezier(0.22, 1, 0.36, 1) forwards;
}
@keyframes hero-bar-grow { to { transform: scaleX(1); } }

/* Hint line */
.hero-hint {
    font-size: 12px;
    color: #444448;
    text-align: center;
    animation: hero-fade-up 0.35s 0.35s ease both;
}
.hero-hint-kbd {
    display: inline-block;
    background: #1d1d20;
    color: #888890;
    padding: 1px 8px;
    border-radius: 3px;
    font-size: 11px;
    font-weight: 600;
    border: 1px solid #2c2c30;
}

@keyframes hero-fade-up {
    from { opacity: 0; transform: translateY(12px); }
    to   { opacity: 1; transform: translateY(0); }
}

/* Export CSV button */
.btn-export-csv {
    padding: 3px 10px;
    border-radius: 4px;
    border: 1px solid #2c2c30;
    background: #1d1d20;
    color: #888890;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: border-color 0.12s, color 0.12s;
}
.btn-export-csv:hover { border-color: #444448; color: #dddde3; }

/* Panel view tabs */
.panel-tabs {
    display: flex;
    gap: 4px;
    margin-bottom: 12px;
}
.panel-tab {
    padding: 5px 16px;
    border-radius: 5px;
    border: 1px solid #2c2c30;
    background: transparent;
    color: #444448;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: color 0.12s, border-color 0.12s;
}
.panel-tab:hover { color: #dddde3; border-color: #444448; }
.panel-tab-active {
    background: #1d1d20 !important;
    color: #dddde3 !important;
    border-color: #444448 !important;
}
.panel-tab-pro {
    color: #b8922a;
    border-color: rgba(184,146,42,0.3);
}
.panel-tab-pro:hover { background: rgba(184,146,42,0.08); color: #c9a030; }

/* Modal overlay */
.modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(8,8,10,0.82);
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
    background: #161618;
    border: 1px solid #2c2c30;
    border-radius: 8px;
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
    color: #dddde3;
}
.modal-close {
    background: transparent;
    border: none;
    color: #444448;
    font-size: 20px;
    cursor: pointer;
    padding: 0 4px;
    line-height: 1;
}
.modal-close:hover { color: #dddde3; }
.modal-desc {
    font-size: 13px;
    color: #888890;
    margin-bottom: 18px;
    line-height: 1.6;
}
.modal-label {
    display: block;
    font-size: 12px;
    font-weight: 600;
    color: #888890;
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
    background: #2c2c30;
    border-radius: 2px;
    transition: background 0.12s;
}
.resize-handle:hover .resize-handle-bar::after { background: #b8922a; }

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
    background: #161618;
    border: 1px solid #2c2c30;
    color: #444448;
    font-size: 9px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    transition: all 0.12s;
    padding: 0;
    line-height: 1;
}
.collapse-panel-btn:hover { background: #1d1d20; color: #dddde3; border-color: #444448; }

/* Features panel */
.premium-panel {
    flex-shrink: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: #111113;
    border: 1px solid #2c2c30;
    border-radius: 6px;
    overflow: hidden;
    min-height: 0;
    transition: width 0.15s ease;
}

.panel-collapse-btn {
    background: transparent;
    border: none;
    color: #444448;
    font-size: 18px;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    line-height: 1;
    transition: all 0.12s;
}
.panel-collapse-btn:hover {
    color: #dddde3;
    background: #1d1d20;
}

.premium-panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px 9px;
    border-bottom: 1px solid #2c2c30;
    flex-shrink: 0;
    background: #161618;
}
.premium-panel-title {
    font-size: 11px;
    font-weight: 700;
    color: #444448;
    letter-spacing: 0.5px;
    text-transform: uppercase;
}
.premium-panel-title-pro { color: #b8922a; }

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
.premium-panel-scroll::-webkit-scrollbar-track { background: #111113; }
.premium-panel-scroll::-webkit-scrollbar-thumb { background: #2c2c30; border-radius: 2px; }
.premium-panel-scroll::-webkit-scrollbar-thumb:hover { background: #444448; }

.feature-tier-label {
    font-size: 10px;
    font-weight: 700;
    color: #2c2c30;
    letter-spacing: 1px;
    padding: 6px 4px 2px;
    margin-top: 2px;
}
.feature-tier-label-premium { color: rgba(184,146,42,0.4); margin-top: 10px; }

.feature-card {
    background: #161618;
    border: 1px solid #2c2c30;
    border-radius: 6px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 5px;
}
.feature-card-locked { opacity: 0.45; }
.feature-card-soon {
    border-color: rgba(184,146,42,0.2);
}
.feature-card-premium {
    border-color: rgba(184,146,42,0.18);
}

.feature-card-top {
    display: flex;
    align-items: center;
    gap: 6px;
}
.feature-card-name {
    font-size: 12px;
    font-weight: 600;
    color: #dddde3;
    flex: 1;
}
.feature-badge {
    flex-shrink: 0;
    font-size: 10px;
    padding: 1px 6px;
}
.feature-card-desc {
    font-size: 11px;
    color: #444448;
    line-height: 1.5;
}
.feature-card-hint {
    font-size: 10px;
    color: #2c2c30;
    font-style: italic;
}

.btn-feature {
    padding: 4px 10px;
    border-radius: 4px;
    border: 1px solid rgba(184,146,42,0.35);
    background: transparent;
    color: #b8922a;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.12s;
    align-self: flex-start;
    margin-top: 2px;
}
.btn-feature:hover { background: rgba(184,146,42,0.08); }
.btn-feature:disabled { border-color: #2c2c30; color: #444448; cursor: not-allowed; }

.btn-feature-upgrade {
    padding: 4px 10px;
    border-radius: 4px;
    border: 1px solid rgba(184,146,42,0.35);
    background: transparent;
    color: #b8922a;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.12s;
    align-self: flex-start;
    margin-top: 2px;
}
.btn-feature-upgrade:hover { background: rgba(184,146,42,0.08); }

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
.lc-clordid { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; color: #6a7890; }
.id-clordid  { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; color: #6a7890; display: block; }
.id-quoteid  { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; color: #5a7890; font-size: 11px; }
.id-quotereqid { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; color: #4a6878; font-size: 11px; }
.id-label    { color: #444448; font-size: 10px; margin-right: 2px; margin-left: 4px; }
.lc-symbol  { color: #c8c8d0; font-weight: 600; }
.lc-side    { color: #c8c8d0; font-weight: 600; }
.lc-qty     { color: #dddde3; font-variant-numeric: tabular-nums; }
.lc-count   { color: #444448; font-weight: 700; text-align: center; }
.lc-time    { color: #444448; font-size: 11px; font-variant-numeric: tabular-nums; }
.lc-seq     { color: #444448; text-align: right; font-variant-numeric: tabular-nums; }
.lc-info    { color: #444448; font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.lc-selected-id { font-size: 12px; color: #b8922a; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
.lc-empty   { padding: 30px 20px; text-align: center; color: #444448; font-size: 13px; }

/* ── Trade Latency Analysis panel ─────────────────────────────────────────── */
.latency-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
    box-sizing: border-box;
    background: #111113;
}
.latency-header { display: flex; align-items: baseline; gap: 12px; flex-wrap: wrap; }
.latency-header-left { display: flex; flex-direction: column; gap: 3px; }
.latency-title { margin: 0; font-size: 15px; font-weight: 600; color: #c8c8d0; letter-spacing: 0.1px; }
.latency-anel-headerheader-meta { font-size: 11px; color: #2c2c30; }
.latency-section { display: flex; flex-direction: column; gap: 8px; }
.latency-section-title {
    font-size: 10px; font-weight: 600; letter-spacing: 0.8px;
    color: #2c2c30; text-transform: uppercase;
}
.latency-section-sub { font-size: 10px; color: #2c2c30; margin-top: -4px; }
.latency-chart-wrap { background: #0d0d0f; border-radius: 4px; padding: 6px; overflow: hidden; }
.latency-stat-row { display: flex; gap: 8px; flex-wrap: wrap; }
.latency-stat-item {
    flex: 1; min-width: 70px;
    background: #0d0d0f; border-radius: 4px; padding: 8px 10px;
    display: flex; flex-direction: column; align-items: center; gap: 2px;
    border-top: 2px solid #1d1d20;
}
.latency-stat-val { font-size: 15px; font-weight: 700; font-variant-numeric: tabular-nums; color: #d4d6e0; }
.latency-stat-lbl { font-size: 10px; color: #2c2c30; text-transform: uppercase; letter-spacing: 0.5px; }
.latency-stat-green  { border-color: #1d1d20; } .latency-stat-green  .latency-stat-val { color: #d4d6e0; }
.latency-stat-cyan   { border-color: #1d1d20; } .latency-stat-cyan   .latency-stat-val { color: #d4d6e0; }
.latency-stat-yellow { border-color: #9e7a28; } .latency-stat-yellow .latency-stat-val { color: #9e7a28; }
.latency-stat-orange { border-color: #9a5a28; } .latency-stat-orange .latency-stat-val { color: #9a5a28; }
.latency-stat-red    { border-color: #903030; } .latency-stat-red    .latency-stat-val { color: #903030; }
.tbl-sym-row, .tbl-slow-row {
    display: grid; align-items: center; font-size: 12px; padding: 5px 8px; gap: 6px;
}
.tbl-sym-row  { grid-template-columns: 80px 50px 48px 70px 70px 60px 60px; }
.tbl-slow-row { grid-template-columns: 130px 70px 38px 68px 68px 40px 1fr; }
.latency-tbl-body .tbl-row:nth-child(even) { background: #131315; }
.latency-tbl-body .tbl-row:hover { background: #1d1d20; }
.latency-cell-mean { color: #d4d6e0; font-variant-numeric: tabular-nums; }
.latency-cell-p95  { color: #9e7a28; font-variant-numeric: tabular-nums; }
.latency-cell-min  { color: #d4d6e0; font-variant-numeric: tabular-nums; }
.latency-cell-max  { color: #903030; font-variant-numeric: tabular-nums; }
.latency-empty {
    display: flex; flex-direction: column; align-items: center;
    justify-content: center; padding: 40px 20px; gap: 8px;
    color: #2c2c30; text-align: center;
}
.latency-empty-icon  { font-size: 40px; }
.latency-empty-title { font-size: 15px; font-weight: 600; color: #c8c8d0; margin: 0; }
.latency-empty-hint  { font-size: 12px; margin: 0; }
.latency-empty-list  { font-size: 12px; text-align: left; padding-left: 20px; }

/* Flow chart */
.flow-chart-viewport {
    position: relative;
    overflow: hidden;
    height: 220px;
    background: #111113;
    border-radius: 6px;
    cursor: grab;
    border: 1px solid #2c2c30;
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
.flow-row-clickable:hover { background: #1d1d20 !important; }
.flow-row-selected {
    background: #1e1e22 !important;
    outline: 1px solid #444448;
    cursor: pointer;
}

/* ── Phase overview ────────────────────────────────────────────────────── */
.phase-light {
    background: #161618;
    border-radius: 6px;
    padding: 14px 16px;
    border: 1px solid #2c2c30;
}
.phase-light .phase-card {
    background: #1d1d20;
    border-color: #2c2c30;
}
.phase-light .phase-card:hover { background: #222225; border-color: #444448; }
.phase-light .phase-card-active {
    background: #252528 !important;
    border-color: #444448 !important;
    border-bottom-color: transparent !important;
}
.phase-light .phase-card-label { color: #444448; }
.phase-light .phase-card-p50   { color: #d0d4f0; }
.phase-light .phase-card-sub   { color: #2c2c30; }
.phase-light .phase-card-caret { color: #2c2c30; }
.phase-light .health-green  { color: #d0d4f0; }
.phase-light .health-yellow { color: #c8a840; }
.phase-light .health-orange { color: #c07030; }
.phase-light .health-red    { color: #b03030; }
.phase-light .health-none   { color: #2c2c30; }
.phase-light .phase-detail {
    background: #1d1d20;
    border-color: #444448;
    border-top: none;
}
.phase-light .phase-detail-count { color: #c8cce8; }
.phase-light .phase-detail-hint  { color: #2c2c30; }
.phase-light .latency-chart-wrap { background: #111113; }
.phase-light .phase-stat-cell { background: #111113; border-color: #1d1d20; }
.phase-light .phase-stat-val  { color: #d0d4f0; }
.phase-light .phase-stat-lbl  { color: #2c2c30; }
.phase-light .phase-stat-green  { border-color: #1d1d20; } .phase-light .phase-stat-green  .phase-stat-val { color: #d0d4f0; }
.phase-light .phase-stat-cyan   { border-color: #1d1d20; } .phase-light .phase-stat-cyan   .phase-stat-val { color: #d0d4f0; }
.phase-light .phase-stat-yellow { border-color: #c8a840; } .phase-light .phase-stat-yellow .phase-stat-val { color: #c8a840; }
.phase-light .phase-stat-orange { border-color: #c07030; } .phase-light .phase-stat-orange .phase-stat-val { color: #c07030; }
.phase-light .phase-stat-red    { border-color: #b03030; } .phase-light .phase-stat-red    .phase-stat-val { color: #b03030; }
.phase-light .phase-stat-drilling {
    border-color: #b8922a;
    border-top-width: 3px;
}
.phase-light .phase-no-data { color: #2c2c30; }

/* ── Lifecycle Reconstructor table layout ────────────────── */
.tbl-chain-row {
    display: grid;
    grid-template-columns: 9rem 6rem 3.5rem 4rem 6rem 6rem 6rem 6rem 6rem 6rem 3.5rem;
    gap: 0;
    align-items: center;
    padding: 0 8px;
}
.recon-filter-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    margin-bottom: 8px;
}
.recon-filter-input {
    background: #0d0d0f;
    border: 1px solid #1d1d20;
    border-radius: 4px;
    color: #c8c8d0;
    font-size: 12px;
    padding: 4px 8px;
    width: 160px;
    outline: none;
}
.recon-filter-input:focus { border-color: #b8922a; }
.recon-filter-btn {
    background: #0d0d0f;
    border: 1px solid #1d1d20;
    border-radius: 4px;
    color: #444448;
    font-size: 11px;
    padding: 3px 9px;
    cursor: pointer;
    transition: all 0.08s;
}
.recon-filter-btn:hover { border-color: #444448; color: #c8c8d0; }
.recon-filter-btn-active { background: #1d1d20; color: #c8c8d0; border-color: #444448; }
.recon-more {
    color: #444448;
    font-size: 11px;
    text-align: center;
    padding: 8px 0 4px;
}
.status-filled    { color: #4a9060; font-weight: 600; font-size: 11px; }
.status-partial   { color: #9e7a28; font-weight: 600; font-size: 11px; }
.status-cancelled { color: #555558; font-size: 11px; }
.status-rejected  { color: #903030; font-weight: 600; font-size: 11px; }
.status-expired   { color: #2c2c30; font-size: 11px; }
.status-open      { color: #5a7898; font-size: 11px; }
.status-unknown   { color: #2c2c30; font-size: 11px; }
.chain-type-rfq    { color: #6a7890; font-size: 11px; font-weight: 600; }
.chain-type-direct { color: #444448; font-size: 11px; }
.lc-sort-hdr { cursor: pointer; user-select: none; transition: color 0.08s; }
.lc-sort-hdr:hover { color: #b8922a; }
.lc-sort-hdr-active { color: #c9a030; font-weight: 700; }

/* ── Phase overview: cards + detail ─────────────────────────────────────── */
.phase-overview-wrap { gap: 0; }

.phase-cards-row {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
}
.phase-card {
    background: #0d0d0f;
    border: 1px solid #1d1d20;
    border-radius: 5px;
    padding: 12px 14px 10px;
    cursor: pointer;
    transition: border-color 0.12s, background 0.12s;
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 3px;
}
.phase-card:hover { border-color: #2c2c30; background: #131315; }
.phase-card-active { border-color: #2c2c30 !important; background: #111113 !important; border-bottom-left-radius: 0 !important; border-bottom-right-radius: 0 !important; border-bottom-color: transparent !important; }
.phase-card-label { font-size: 10px; font-weight: 600; letter-spacing: 0.8px; text-transform: uppercase; color: #2c2c30; }
.phase-card-p50 { font-size: 22px; font-weight: 700; font-variant-numeric: tabular-nums; letter-spacing: -0.5px; line-height: 1.1; color: #d4d6e0; }
.phase-card-sub { font-size: 11px; color: #1d1d20; }
.phase-card-caret { position: absolute; top: 10px; right: 12px; font-size: 9px; color: #1d1d20; }
.health-green  { color: #d4d6e0; }
.health-yellow { color: #9e7a28; }
.health-orange { color: #9a5a28; }
.health-red    { color: #903030; }
.health-none   { color: #1d1d20; }
.phase-card:has(.health-yellow) { border-left: 3px solid #9e7a28; }
.phase-card:has(.health-orange) { border-left: 3px solid #9a5a28; }
.phase-card:has(.health-red)    { border-left: 3px solid #903030; }

.phase-detail {
    background: #0d0d0f;
    border: 1px solid #1d1d20;
    border-top: none;
    border-radius: 0 0 5px 5px;
    padding: 12px 12px 10px;
    display: flex;
    flex-direction: column;
    gap: 10px;
}
.phase-detail-meta { display: flex; align-items: center; gap: 12px; }
.phase-detail-count { font-size: 12px; color: #8a8c9e; font-weight: 600; }
.phase-detail-hint  { font-size: 11px; color: #1d1d20; font-style: italic; }
.phase-hist-full { width: 100%; }

.phase-stats-table { display: grid; grid-template-columns: repeat(6, 1fr); gap: 6px; }
.phase-stat-cell {
    background: #111113; border-radius: 4px; padding: 8px 10px;
    display: flex; flex-direction: column; align-items: center; gap: 2px;
    border-top: 2px solid #1d1d20;
}
.phase-stat-val { font-size: 14px; font-weight: 700; font-variant-numeric: tabular-nums; color: #d4d6e0; }
.phase-stat-lbl { font-size: 10px; color: #1d1d20; text-transform: uppercase; letter-spacing: 0.5px; }
.phase-stat-green  { border-color: #1d1d20; } .phase-stat-green  .phase-stat-val { color: #d4d6e0; }
.phase-stat-cyan   { border-color: #1d1d20; } .phase-stat-cyan   .phase-stat-val { color: #d4d6e0; }
.phase-stat-yellow { border-color: #9e7a28; } .phase-stat-yellow .phase-stat-val { color: #9e7a28; }
.phase-stat-orange { border-color: #9a5a28; } .phase-stat-orange .phase-stat-val { color: #9a5a28; }
.phase-stat-red    { border-color: #903030; } .phase-stat-red    .phase-stat-val { color: #903030; }

.phase-stat-drill {
    cursor: pointer;
    transition: border-color 0.08s;
}
.phase-stat-drill:hover {
    border-top-width: 3px;
    border-color: #444448;
}
.phase-stat-drilling {
    border-color: #b8922a;
    border-top-width: 3px;
}

.drill-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    background: rgba(40,40,46,0.6);
    border: 1px solid #2c2c30;
    border-radius: 4px;
    padding: 6px 12px;
    font-size: 12px;
    color: #888890;
    margin-bottom: 8px;
    font-variant-numeric: tabular-nums;
}
.drill-banner-clear {
    margin-left: auto;
    cursor: pointer;
    color: #444448;
    font-size: 16px;
    line-height: 1;
    padding: 0 3px;
    transition: color 0.08s;
}
.drill-banner-clear:hover { color: #963232; }

.phase-no-data { font-size: 11px; color: #2c2c30; padding: 12px 0; }

/* ── Inline chain timeline expansion ────────────────────────────────────── */
.chain-inline-expand {
    background: #0d0d0f;
    border-left: 2px solid #2c2c30;
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
    color: #444448;
    padding: 0 2px;
    user-select: none;
}
.cit-node {
    display: inline-block;
    padding: 2px 7px;
    border-radius: 3px;
    border: 1px solid currentColor;
    font-weight: 600;
    font-size: 11px;
    background: rgba(0,0,0,0.2);
}

/* ── Overview / Session Analysis ──────────────────────────────────────────── */
.overview-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: #111113;
}
.overview-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 14px 16px 10px;
    border-bottom: 1px solid #2c2c30;
    flex-shrink: 0;
    gap: 12px;
}
.overview-header-left { flex: 1; min-width: 0; }
.overview-title {
    font-size: 15px;
    font-weight: 700;
    color: #dddde3;
    margin-bottom: 4px;
}
.overview-meta {
    font-size: 11px;
    color: #444448;
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
    border-bottom: 1px solid #2c2c30;
    flex-shrink: 0;
    background: #161618;
}
.overview-tab {
    padding: 6px 14px;
    border: none;
    border-radius: 5px 5px 0 0;
    background: transparent;
    color: #444448;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: color 0.12s, border-color 0.12s;
}
.overview-tab:hover { color: #dddde3; }
.overview-tab-active {
    color: #b8922a;
    border-bottom-color: #b8922a;
    background: rgba(184,146,42,0.06);
}
.tab-badge-warn {
    display: inline-block;
    background: #963232;
    color: #dddde3;
    font-size: 10px;
    font-weight: 700;
    border-radius: 8px;
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
    color: #444448;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
.summary-pie { height: 200px; width: 100%; }
.summary-section { padding: 4px 0; }
.summary-divider {
    height: 1px;
    background: #2c2c30;
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
    color: #444448;
    font-size: 12px;
    flex-shrink: 0;
}
.summary-value { color: #dddde3; }
.summary-bold { font-weight: 700; }
.summary-mono { font-family: ui-monospace, 'SF Mono', monospace; }
.summary-session-label { color: #b8922a; font-weight: 600; }
.summary-duration { color: #444448; font-size: 12px; }
.summary-pct { color: #444448; font-size: 11px; }
.summary-pct-green { color: #3d8f5a; }
.summary-pct-warn  { color: #963232; }
.summary-warn      { color: #9e7a28; }
.summary-spike-meta { color: #444448; font-size: 11px; font-weight: 400; }
.summary-symbol {
    display: inline-block;
    font-family: ui-monospace, 'SF Mono', monospace;
    font-size: 12px;
    color: #888890;
}
.summary-symbol-count { color: #444448; font-size: 11px; }
.summary-events-header {
    font-size: 12px;
    font-weight: 700;
    color: #444448;
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
.event-warn    { color: #9e7a28; }
.event-info    { color: #5a8fa8; }
.event-ok      { color: #3d8f5a; }
.event-time { color: #444448; font-family: ui-monospace, 'SF Mono', monospace; font-size: 11px; }
.event-desc { color: #dddde3; }

/* ── Fill Quality tab ────────────────────────────────────────────────────── */
.scorecard-wrap { overflow: hidden; }
.scorecard-breadcrumb {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: #444448;
    margin-bottom: 10px;
}
.scorecard-back-btn {
    background: transparent;
    border: none;
    color: #b8922a;
    font-size: 12px;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
}
.scorecard-back-btn:hover { background: rgba(184,146,42,0.08); }
.scorecard-breadcrumb-sep { color: #2c2c30; }
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
.scorecard-row-clickable:hover > .sc-cell { background: rgba(40,40,44,0.5); }
.scorecard-header > .sc-cell {
    background: #161618;
    border-bottom: 1px solid #2c2c30;
    padding: 6px 8px;
    font-weight: 700;
    color: #444448;
    text-transform: uppercase;
    font-size: 10px;
    letter-spacing: 0.4px;
    position: sticky;
    top: 0;
    z-index: 1;
}
.sc-cell {
    padding: 5px 8px;
    border-bottom: 1px solid rgba(44,44,48,0.5);
    color: #dddde3;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.sc-header-cell { cursor: pointer; user-select: none; }
.sc-header-cell:hover { color: #dddde3; }
.sc-sorted { color: #b8922a !important; }
.sc-num  { font-family: ui-monospace, 'SF Mono', monospace; text-align: right; }
.sc-cp   { color: #888890; font-weight: 600; }
.sc-sym  { color: #b8922a; }
.sc-good { color: #3d8f5a; font-family: ui-monospace, 'SF Mono', monospace; text-align: right; }
.sc-ok   { color: #9e7a28; font-family: ui-monospace, 'SF Mono', monospace; text-align: right; }
.sc-bad  { color: #963232; font-family: ui-monospace, 'SF Mono', monospace; text-align: right; }

/* ── Health tab ──────────────────────────────────────────────────────────── */
.health-empty {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 24px 0;
    font-size: 14px;
    color: #444448;
}
.health-ok-icon { font-size: 20px; color: #3d8f5a; }
.health-list { display: flex; flex-direction: column; gap: 14px; }

/* Card */
.health-card {
    background: #161618;
    border: 1px solid #2c2c30;
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
.health-critical { color: #963232; }
.health-warning  { color: #9e7a28; }
.health-info     { color: #5a8fa8; }
.health-kind {
    font-weight: 700;
    font-size: 13px;
    color: #dddde3;
    flex-shrink: 0;
}
.health-tech-desc {
    font-size: 12px;
    color: #9e7a28;
    font-family: ui-monospace, 'SF Mono', monospace;
    flex: 1;
    min-width: 0;
}
.health-impact {
    font-size: 12px;
    color: #444448;
    line-height: 1.5;
}

/* Detail rows (per-event text list) */
.health-detail-lines {
    display: flex;
    flex-direction: column;
    gap: 2px;
    border-left: 2px solid #2c2c30;
    padding-left: 10px;
}
.health-detail-line {
    font-size: 11px;
    font-family: ui-monospace, 'SF Mono', monospace;
    color: #888890;
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
    border: 1px solid #2c2c30;
    border-radius: 4px;
    background: transparent;
    color: #444448;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: color 0.12s, border-color 0.12s;
}
.fq-view-btn:hover { color: #dddde3; border-color: #444448; }
.fq-view-btn-active {
    background: rgba(184,146,42,0.1);
    border-color: #b8922a;
    color: #b8922a;
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
    color: #444448;
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
    color: #444448;
}

/* ── Validator panel ──────────────────────────────────────────────────────── */
.validator-panel {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    overflow: hidden;
    background: #111113;
    padding: 0 0 16px;
}
.validator-tabs {
    display: flex;
    gap: 4px;
    padding: 8px 16px 0;
    border-bottom: 1px solid #2c2c30;
    flex-shrink: 0;
    background: #161618;
    margin-bottom: 12px;
}
.validator-msg-count {
    color: #444448;
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
    border: 1px solid #2c2c30;
    border-radius: 4px;
    background: #161618;
    color: #dddde3;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
    resize: vertical;
    transition: border-color 0.12s;
    line-height: 1.5;
}
.validator-input::placeholder { color: #2c2c30; font-size: 11px; }
.validator-input:focus { outline: none; border-color: #b8922a; }
.validator-validate-btn { flex-shrink: 0; align-self: flex-start; }

/* Summary bar */
.validator-summary {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 7px 12px;
    background: #161618;
    border: 1px solid #2c2c30;
    border-radius: 4px;
    font-size: 12px;
    flex-shrink: 0;
    flex-wrap: wrap;
}
.vsummary-ok   { color: #3d8f5a; font-weight: 600; }
.vsummary-err  { color: #963232; font-weight: 700; }
.vsummary-warn { color: #9e7a28; font-weight: 600; }
.vsummary-chk-ok  { color: #3d8f5a; font-size: 11px; }
.vsummary-chk-err { color: #963232; font-size: 11px; font-weight: 600; }

/* Field table */
.validator-field-table {
    border: 1px solid #2c2c30;
    border-radius: 4px;
    overflow: hidden;
    background: #161618;
    font-size: 12px;
}
.vfield-header {
    background: #1d1d20;
    font-weight: 600;
    color: #888890;
    font-size: 11px;
    border-bottom: 1px solid #2c2c30;
}
.vfield-row {
    display: grid;
    grid-template-columns: 48px 160px 1fr 52px;
    gap: 6px;
    padding: 5px 10px;
    align-items: center;
    border-top: 1px solid #1d1d20;
}
.vfield-header.vfield-row { border-top: none; }
.vfield-ok   { }
.vfield-error { background: rgba(150,50,50,0.07); }
.vfield-warn  { background: rgba(158,122,40,0.07); }
.vfield-tag-num {
    color: #b8922a;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
}
.vfield-name  { color: #888890; }
.vfield-value {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    color: #dddde3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.vfield-status { text-align: center; }
.vstatus-ok   { color: #3d8f5a; font-weight: 700; }
.vstatus-err  { color: #963232; font-weight: 700; }
.vstatus-warn { color: #9e7a28; font-weight: 700; }

/* Issue detail rows */
.vfield-issue {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 3px 10px 4px 68px;
    background: #111113;
    border-top: 1px solid #1d1d20;
    font-size: 11px;
}
.vissue-rule-err  {
    display: inline-block;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.03em;
    color: #963232;
    background: rgba(150, 50, 50, 0.12);
    border: 1px solid rgba(150, 50, 50, 0.3);
    border-radius: 3px;
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
    color: #9e7a28;
    background: rgba(158, 122, 40, 0.12);
    border: 1px solid rgba(158, 122, 40, 0.3);
    border-radius: 3px;
    padding: 1px 5px;
    margin-right: 6px;
    white-space: nowrap;
    flex-shrink: 0;
}
.vissue-err  { color: #963232; }
.vissue-warn { color: #9e7a28; }
.vissue-hint { color: #444448; font-style: italic; }

/* Structural issues */
.validator-structural {
    border: 1px solid rgba(150,50,50,0.25);
    border-radius: 4px;
    background: rgba(150,50,50,0.04);
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
    border: 1px solid #2c2c30;
    border-radius: 6px;
    background: #161618;
    flex-shrink: 0;
}
.vbatch-summary-running { font-size: 11px; color: #444448; font-style: italic; }
.vbatch-summary-empty   { font-size: 11px; color: #2c2c30; }
.vbatch-summary-stat {
    display: flex;
    flex-direction: column;
    gap: 1px;
}
.vbatch-stat-value { font-size: 15px; font-weight: 700; font-variant-numeric: tabular-nums; }
.vbatch-stat-label { font-size: 10px; color: #444448; text-transform: uppercase; letter-spacing: 0.4px; }
.vbatch-stat-ok   .vbatch-stat-value { color: #dddde3; }
.vbatch-stat-err  .vbatch-stat-value { color: #963232; }
.vbatch-stat-warn .vbatch-stat-value { color: #9e7a28; }

/* Error code breakdown table */
.vbatch-breakdown {
    border: 1px solid #2c2c30;
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
    background: #161618;
    border-bottom: 1px solid #2c2c30;
    font-size: 10px;
    font-weight: 700;
    color: #444448;
    text-transform: uppercase;
    letter-spacing: 0.4px;
}
.vbd-row {
    border-bottom: 1px solid #1e1e22;
    font-size: 11px;
}
.vbd-row:last-child { border-bottom: none; }
.vbd-row:hover { background: #1a1a1d; }
.vbd-rule { }
.vbd-code { color: #888890; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 10px; }
.vbd-count { text-align: right; font-weight: 700; font-variant-numeric: tabular-nums; color: #dddde3; }
.validator-batch-empty {
    padding: 24px;
    text-align: center;
    color: #3d8f5a;
    font-size: 13px;
    border: 1px solid rgba(61,143,90,0.2);
    border-radius: 4px;
    background: rgba(61,143,90,0.04);
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
    background: #161618;
    border: 1px solid #2c2c30;
    border-radius: 4px;
    color: #dddde3;
    font-size: 12px;
    padding: 5px 28px 5px 10px;
    outline: none;
}
.vbatch-filter:focus { border-color: #444448; }
.vbatch-filter-clear {
    position: absolute;
    right: 6px;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    color: #444448;
    font-size: 14px;
    cursor: pointer;
    line-height: 1;
    padding: 0 2px;
}
.vbatch-filter-clear:hover { color: #888890; }
.vbatch-filter-count {
    font-size: 11px;
    color: #444448;
    white-space: nowrap;
    margin-right: auto;
}

.validator-batch-table {
    border: 1px solid #2c2c30;
    border-radius: 4px;
    overflow-y: auto;
    background: #161618;
    flex: 1;
    min-height: 0;
}
.vbatch-header {
    background: #1d1d20;
    font-weight: 600;
    color: #888890;
    font-size: 11px;
    border-bottom: 1px solid #2c2c30;
}
.vbatch-row {
    display: grid;
    grid-template-columns: 60px 80px 80px 1fr;
    gap: 6px;
    padding: 6px 10px;
    align-items: center;
    font-size: 12px;
    border-top: 1px solid #1d1d20;
    cursor: pointer;
    transition: background 0.1s;
}
.vbatch-header.vbatch-row { border-top: none; cursor: default; }
.vbatch-error { background: rgba(150,50,50,0.05); }
.vbatch-warn  { background: rgba(158,122,40,0.05); }
.vbatch-error:hover { background: rgba(150,50,50,0.12); }
.vbatch-warn:hover  { background: rgba(158,122,40,0.12); }
.vbatch-idx   { color: #444448; font-variant-numeric: tabular-nums; }
.vbatch-type  { color: #b8922a; font-weight: 600; }
.vbatch-issues { display: flex; align-items: center; gap: 4px; }
.vbatch-first { color: #888890; font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

/* ── License / activation UI ─────────────────────────────────────────────── */
.feature-card-locked { opacity: 0.45; pointer-events: none; }
.license-upgrade-wrap { display: flex; flex-direction: column; gap: 8px; padding: 12px 0 4px; }
.btn-upgrade {
  background: #b8922a; color: #fff; font-weight: 700; font-size: 12px;
  border: none; border-radius: 6px; padding: 10px 14px; cursor: pointer; text-align: center;
}
.btn-upgrade:hover { background: #c9a030; }
.btn-activate-link {
  background: transparent; color: #666672; font-size: 11px;
  border: none; cursor: pointer; padding: 4px 0; text-align: center; text-decoration: underline;
}
.btn-activate-link:hover { color: #aaaabc; }
.activate-dialog {
  background: #1a1a1e; border: 1px solid #3a3a40; border-radius: 8px;
  padding: 16px; display: flex; flex-direction: column; gap: 10px; margin-top: 8px;
}
.activate-dialog-title { font-size: 13px; font-weight: 700; color: #e8e8ec; }
.activate-dialog-sub { font-size: 11px; color: #666672; margin: 0; line-height: 1.5; }
.activate-input {
  background: #111113; border: 1px solid #2c2c30; border-radius: 5px;
  padding: 8px 10px; color: #e8e8ec; font-size: 12px; font-family: monospace;
  width: 100%; box-sizing: border-box;
}
.activate-input:focus { border-color: #b8922a; outline: none; }
.activate-error { font-size: 11px; color: #e05252; margin: 0; }
.activate-dialog-actions { display: flex; gap: 8px; }
.btn-activate-confirm {
  flex: 1; background: #b8922a; color: #fff; font-weight: 600; font-size: 12px;
  border: none; border-radius: 5px; padding: 8px; cursor: pointer;
}
.btn-activate-confirm:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-activate-confirm:not(:disabled):hover { background: #c9a030; }
.btn-activate-cancel {
  background: #1e1e22; color: #888890; font-size: 12px;
  border: 1px solid #2c2c30; border-radius: 5px; padding: 8px 12px; cursor: pointer;
}
.btn-activate-cancel:hover { color: #e8e8ec; }
.license-deactivate-wrap { padding: 12px 0 4px; }
.btn-deactivate {
  background: transparent; color: #444448; font-size: 11px;
  border: none; cursor: pointer; padding: 0; text-decoration: underline;
}
.btn-deactivate:hover { color: #e05252; }
"#;
