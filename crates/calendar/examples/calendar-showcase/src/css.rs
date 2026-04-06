pub const CSS: &str = r#"
*, *::before, *::after { box-sizing: border-box; }

body {
    margin: 0;
    font-family: system-ui, -apple-system, sans-serif;
    background: #f8fafc;
    color: #1e293b;
}

/* ── Layout ─────────────────────────────────────────────────────── */

.page {
    display: flex;
    min-height: 100vh;
}

.sidebar {
    position: sticky;
    top: 0;
    width: 240px;
    height: 100vh;
    overflow-y: auto;
    padding: 24px 16px;
    background: #fff;
    border-right: 1px solid #e2e8f0;
    flex-shrink: 0;
}

.sidebar h2 {
    margin: 0 0 12px;
    font-size: 0.85rem;
    font-weight: 600;
    color: #94a3b8;
    text-transform: uppercase;
    letter-spacing: 0.05em;
}

.sidebar a {
    display: block;
    padding: 4px 8px;
    margin: 1px 0;
    font-size: 0.85rem;
    color: #475569;
    text-decoration: none;
    border-radius: 4px;
    transition: background 0.15s;
}

.sidebar a:hover {
    background: #f1f5f9;
    color: #0f172a;
}

.main {
    flex: 1;
    padding: 32px 48px;
    max-width: 960px;
}

.main > h1 {
    margin: 0 0 8px;
    font-size: 1.75rem;
}

.main > p {
    margin: 0 0 32px;
    color: #64748b;
}

/* ── Section ────────────────────────────────────────────────────── */

.section {
    margin-bottom: 48px;
    scroll-margin-top: 24px;
}

.section h2 {
    margin: 0 0 4px;
    font-size: 1.15rem;
}

.section .desc {
    margin: 0 0 16px;
    color: #64748b;
    font-size: 0.9rem;
}

.section .demo {
    padding: 24px;
    background: #fff;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
}

.output {
    margin-top: 12px;
    padding: 8px 12px;
    background: #f1f5f9;
    border-radius: 4px;
    font-size: 0.85rem;
    color: #475569;
    font-family: monospace;
}

/* ── Calendar Grid ──────────────────────────────────────────────── */

[role="application"] {
    display: inline-block;
}

[role="application"] [role="group"] {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
}

[role="application"] [role="heading"] {
    flex: 1;
    text-align: center;
    font-weight: 600;
    font-size: 0.95rem;
    cursor: pointer;
    user-select: none;
}

[role="application"] table {
    border-collapse: collapse;
}

[role="application"] th {
    padding: 4px 0;
    font-size: 0.75rem;
    font-weight: 500;
    color: #94a3b8;
    width: 36px;
    text-align: center;
}

[role="application"] td {
    padding: 1px;
}

[role="application"] td button {
    width: 36px;
    height: 36px;
    border: none;
    border-radius: 6px;
    background: transparent;
    font-size: 0.85rem;
    cursor: pointer;
    color: inherit;
    transition: background 0.1s;
    font-family: inherit;
}

[role="application"] td button:hover:not(:disabled) {
    background: #f1f5f9;
}

[role="application"] td button:focus-visible {
    outline: 2px solid #3b82f6;
    outline-offset: -2px;
}

/* Nav buttons */
[role="application"] [role="group"] button {
    width: 32px;
    height: 32px;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    background: #fff;
    cursor: pointer;
    font-size: 0.9rem;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #475569;
    transition: background 0.1s;
}

[role="application"] [role="group"] button:hover:not(:disabled) {
    background: #f1f5f9;
}

[role="application"] [role="group"] button:disabled {
    opacity: 0.3;
    cursor: default;
}

/* Select dropdowns */
[role="application"] select {
    padding: 4px 6px;
    border: 1px solid #e2e8f0;
    border-radius: 4px;
    font-size: 0.85rem;
    background: #fff;
    color: #1e293b;
}

/* ── Cell States ────────────────────────────────────────────────── */

[data-today="true"] {
    font-weight: 700;
    color: #3b82f6;
}

[data-selected="true"] {
    background: #3b82f6 !important;
    color: #fff !important;
    font-weight: 600;
}

[data-disabled="true"] td button,
td button[data-disabled="true"] {
    opacity: 0.3;
    cursor: default;
    pointer-events: none;
}

[data-unavailable="true"] {
    color: #ef4444;
    text-decoration: line-through;
}

[data-outside-month="true"] {
    opacity: 0.3;
}

[data-focused="true"]:not([data-selected="true"]) {
    background: #e0e7ff;
}

/* ── Range States ───────────────────────────────────────────────── */

[data-range-position="start"] {
    border-radius: 6px 0 0 6px !important;
    background: #3b82f6 !important;
    color: #fff !important;
}

[data-range-position="middle"] {
    border-radius: 0 !important;
    background: #dbeafe !important;
    color: #1e40af !important;
}

[data-range-position="end"] {
    border-radius: 0 6px 6px 0 !important;
    background: #3b82f6 !important;
    color: #fff !important;
}

/* ── Week Numbers ───────────────────────────────────────────────── */

td[data-week-number] {
    font-size: 0.7rem;
    color: #94a3b8;
    text-align: center;
    width: 28px;
    padding-right: 4px;
}

/* ── Year / Decade View ─────────────────────────────────────────── */

[role="application"][data-view-mode="year"] > table,
[role="application"][data-view-mode="decade"] > table {
    display: none;
}

[role="application"][data-view-mode="month"] > [aria-label="Year view"],
[role="application"][data-view-mode="month"] > [aria-label="Decade view"],
[role="application"][data-view-mode="decade"] > [aria-label="Year view"] {
    display: none;
}

[role="application"][data-view-mode="year"] > [aria-label="Decade view"] {
    display: none;
}

[aria-label="Year view"],
[aria-label="Decade view"] {
    margin-top: 8px;
}

[aria-label="Year view"] [role="row"],
[aria-label="Decade view"] [role="row"] {
    display: flex;
    gap: 4px;
    margin-bottom: 4px;
}

[data-month-cell] , [data-year-cell] {
    flex: 1;
    padding: 10px 4px;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    background: #fff;
    cursor: pointer;
    font-size: 0.85rem;
    transition: background 0.1s;
    font-family: inherit;
    color: inherit;
}

[data-month-cell]:hover, [data-year-cell]:hover {
    background: #f1f5f9;
}

[data-month-cell][data-selected="true"],
[data-year-cell][data-selected="true"] {
    background: #3b82f6 !important;
    color: #fff !important;
}

/* ── Multi-month ────────────────────────────────────────────────── */

.multi-month {
    display: flex;
    gap: 24px;
}

/* ── Date Field / Picker Segments ───────────────────────────────── */

[data-slot="date-field-input"],
[data-slot="date-picker-input"],
[data-slot="range-start-input"],
[data-slot="range-end-input"] {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 6px 10px;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    background: #fff;
    font-size: 0.9rem;
    font-family: monospace;
}

[role="spinbutton"][data-segment] {
    padding: 2px 4px;
    border-radius: 3px;
    outline: none;
    min-width: 2ch;
    text-align: center;
    cursor: text;
}

[role="spinbutton"][data-segment]:focus {
    background: #3b82f6;
    color: #fff;
}

[data-placeholder="true"] {
    color: #94a3b8;
}

/* ── Date Picker Popover ────────────────────────────────────────── */

.picker-container {
    position: relative;
    display: inline-block;
}

.picker-container [data-state="closed"][role="dialog"] {
    display: none;
}

.picker-container [data-state="open"][role="dialog"] {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 10;
    background: #fff;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 12px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

/* Trigger buttons */
.picker-trigger {
    padding: 6px 12px;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    background: #fff;
    cursor: pointer;
    font-size: 0.85rem;
    margin-left: 8px;
}

.picker-trigger:hover {
    background: #f1f5f9;
}

/* ── Presets ─────────────────────────────────────────────────────── */

.preset-sidebar {
    display: flex;
    gap: 16px;
    align-items: flex-start;
}

.preset-sidebar [role="listbox"] {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 140px;
}

.preset-sidebar [role="listbox"] button {
    padding: 6px 12px;
    border: 1px solid #e2e8f0;
    border-radius: 4px;
    background: #fff;
    cursor: pointer;
    font-size: 0.8rem;
    text-align: left;
    transition: background 0.1s;
    font-family: inherit;
    color: inherit;
}

.preset-sidebar [role="listbox"] button:hover {
    background: #f1f5f9;
}

.preset-sidebar [role="listbox"] button[data-selected="true"] {
    background: #3b82f6;
    color: #fff;
    border-color: #3b82f6;
}

/* ── Time Picker ────────────────────────────────────────────────── */

[aria-label="Time"] {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 6px 10px;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    background: #fff;
    font-family: monospace;
    font-size: 0.95rem;
}

[aria-label="Time"] [role="spinbutton"] {
    padding: 2px 4px;
    border-radius: 3px;
    outline: none;
    min-width: 2ch;
    text-align: center;
    cursor: text;
    border: none;
    background: transparent;
    font-family: inherit;
    font-size: inherit;
    color: inherit;
}

[aria-label="Time"] [role="spinbutton"]:focus {
    background: #3b82f6;
    color: #fff;
}

[data-slot="separator"] {
    color: #94a3b8;
}

[data-period] {
    padding: 2px 6px !important;
    font-size: 0.8rem !important;
    font-family: system-ui, sans-serif !important;
}

/* ── Buttons ────────────────────────────────────────────────────── */

.btn {
    padding: 6px 14px;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    background: #fff;
    cursor: pointer;
    font-size: 0.85rem;
    transition: background 0.1s;
    font-family: inherit;
    color: inherit;
}

.btn:hover {
    background: #f1f5f9;
}

.btn-row {
    display: flex;
    gap: 8px;
    margin-top: 12px;
}

/* ── Inline layout helpers ──────────────────────────────────────── */

.row {
    display: flex;
    align-items: center;
    gap: 16px;
}

.datetime-row {
    display: flex;
    align-items: flex-start;
    gap: 24px;
}
"#;
