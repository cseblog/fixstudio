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
.btn-sample { background: #bd93f9; }
.btn-sample:hover { background: #d0afff; }

/* Textarea */
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
.panel-header h2 {
    font-size: 16px;
    font-weight: 700;
    color: #f8f8f2;
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
    grid-template-columns: 100px 72px 72px 150px 1fr 160px;
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
"#;
