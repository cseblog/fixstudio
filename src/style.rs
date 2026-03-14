/// Global CSS for the application (Dracula theme).
pub const CSS: &str = r#"
* { margin: 0; padding: 0; box-sizing: border-box; }
html, body { height: 100%; overflow: hidden; background: #282a36; }

.root {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    color: #f8f8f2;
    padding: 16px 20px;
    background: #282a36;
    height: 100vh;
    font-size: 13px;
    display: flex;
    flex-direction: column;
}

/* Toolbar */
.toolbar {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
}
.btn {
    padding: 7px 16px;
    border-radius: 6px;
    border: none;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    color: #282a36;
    transition: all 0.15s;
}
.btn-process { background: #50fa7b; }
.btn-process:hover { background: #69ff94; }
.btn-clear { background: #8be9fd; }
.btn-clear:hover { background: #a4f0ff; }
.btn-load { background: #ffb86c; }
.btn-load:hover { background: #ffcc8c; }
.btn-sample { background: #bd93f9; }
.btn-sample:hover { background: #d0afff; }
.btn-sample-inline {
    background: transparent;
    color: #bd93f9;
    padding: 4px 8px;
    font-size: 12px;
}
.btn-sample-inline:hover { background: rgba(189,147,249,0.2); color: #d0afff; }
.sample-label { color: #6272a4; font-size: 12px; margin-right: 4px; }
.sample-sep { color: #44475a; font-size: 12px; }
.toolbar-spacer { flex: 1; }
.update-version { color: #44475a; font-size: 11px; align-self: center; }
.update-ok { color: #50fa7b; font-size: 11px; align-self: center; }
.update-checking { color: #6272a4; font-size: 11px; align-self: center; font-style: italic; }
.btn-update-available {
    background: #50fa7b;
    color: #282a36;
    padding: 5px 14px;
    font-size: 12px;
    animation: pulse-update 2s ease-in-out infinite;
}
.btn-update-available:hover { background: #69ff94; }
@keyframes pulse-update {
    0%, 100% { box-shadow: 0 0 0 0 rgba(80,250,123,0.5); }
    50%       { box-shadow: 0 0 0 7px rgba(80,250,123,0); }
}

/* Textarea / loading placeholder */
.fix-loading {
    width: 100%;
    min-height: 110px;
    padding: 10px 12px;
    border: 1px solid #bd93f9;
    border-radius: 6px;
    background: #21222c;
    color: #bd93f9;
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
    border: 1px solid #ffb86c;
    border-radius: 6px;
    background: rgba(255,184,108,0.05);
    color: #ffb86c;
    font-size: 13px;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
    gap: 10px;
}
.fix-file-icon { font-size: 18px; }
.fix-file-name { font-weight: 700; }
.fix-file-hint { color: #6272a4; font-size: 12px; }

.fix-input {
    width: 100%;
    min-height: 110px;
    padding: 10px 12px;
    border: 1px solid #44475a;
    border-radius: 6px;
    background: #21222c;
    color: #f8f8f2;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
    resize: vertical;
    margin-bottom: 16px;
    transition: border-color 0.15s;
}
.fix-input::placeholder { color: #6272a4; }
.fix-input:focus { outline: none; border-color: #bd93f9; }

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
}
.panel-title {
    display: flex;
    align-items: baseline;
    gap: 10px;
}
.panel-header h2 {
    font-size: 16px;
    font-weight: 700;
    color: #f8f8f2;
}
.parse-stats {
    font-size: 11px;
    color: #6272a4;
}
.filter-count {
    font-size: 11px;
    color: #8be9fd;
}
.cap-notice {
    padding: 10px 20px;
    text-align: center;
    font-size: 11px;
    color: #ffb86c;
    border-top: 1px solid #44475a;
}

.check-label {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: #6272a4;
    cursor: pointer;
    transition: color 0.15s;
}
.check-label:hover { color: #f8f8f2; }
.check-label input {
    cursor: pointer;
    accent-color: #bd93f9;
}

/* Tables */
.table-wrap {
    border: 1px solid #44475a;
    border-radius: 6px;
    overflow: hidden;
    background: #21222c;
    display: flex;
    flex-direction: column;
}
.panel-timeline .table-wrap {
    flex: 1;
    min-height: 0;
}

.tbl-header {
    background: #44475a;
    font-weight: 600;
    color: #f8f8f2;
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
    border-top: 1px solid #343746;
    cursor: pointer;
    transition: background 0.12s;
}
.tbl-row:hover { background: #343746; }
.row-selected { background: #44475a !important; }

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

.cell-time { font-variant-numeric: tabular-nums; color: #6272a4; }
.cell-detail { color: #6272a4; font-size: 11px; }
.tag-num { color: #bd93f9; font-variant-numeric: tabular-nums; text-align: right; }

/* Column filters */
.tbl-filter { background: #1e1f29; border-bottom: 1px solid #343746; }

/* Time filter: operator dropdown + value input side-by-side */
.time-filter-wrap {
    display: flex;
    align-items: center;
    gap: 3px;
    width: 100%;
}
.time-op-select {
    flex-shrink: 0;
    width: 34px;
    background: #2d2f3f;
    border: 1px solid #44475a;
    border-radius: 3px;
    color: #8be9fd;
    font-size: 11px;
    font-weight: 700;
    font-family: inherit;
    padding: 1px 2px;
    outline: none;
    cursor: pointer;
    text-align: center;
}
.time-op-select:focus { border-color: #bd93f9; }
.col-filter {
    width: 100%;
    background: transparent;
    border: none;
    border-bottom: 1px solid transparent;
    color: #f8f8f2;
    font-size: 11px;
    font-family: inherit;
    padding: 2px 2px;
    outline: none;
}
.col-filter::placeholder { color: #44475a; }
.col-filter:focus { border-bottom-color: #bd93f9; }
.btn-clear-filter {
    padding: 3px 10px;
    border-radius: 4px;
    border: 1px solid #ff5555;
    background: transparent;
    color: #ff5555;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
}
.btn-clear-filter:hover { background: rgba(255,85,85,0.12); }

.empty-state { padding: 20px; text-align: center; color: #6272a4; }

/* Badges */
.badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
}
.badge-green   { background: rgba(80,250,123,0.15);  color: #50fa7b; }
.badge-red     { background: rgba(255,85,85,0.15);   color: #ff5555; }
.badge-orange  { background: rgba(255,184,108,0.15);  color: #ffb86c; }
.badge-gray    { background: rgba(98,114,164,0.2);    color: #6272a4; }
.badge-blue    { background: rgba(139,233,253,0.15);  color: #8be9fd; }
.badge-teal    { background: rgba(139,233,253,0.15);  color: #8be9fd; }
.badge-purple  { background: rgba(189,147,249,0.15);  color: #bd93f9; }
.badge-yellow  { background: rgba(241,250,140,0.15);  color: #f1fa8c; }
.badge-slate   { background: rgba(68,71,90,0.6);      color: #f8f8f2; }

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
    border-radius: 6px;
    border: 1px solid #44475a;
    background: transparent;
    color: #6272a4;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
}
.tab-btn:hover { background: #343746; color: #f8f8f2; }
.tab-active {
    background: #44475a !important;
    color: #f8f8f2 !important;
    border-color: #6272a4;
}

/* Raw text view */
.raw-text-wrap {
    border: 1px solid #44475a;
    border-radius: 6px;
    background: #21222c;
    overflow: hidden;
}
.raw-text-toolbar {
    display: flex;
    justify-content: flex-end;
    padding: 6px 10px;
    border-bottom: 1px solid #343746;
    background: #44475a;
}
.btn-copy {
    background: #bd93f9;
    color: #282a36;
    padding: 4px 14px;
    font-size: 12px;
}
.btn-copy:hover { background: #d0afff; }
.btn-copied {
    background: #50fa7b;
    color: #282a36;
    padding: 4px 14px;
    font-size: 12px;
}
.raw-text {
    padding: 12px 14px;
    margin: 0;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
    line-height: 1.6;
    color: #f8f8f2;
    white-space: pre-wrap;
    word-break: break-all;
    overflow-y: auto;
    user-select: text;
    -webkit-user-select: text;
}

/* Scrollbar */
.tbl-body::-webkit-scrollbar, .raw-text::-webkit-scrollbar { width: 6px; }
.tbl-body::-webkit-scrollbar-track, .raw-text::-webkit-scrollbar-track { background: #21222c; }
.tbl-body::-webkit-scrollbar-thumb, .raw-text::-webkit-scrollbar-thumb { background: #44475a; border-radius: 3px; }
.tbl-body::-webkit-scrollbar-thumb:hover, .raw-text::-webkit-scrollbar-thumb:hover { background: #6272a4; }

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
    position: relative;
    overflow: hidden;
}
/* Animated dual-radial ambient glow */
.hero::before {
    content: '';
    position: absolute;
    inset: 0;
    background:
        radial-gradient(ellipse 65% 55% at 28% 42%, rgba(189,147,249,0.10) 0%, transparent 60%),
        radial-gradient(ellipse 55% 45% at 72% 62%, rgba(139,233,253,0.07) 0%, transparent 60%),
        radial-gradient(ellipse 40% 35% at 50% 50%, rgba(80,250,123,0.04) 0%, transparent 55%);
    animation: hero-bg-pulse 7s ease-in-out infinite;
    pointer-events: none;
}
@keyframes hero-bg-pulse {
    0%, 100% { opacity: 0.55; }
    50%       { opacity: 1; }
}

/* Title block */
.hero-title { text-align: center; animation: hero-fade-up 0.45s ease both; }
.hero-icon {
    font-size: 46px;
    display: block;
    margin-bottom: 10px;
    animation: hero-icon-pulse 2.8s ease-in-out infinite;
}
@keyframes hero-icon-pulse {
    0%, 100% { filter: drop-shadow(0 0  8px rgba(189,147,249,0.55)); }
    50%       { filter: drop-shadow(0 0 22px rgba(189,147,249,1.00)); }
}
.hero-title h1 {
    font-size: 30px;
    font-weight: 800;
    color: #f8f8f2;
    letter-spacing: -0.5px;
    margin-bottom: 7px;
}
.hero-title p { font-size: 13px; color: #6272a4; letter-spacing: 0.3px; }

/* Stat cards */
.hero-stats { display: flex; gap: 20px; align-items: stretch; }
.hero-stat {
    border: 1px solid #44475a;
    border-radius: 12px;
    padding: 20px 28px;
    text-align: center;
    min-width: 150px;
    background: rgba(33,34,44,0.55);
    animation: hero-fade-up 0.45s ease both;
}
.hero-stat-green  { border-color: rgba(80,250,123,0.35);   animation-delay: 0.10s; }
.hero-stat-purple { border-color: rgba(189,147,249,0.50);  animation-delay: 0.18s; }
.hero-stat-cyan   { border-color: rgba(139,233,253,0.35);  animation-delay: 0.26s; }
/* Featured centre card — slightly larger, glowing */
.hero-stat-featured {
    padding: 26px 38px;
    background: rgba(189,147,249,0.06);
    box-shadow: 0 0 32px rgba(189,147,249,0.12), inset 0 0 0 1px rgba(189,147,249,0.25);
}
.hero-stat-value {
    font-size: 36px;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
    letter-spacing: -1.5px;
    line-height: 1;
    margin-bottom: 10px;
}
.hero-stat-featured .hero-stat-value { font-size: 42px; }
.hero-stat-green  .hero-stat-value { color: #50fa7b; }
.hero-stat-purple .hero-stat-value { color: #bd93f9; }
.hero-stat-cyan   .hero-stat-value { color: #8be9fd; }
.hero-stat-suffix { font-size: 18px; font-weight: 700; opacity: 0.75; letter-spacing: 0; }
.hero-stat-featured .hero-stat-suffix { font-size: 22px; }
.hero-stat-unit  { font-size: 12px; font-weight: 600; color: #6272a4; margin-bottom: 3px; }
.hero-stat-label { font-size: 11px; color: #44475a; }

/* Parse simulation bar */
.hero-demo {
    width: 100%;
    max-width: 490px;
    animation: hero-fade-up 0.45s 0.34s ease both;
}
.hero-demo-label {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: #6272a4;
    margin-bottom: 8px;
}
.hero-demo-time { color: #bd93f9; font-weight: 700; }
.hero-bar-track {
    height: 6px;
    border-radius: 3px;
    background: #1a1b26;
    border: 1px solid #44475a;
    overflow: hidden;
}
.hero-bar-fill {
    height: 100%;
    border-radius: 3px;
    background: linear-gradient(90deg, #50fa7b 0%, #8be9fd 55%, #bd93f9 100%);
    transform: scaleX(0);
    transform-origin: left;
    animation: hero-bar-grow 1.55s 0.55s cubic-bezier(0.22, 1, 0.36, 1) forwards;
}
@keyframes hero-bar-grow { to { transform: scaleX(1); } }

/* Hint line */
.hero-hint {
    font-size: 12px;
    color: #6272a4;
    text-align: center;
    animation: hero-fade-up 0.45s 0.42s ease both;
}
.hero-hint-kbd {
    display: inline-block;
    background: #44475a;
    color: #f8f8f2;
    padding: 1px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
}

@keyframes hero-fade-up {
    from { opacity: 0; transform: translateY(16px); }
    to   { opacity: 1; transform: translateY(0); }
}

/* ── Premium / License UI ─────────────────────────────────────────────── */

/* Upgrade button in toolbar */
.btn-upgrade {
    background: linear-gradient(135deg, #bd93f9 0%, #ff79c6 100%);
    color: #282a36;
    padding: 5px 14px;
    font-size: 12px;
    font-weight: 700;
    border-radius: 6px;
    animation: pulse-upgrade 3s ease-in-out infinite;
}
.btn-upgrade:hover { opacity: 0.88; }
@keyframes pulse-upgrade {
    0%, 100% { box-shadow: 0 0 0 0 rgba(189,147,249,0.45); }
    50%       { box-shadow: 0 0 0 7px rgba(189,147,249,0); }
}

/* Pro badge */
.pro-badge {
    display: inline-block;
    padding: 3px 9px;
    border-radius: 5px;
    background: linear-gradient(135deg, #bd93f9 0%, #ff79c6 100%);
    color: #282a36;
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0.5px;
    align-self: center;
    cursor: pointer;
}
.pro-badge:hover { opacity: 0.85; }

/* Export CSV button */
.btn-export {
    background: #50fa7b;
    color: #282a36;
    padding: 5px 14px;
    font-size: 12px;
    font-weight: 700;
}
.btn-export:hover { background: #69ff94; }

/* Panel view tabs (Timeline / Lifecycle) */
.panel-tabs {
    display: flex;
    gap: 4px;
    margin-bottom: 12px;
}
.panel-tab {
    padding: 5px 16px;
    border-radius: 6px;
    border: 1px solid #44475a;
    background: transparent;
    color: #6272a4;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
}
.panel-tab:hover { background: #343746; color: #f8f8f2; }
.panel-tab-active {
    background: #44475a !important;
    color: #f8f8f2 !important;
    border-color: #6272a4;
}
.panel-tab-pro {
    color: #bd93f9;
    border-color: rgba(189,147,249,0.35);
}
.panel-tab-pro:hover { background: rgba(189,147,249,0.1); color: #d0afff; }

/* Modal overlay */
.modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(20, 21, 28, 0.85);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(4px);
    animation: modal-fade-in 0.18s ease both;
}
@keyframes modal-fade-in {
    from { opacity: 0; }
    to   { opacity: 1; }
}
.modal {
    background: #282a36;
    border: 1px solid #44475a;
    border-radius: 12px;
    padding: 28px 32px;
    width: 460px;
    max-width: 90vw;
    box-shadow: 0 24px 64px rgba(0,0,0,0.6);
    animation: modal-slide-up 0.2s cubic-bezier(0.22,1,0.36,1) both;
}
@keyframes modal-slide-up {
    from { opacity: 0; transform: translateY(20px); }
    to   { opacity: 1; transform: translateY(0); }
}
.modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 20px;
}
.modal-title {
    font-size: 17px;
    font-weight: 700;
    color: #f8f8f2;
}
.modal-close {
    background: transparent;
    border: none;
    color: #6272a4;
    font-size: 20px;
    cursor: pointer;
    padding: 0 4px;
    line-height: 1;
}
.modal-close:hover { color: #f8f8f2; }
.modal-desc {
    font-size: 13px;
    color: #6272a4;
    margin-bottom: 18px;
    line-height: 1.6;
}
.modal-label {
    display: block;
    font-size: 12px;
    font-weight: 600;
    color: #8be9fd;
    margin-bottom: 6px;
}
.license-input {
    width: 100%;
    padding: 9px 12px;
    border: 1px solid #44475a;
    border-radius: 6px;
    background: #21222c;
    color: #f8f8f2;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 13px;
    outline: none;
    transition: border-color 0.15s;
    margin-bottom: 14px;
}
.license-input::placeholder { color: #44475a; }
.license-input:focus { border-color: #bd93f9; }
.modal-actions {
    display: flex;
    gap: 10px;
    align-items: center;
}
.btn-activate {
    background: linear-gradient(135deg, #bd93f9 0%, #ff79c6 100%);
    color: #282a36;
    padding: 8px 22px;
    font-size: 13px;
    font-weight: 700;
}
.btn-activate:hover { opacity: 0.88; }
.btn-activate:disabled { opacity: 0.45; cursor: not-allowed; }
.btn-buy {
    background: transparent;
    border: 1px solid #bd93f9;
    color: #bd93f9;
    padding: 7px 16px;
    font-size: 12px;
    font-weight: 600;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s;
}
.btn-buy:hover { background: rgba(189,147,249,0.12); }
.activate-status {
    font-size: 12px;
    margin-top: 10px;
    min-height: 18px;
}
.activate-ok    { color: #50fa7b; }
.activate-err   { color: #ff5555; }
.activate-wait  { color: #6272a4; font-style: italic; }
.modal-deactivate {
    margin-top: 18px;
    padding-top: 14px;
    border-top: 1px solid #44475a;
    font-size: 12px;
    color: #6272a4;
}
.btn-deactivate {
    background: transparent;
    border: 1px solid #ff5555;
    color: #ff5555;
    padding: 4px 12px;
    font-size: 11px;
    font-weight: 600;
    border-radius: 5px;
    cursor: pointer;
    margin-left: 10px;
    transition: all 0.15s;
}
.btn-deactivate:hover { background: rgba(255,85,85,0.1); }

/* ── Two-panel body layout ───────────────────────────────────────────── */

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

/* Resize handle — full-area drag bar + centered collapse buttons */
.resize-handle {
    width: 18px;
    flex-shrink: 0;
    position: relative;
    margin: 0 1px;
    cursor: col-resize;
    user-select: none;
    -webkit-user-select: none;
}
/* Visual bar only — pointer events handled by parent .resize-handle */
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
    width: 5px;
    background: #44475a;
    border-radius: 3px;
    transition: background 0.15s;
}
.resize-handle:hover .resize-handle-bar::after { background: #bd93f9; }

/* Centered collapse buttons group (above drag bar) */
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
    background: #21222c;
    border: 1px solid #44475a;
    color: #6272a4;
    font-size: 9px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    transition: all 0.15s;
    padding: 0;
    line-height: 1;
}
.collapse-panel-btn:hover { background: #44475a; color: #f8f8f2; border-color: #bd93f9; }

/* Premium panel — expanded */
.premium-panel {
    flex-shrink: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: #1e1f29;
    border: 1px solid #44475a;
    border-radius: 8px;
    overflow: hidden;
    min-height: 0;
    transition: width 0.18s ease;
}

.panel-collapse-btn {
    background: transparent;
    border: none;
    color: #6272a4;
    font-size: 18px;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    line-height: 1;
    transition: all 0.15s;
}
.panel-collapse-btn:hover {
    color: #f8f8f2;
    background: #343746;
}

/* Panel header */
.premium-panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px 9px;
    border-bottom: 1px solid #44475a;
    flex-shrink: 0;
    background: #21222c;
}
.premium-panel-title {
    font-size: 11px;
    font-weight: 700;
    color: #6272a4;
    letter-spacing: 0.5px;
    text-transform: uppercase;
}
.premium-panel-title-pro { color: #bd93f9; }

/* Upgrade CTA block */
.premium-upgrade-cta {
    padding: 12px;
    border-bottom: 1px solid #44475a;
    background: rgba(189,147,249,0.04);
    flex-shrink: 0;
}
.upgrade-cta-text {
    font-size: 11px;
    color: #6272a4;
    line-height: 1.5;
    margin-bottom: 10px;
}
.btn-upgrade-panel {
    background: linear-gradient(135deg, #bd93f9 0%, #ff79c6 100%);
    color: #282a36;
    padding: 7px 0;
    font-size: 12px;
    font-weight: 700;
    border-radius: 6px;
    width: 100%;
    margin-bottom: 6px;
    cursor: pointer;
    border: none;
    transition: opacity 0.15s;
}
.btn-upgrade-panel:hover { opacity: 0.88; }
.upgrade-cta-price {
    display: block;
    text-align: center;
    font-size: 10px;
    color: #44475a;
    margin-top: 2px;
}

/* Scrollable feature list */
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
.premium-panel-scroll::-webkit-scrollbar-track { background: #1e1f29; }
.premium-panel-scroll::-webkit-scrollbar-thumb { background: #44475a; border-radius: 2px; }
.premium-panel-scroll::-webkit-scrollbar-thumb:hover { background: #6272a4; }

/* Tier labels */
.feature-tier-label {
    font-size: 10px;
    font-weight: 700;
    color: #44475a;
    letter-spacing: 1px;
    padding: 6px 4px 2px;
    margin-top: 2px;
}
.feature-tier-label-premium { color: rgba(189,147,249,0.45); margin-top: 10px; }

/* Feature cards */
.feature-card {
    background: #282a36;
    border: 1px solid #44475a;
    border-radius: 8px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 5px;
}
.feature-card-locked { opacity: 0.55; }
.feature-card-soon {
    border-color: rgba(255,184,108,0.25);
    background: rgba(255,184,108,0.02);
}
.feature-card-premium {
    border-color: rgba(189,147,249,0.25);
    background: rgba(189,147,249,0.03);
}

.feature-card-top {
    display: flex;
    align-items: center;
    gap: 6px;
}
.feature-card-name {
    font-size: 12px;
    font-weight: 600;
    color: #f8f8f2;
    flex: 1;
}
.feature-badge {
    flex-shrink: 0;
    font-size: 10px;
    padding: 1px 6px;
}
.feature-card-desc {
    font-size: 11px;
    color: #6272a4;
    line-height: 1.5;
}
.feature-card-hint {
    font-size: 10px;
    color: #44475a;
    font-style: italic;
}

/* Feature action buttons */
.btn-feature {
    padding: 4px 10px;
    border-radius: 5px;
    border: none;
    background: #50fa7b;
    color: #282a36;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s;
    align-self: flex-start;
    margin-top: 2px;
}
.btn-feature:hover { background: #69ff94; }
.btn-feature:disabled { background: #44475a; color: #6272a4; cursor: not-allowed; }

.btn-feature-upgrade {
    padding: 4px 10px;
    border-radius: 5px;
    border: 1px solid rgba(189,147,249,0.4);
    background: transparent;
    color: #bd93f9;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
    align-self: flex-start;
    margin-top: 2px;
}
.btn-feature-upgrade:hover { background: rgba(189,147,249,0.1); }

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
.lc-clordid { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; color: #bd93f9; }
.lc-symbol  { color: #f8f8f2; font-weight: 600; }
.lc-side    { color: #8be9fd; font-weight: 600; }
.lc-qty     { color: #f8f8f2; font-variant-numeric: tabular-nums; }
.lc-count   { color: #6272a4; font-weight: 700; text-align: center; }
.lc-time    { color: #6272a4; font-size: 11px; font-variant-numeric: tabular-nums; }
.lc-seq     { color: #6272a4; text-align: right; font-variant-numeric: tabular-nums; }
.lc-info    { color: #6272a4; font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.lc-selected-id { font-size: 12px; color: #bd93f9; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
.lc-empty   { padding: 30px 20px; text-align: center; color: #6272a4; font-size: 13px; }

/* ── Trade Latency Analysis panel ─────────────────────────────────────────── */
.latency-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
    padding: 16px;
    gap: 16px;
    box-sizing: border-box;
    background: #282a36;
}
.latency-header { display: flex; align-items: baseline; gap: 12px; flex-wrap: wrap; }
.latency-header-left { display: flex; flex-direction: column; gap: 3px; }
.latency-title { margin: 0; font-size: 16px; font-weight: 700; color: #f8f8f2; letter-spacing: 0.3px; }
.latency-header-meta { font-size: 11px; color: #6272a4; }
.latency-section { display: flex; flex-direction: column; gap: 8px; }
.latency-section-title {
    font-size: 10px; font-weight: 700; letter-spacing: 1px;
    color: #6272a4; text-transform: uppercase;
}
.latency-section-sub { font-size: 10px; color: #6272a4; margin-top: -4px; }
.latency-chart-wrap { background: #1e1f29; border-radius: 6px; padding: 8px; overflow: hidden; }
.latency-stat-row { display: flex; gap: 8px; flex-wrap: wrap; }
.latency-stat-item {
    flex: 1; min-width: 70px;
    background: #1e1f29; border-radius: 6px; padding: 8px 10px;
    display: flex; flex-direction: column; align-items: center; gap: 2px;
    border-top: 2px solid transparent;
}
.latency-stat-val { font-size: 15px; font-weight: 700; font-variant-numeric: tabular-nums; }
.latency-stat-lbl { font-size: 10px; color: #6272a4; text-transform: uppercase; letter-spacing: 0.5px; }
.latency-stat-green  { border-color: #50fa7b; } .latency-stat-green  .latency-stat-val { color: #50fa7b; }
.latency-stat-cyan   { border-color: #8be9fd; } .latency-stat-cyan   .latency-stat-val { color: #8be9fd; }
.latency-stat-yellow { border-color: #f1fa8c; } .latency-stat-yellow .latency-stat-val { color: #f1fa8c; }
.latency-stat-orange { border-color: #ffb86c; } .latency-stat-orange .latency-stat-val { color: #ffb86c; }
.latency-stat-red    { border-color: #ff5555; } .latency-stat-red    .latency-stat-val { color: #ff5555; }
.tbl-sym-row, .tbl-slow-row {
    display: grid; align-items: center; font-size: 12px; padding: 5px 8px; gap: 6px;
}
.tbl-sym-row  { grid-template-columns: 80px 50px 48px 70px 70px 60px 60px; }
.tbl-slow-row { grid-template-columns: 130px 70px 38px 68px 68px 40px 1fr; }
.latency-tbl-body .tbl-row:nth-child(even) { background: #2a2c3a; }
.latency-tbl-body .tbl-row:hover { background: #383a4a; }
.latency-cell-mean { color: #8be9fd; font-variant-numeric: tabular-nums; }
.latency-cell-p95  { color: #f1fa8c; font-variant-numeric: tabular-nums; }
.latency-cell-min  { color: #50fa7b; font-variant-numeric: tabular-nums; }
.latency-cell-max  { color: #ff5555; font-variant-numeric: tabular-nums; }
.latency-empty {
    display: flex; flex-direction: column; align-items: center;
    justify-content: center; padding: 40px 20px; gap: 8px;
    color: #6272a4; text-align: center;
}
.latency-empty-icon  { font-size: 40px; }
.latency-empty-title { font-size: 15px; font-weight: 600; color: #f8f8f2; margin: 0; }
.latency-empty-hint  { font-size: 12px; margin: 0; }
.latency-empty-list  { font-size: 12px; text-align: left; padding-left: 20px; }

/* Flow chart */
.flow-chart-viewport {
    position: relative;
    overflow: hidden;
    height: 220px;
    background: #1a1b26;
    border-radius: 8px;
    cursor: grab;
    border: 1px solid #44475a;
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
    transition: background 0.1s;
}
.flow-row-clickable:hover { background: #383a4a !important; }
.flow-row-selected {
    background: #2d2f45 !important;
    outline: 1px solid #bd93f9;
    cursor: pointer;
}
"#;
