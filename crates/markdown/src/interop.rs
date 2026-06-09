use dioxus::document::Eval;
use dioxus::prelude::document;

/// Adapter boundary for caret/selection interop.
///
/// All DOM/eval behavior should be expressed here, and consumers should call
/// helper functions in this module instead of `document::eval()` directly.
pub trait CaretAdapter: Send + Sync {
    /// JS to read `[selectionStart, selectionEnd]` from a textarea.
    fn read_textarea_selection_js(&self, editor_id: &str) -> String;
    /// JS to read `selectionStart` from a textarea.
    fn read_textarea_cursor_js(&self, editor_id: &str) -> String;
    /// JS to compute UTF-16 offset from block start to current DOM selection.
    fn read_block_visual_offset_js(&self, block_id: &str) -> String;
    /// JS to read contenteditable cursor selection as UTF-16 offset.
    fn read_contenteditable_selection_js(&self, block_id: &str) -> String;
    /// JS to read contenteditable selection details as compact string:
    /// `start<US>end<US>collapsed`.
    fn read_contenteditable_selection_detailed_js(&self, block_id: &str) -> String;
    /// JS to read cached beforeinput metadata as compact string:
    /// `start<US>end<US>collapsed<US>inputType<US>data`.
    fn read_contenteditable_beforeinput_meta_js(&self, block_id: &str) -> String;
    /// JS to focus and set textarea selection, with hydration-safe retries.
    fn mount_active_textarea_js(&self, textarea_id: &str, cursor_utf16: usize) -> String;
    /// JS to read contenteditable plain text.
    fn read_contenteditable_text_js(&self, block_id: &str) -> String;
    /// JS to place caret in a contenteditable block by UTF-16 code-unit index.
    fn set_contenteditable_selection_js(&self, block_id: &str, raw_utf16: usize) -> String;
    /// JS to restore a non-collapsed selection in a contenteditable block by
    /// visible UTF-16 offsets `[start, end]`.
    fn set_contenteditable_selection_range_js(
        &self,
        block_id: &str,
        start_utf16: usize,
        end_utf16: usize,
    ) -> String;
    /// JS hook for contenteditable input/traversal behavior.
    fn bind_contenteditable_input_js(&self, block_id: &str) -> String;
    /// JS to attempt a one-visual-row vertical caret move inside a contenteditable
    /// block using DOM line geometry. `going_up` selects the direction; `goal_x` is
    /// the desired horizontal caret position in viewport CSS px (or a negative value
    /// to use the caret's live x). Sends back one of:
    /// - `"none"` — no element/selection to act on.
    /// - `"moved:<visibleUtf16>:<x>"` — caret moved one visual row within the block.
    /// - `"escape:<visibleUtf16>:<x>"` — caret is on the boundary visual row in the
    ///   press direction; the caller hops to the adjacent block at column `<x>`.
    fn vertical_caret_move_js(&self, block_id: &str, going_up: bool, goal_x: f64) -> String;
}

#[derive(Debug, Default)]
pub struct WebviewCaretAdapter;

impl CaretAdapter for WebviewCaretAdapter {
    fn read_textarea_selection_js(&self, editor_id: &str) -> String {
        format!(
            "var el = document.getElementById('{editor_id}');\
             if(el) dioxus.send([el.selectionStart ?? 0, el.selectionEnd ?? 0]);\
             else dioxus.send([0, 0]);"
        )
    }

    fn read_textarea_cursor_js(&self, editor_id: &str) -> String {
        format!(
            "var el = document.getElementById('{editor_id}');\
             if(el) dioxus.send(el.selectionStart ?? 0);\
             else dioxus.send(0);"
        )
    }

    fn read_block_visual_offset_js(&self, block_id: &str) -> String {
        format!(
            r#"(function() {{
    var el = document.getElementById('{block_id}');
    if (!el) {{ dioxus.send("0"); return; }}
    var sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) {{ dioxus.send("0"); return; }}
    var range = sel.getRangeAt(0);
    var pre = range.cloneRange();
    pre.selectNodeContents(el);
    pre.setEnd(range.endContainer, range.endOffset);
    dioxus.send(pre.toString().length.toString());
}})();"#
        )
    }

    fn mount_active_textarea_js(&self, textarea_id: &str, cursor_utf16: usize) -> String {
        format!(
            r#"(function() {{
    var el = document.getElementById('{textarea_id}');
    if (!el) return;
    var tryFocus = function(attempts) {{
        if (attempts > 10) return;
        if (el.value.length === 0 && {cursor_utf16} > 0) {{
            setTimeout(function() {{ tryFocus(attempts + 1); }}, 10);
            return;
        }}
        el.focus();
        try {{
            el.setSelectionRange({cursor_utf16}, {cursor_utf16});
        }} catch (e) {{}}
        var resize = function() {{
            el.style.height = 'auto';
            el.style.height = el.scrollHeight + 'px';
        }};
        resize();
        if (!el._noxResizeBound) {{
            el.addEventListener('input', resize);
            el._noxResizeBound = true;
        }}
        if (!el._noxTraversalBound) {{
            el.addEventListener('keydown', function(e) {{
                if (e.key === 'ArrowUp') {{
                    var pos = el.selectionStart;
                    var text = el.value;
                    var isFirstLine = text.lastIndexOf('\n', pos - 1) === -1;
                    if (isFirstLine) {{
                        e.preventDefault();
                        dioxus.send("prev");
                    }}
                }} else if (e.key === 'ArrowDown') {{
                    var pos = el.selectionStart;
                    var text = el.value;
                    var isLastLine = text.indexOf('\n', pos) === -1;
                    if (isLastLine) {{
                        e.preventDefault();
                        dioxus.send("next");
                    }}
                }} else if (e.key === 'Backspace') {{
                    var pos = el.selectionStart;
                    if (pos === 0 && el.selectionStart === el.selectionEnd) {{
                        e.preventDefault();
                        dioxus.send("backjoin");
                    }}
                }}
                // Enter is intentionally NOT intercepted: this textarea backs a fenced
                // code block, where Enter must insert a literal newline within the block
                // (native textarea behavior), not split it into two paragraphs.
            }});
            el._noxTraversalBound = true;
        }}
    }};
    tryFocus(0);
}})();"#
        )
    }

    fn read_contenteditable_selection_js(&self, block_id: &str) -> String {
        self.read_block_visual_offset_js(block_id)
    }

    fn read_contenteditable_selection_detailed_js(&self, block_id: &str) -> String {
        format!(
            r#"(function() {{
    var root = document.getElementById('{block_id}');
    if (!root) {{ dioxus.send("0\u001f0\u001f1"); return; }}
    var sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) {{ dioxus.send("0\u001f0\u001f1"); return; }}
    var range = sel.getRangeAt(0);
    var toOffset = function(node, offset) {{
        if (!node || !root.contains(node)) return 0;
        try {{
            var r = document.createRange();
            r.selectNodeContents(root);
            r.setEnd(node, offset);
            return r.toString().length;
        }} catch (_e) {{
            return 0;
        }}
    }};
    var start = toOffset(range.startContainer, range.startOffset);
    var end = toOffset(range.endContainer, range.endOffset);
    var collapsed = start === end ? "1" : "0";
    dioxus.send(start.toString() + "\u001f" + end.toString() + "\u001f" + collapsed);
}})();"#
        )
    }

    fn read_contenteditable_beforeinput_meta_js(&self, block_id: &str) -> String {
        format!(
            r#"(function() {{
    var root = document.getElementById('{block_id}');
    if (!root || typeof root._noxBeforeInputMeta !== "string") {{
        dioxus.send("");
        return;
    }}
    // Non-destructive: do NOT null the slot here. Each `beforeinput` overwrites it,
    // so the slot always reflects the latest edit; reading it destructively let a
    // stale (superseded) sync consume the metadata before the surviving latest sync
    // could read it, which dropped the latest edit onto the lossy diff fallback.
    dioxus.send(root._noxBeforeInputMeta);
}})();"#
        )
    }

    fn read_contenteditable_text_js(&self, block_id: &str) -> String {
        // Use `textContent`, not `innerText`: every caret/selection offset in this
        // module is measured with `range.toString().length` / `node.nodeValue.length`,
        // which share `textContent` semantics. `innerText` is layout-aware (collapses
        // whitespace, emits "\n" for <br>/block boundaries, forces reflow) and would
        // put the text read in a different coordinate space than the offsets, corrupting
        // edit reconstruction. Hidden markers are excluded from the rendered DOM, so
        // `textContent` equals the model's `visible_text`.
        format!(
            "var el = document.getElementById('{block_id}');\
             if(el) dioxus.send(el.textContent ?? '');\
             else dioxus.send('');"
        )
    }

    fn set_contenteditable_selection_js(&self, block_id: &str, raw_utf16: usize) -> String {
        format!(
            r#"(function() {{
    var root = document.getElementById('{block_id}');
    if (!root) return;
    root.focus();
    var walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    var remaining = {raw_utf16};
    var node = null;
    while ((node = walker.nextNode())) {{
        var len = (node.nodeValue || '').length;
        if (remaining <= len) {{
            try {{
                var range = document.createRange();
                range.setStart(node, remaining);
                range.collapse(true);
                var sel = window.getSelection();
                sel.removeAllRanges();
                sel.addRange(range);
            }} catch (e) {{}}
            return;
        }}
        remaining -= len;
    }}
}})();"#
        )
    }

    fn set_contenteditable_selection_range_js(
        &self,
        block_id: &str,
        start_utf16: usize,
        end_utf16: usize,
    ) -> String {
        format!(
            r#"(function() {{
    var root = document.getElementById('{block_id}');
    if (!root) return;
    root.focus();
    function findPos(target) {{
        var walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
        var remaining = target;
        var node = null;
        while ((node = walker.nextNode())) {{
            var len = (node.nodeValue || '').length;
            if (remaining <= len) return {{ node: node, offset: remaining }};
            remaining -= len;
        }}
        return null;
    }}
    var s = findPos({start_utf16});
    var e = findPos({end_utf16});
    if (!s || !e) return;
    try {{
        var range = document.createRange();
        range.setStart(s.node, s.offset);
        range.setEnd(e.node, e.offset);
        var sel = window.getSelection();
        sel.removeAllRanges();
        sel.addRange(range);
    }} catch (ex) {{}}
}})();"#
        )
    }

    fn bind_contenteditable_input_js(&self, block_id: &str) -> String {
        format!(
            r#"(function() {{
    var root = document.getElementById('{block_id}');
    if (!root) {{ dioxus.send("missing"); return; }}
    if (root._noxBeforeInputBound) {{ dioxus.send("bound"); return; }}
    root._noxBeforeInputMeta = null;

    // Shared DOM-node → visible-UTF-16-offset converter. Returns -1 when the node
    // is outside this editing host so callers can distinguish "no offset".
    var toOffset = function(node, offset) {{
        if (!node || !root.contains(node)) return -1;
        try {{
            var r = document.createRange();
            r.selectNodeContents(root);
            r.setEnd(node, offset);
            return r.toString().length;
        }} catch (_e) {{
            return -1;
        }}
    }};

    var selectionDetails = function() {{
        var sel = window.getSelection();
        if (!sel || sel.rangeCount === 0) {{
            return {{ start: 0, end: 0, collapsed: true }};
        }}
        var range = sel.getRangeAt(0);
        var start = Math.max(0, toOffset(range.startContainer, range.startOffset));
        var end = Math.max(0, toOffset(range.endContainer, range.endOffset));
        if (end < start) {{
            var t = start; start = end; end = t;
        }}
        return {{ start: start, end: end, collapsed: start === end }};
    }};

    root.addEventListener('beforeinput', function(e) {{
        var sel = selectionDetails();
        var inputType = (e && typeof e.inputType === 'string') ? e.inputType : '';
        var rawData = (e && typeof e.data === 'string') ? e.data : '';
        // Defensively drop any U+001F from composed text: it is the meta-string
        // field separator, and a stray one would desync the Rust splitn(7) parse
        // (shifting the trailing targetStart/targetEnd fields). Browsers do not
        // emit U+001F in `data`, so this strip is a no-op in practice.
        var data = rawData.split('\u001f').join('');
        // getTargetRanges() is the exact range the browser will modify - the
        // authoritative span for word/line deletes (deleteWord*, deleteSoftLine*,
        // deleteHardLine*), where the selection is collapsed but more than one
        // character is removed. Converting it to visible UTF-16 offsets lets Rust
        // splice the orphaned hidden delimiters of an emptied inline span (#87).
        var tStart = -1, tEnd = -1;
        try {{
            var ranges = (e && typeof e.getTargetRanges === 'function')
                ? e.getTargetRanges() : null;
            if (ranges && ranges.length > 0) {{
                var tr = ranges[0];
                var a = toOffset(tr.startContainer, tr.startOffset);
                var b = toOffset(tr.endContainer, tr.endOffset);
                if (a >= 0 && b >= 0) {{
                    tStart = Math.min(a, b);
                    tEnd = Math.max(a, b);
                }}
            }}
        }} catch (_e) {{}}
        root._noxBeforeInputMeta =
            sel.start.toString() + '\u001f' +
            sel.end.toString() + '\u001f' +
            (sel.collapsed ? '1' : '0') + '\u001f' +
            inputType + '\u001f' +
            data + '\u001f' +
            tStart.toString() + '\u001f' +
            tEnd.toString();
    }});

    root._noxBeforeInputBound = true;
    dioxus.send("bound");
}})();"#
        )
    }

    fn vertical_caret_move_js(&self, block_id: &str, going_up: bool, goal_x: f64) -> String {
        // document::eval is Dioxus-native (NOT web_sys): all DOM access here goes
        // through the framework's eval channel, so this works on every webview
        // target. Confirmed no Dioxus 0.7 native API exposes caret line-geometry
        // (getClientRects on a Range) or hit-testing (caretRangeFromPoint /
        // caretPositionFromPoint) as of 2026-06 — these are required to detect
        // visual-row boundaries and to move the caret by one visual row inside a
        // soft-wrapped/multi-line block. `NoopCaretAdapter` is the non-web fallback.
        let going_up_js = if going_up { "true" } else { "false" };
        format!(
            r#"(function() {{
    try {{
        var root = document.getElementById('{block_id}');
        if (!root) {{ dioxus.send("none"); return; }}
        var sel = window.getSelection();
        if (!sel || sel.rangeCount === 0) {{ dioxus.send("none"); return; }}
        var range = sel.getRangeAt(0);
        var goingUp = {going_up_js};
        var goalX = {goal_x};
        var rootRect = root.getBoundingClientRect();

        // Visible UTF-16 offset from block start to the caret (textContent space).
        var pre = document.createRange();
        pre.selectNodeContents(root);
        pre.setEnd(range.endContainer, range.endOffset);
        var caretVisible = pre.toString().length;

        // Last client rect of a range, falling back to its bounding box when
        // getClientRects() is empty (e.g. a collapsed caret between nodes).
        var caretRect = function(r) {{
            var rects = r.getClientRects();
            if (rects && rects.length) {{ return rects[rects.length - 1]; }}
            var b = r.getBoundingClientRect();
            if (b && (b.width || b.height || b.top || b.left)) {{ return b; }}
            return null;
        }};

        var full = document.createRange();
        full.selectNodeContents(root);
        var fr = full.getClientRects();
        var firstTop = fr.length ? fr[0].top : rootRect.top;
        var lastBottom = fr.length ? fr[fr.length - 1].bottom : rootRect.bottom;

        var cr = caretRect(range);
        var lineH = cr ? (cr.bottom - cr.top)
            : (fr.length ? (fr[0].bottom - fr[0].top) : 16);
        var caretTop = cr ? cr.top : firstTop;
        var caretBottom = cr ? cr.bottom : lastBottom;
        var caretMidY = (caretTop + caretBottom) / 2;
        var caretX = (goalX >= 0) ? goalX : (cr ? cr.left : rootRect.left);

        // Row-relative visible column for an escape. `caretVisible` is measured from
        // the BLOCK start, but the adjacent-block seeding in Rust treats the value as
        // a column within the target block's first row. For a multi-line block,
        // escaping from a non-first visual row would otherwise seed a too-large column.
        // Hit-test this row's left edge (rootRect.left, caretMidY) to find the visible
        // offset where the caret's row begins, then subtract. Defensive: any failure or
        // out-of-host hit falls back to the block-start offset so escape never breaks.
        var escapeColumn = caretVisible;
        try {{
            var rowStart = null;
            if (document.caretRangeFromPoint) {{
                rowStart = document.caretRangeFromPoint(rootRect.left, caretMidY);
            }} else if (document.caretPositionFromPoint) {{
                var rcp = document.caretPositionFromPoint(rootRect.left, caretMidY);
                if (rcp && rcp.offsetNode) {{
                    rowStart = document.createRange();
                    rowStart.setStart(rcp.offsetNode, rcp.offset);
                }}
            }}
            if (rowStart && root.contains(rowStart.startContainer)) {{
                var rs = document.createRange();
                rs.selectNodeContents(root);
                rs.setEnd(rowStart.startContainer, rowStart.startOffset);
                var rowStartVisible = rs.toString().length;
                escapeColumn = Math.max(0, caretVisible - rowStartVisible);
            }}
        }} catch (_e) {{
            escapeColumn = caretVisible;
        }}

        // Boundary detection: caret sits on the first/last visual row when its
        // top/bottom is within half a line height of the block's first/last row.
        var isFirst = (caretTop - firstTop) < lineH * 0.5;
        var isLast = (lastBottom - caretBottom) < lineH * 0.5;

        if (goingUp && isFirst) {{
            dioxus.send("escape:" + escapeColumn + ":" + Math.round(caretX));
            return;
        }}
        if (!goingUp && isLast) {{
            dioxus.send("escape:" + escapeColumn + ":" + Math.round(caretX));
            return;
        }}

        // Hit-test one visual row above/below at the goal x.
        var targetY = goingUp ? (caretTop - lineH * 0.5) : (caretBottom + lineH * 0.5);
        var pos = null;
        if (document.caretRangeFromPoint) {{
            pos = document.caretRangeFromPoint(caretX, targetY);
        }} else if (document.caretPositionFromPoint) {{
            var cp = document.caretPositionFromPoint(caretX, targetY);
            if (cp && cp.offsetNode) {{
                pos = document.createRange();
                pos.setStart(cp.offsetNode, cp.offset);
            }}
        }}
        // Defensive: a hit-test that misses or lands outside this editing host
        // escapes rather than risk placing the caret in a sibling block. Seed the
        // row-relative column (same rationale as the boundary escapes above).
        if (!pos || !root.contains(pos.startContainer)) {{
            dioxus.send("escape:" + escapeColumn + ":" + Math.round(caretX));
            return;
        }}

        pos.collapse(true);
        sel.removeAllRanges();
        sel.addRange(pos);
        var post = document.createRange();
        post.selectNodeContents(root);
        post.setEnd(pos.startContainer, pos.startOffset);
        var newVisible = post.toString().length;
        dioxus.send("moved:" + newVisible + ":" + Math.round(caretX));
    }} catch (_e) {{
        dioxus.send("none");
    }}
}})();"#
        )
    }
}

#[derive(Debug, Default)]
pub struct NoopCaretAdapter;

impl CaretAdapter for NoopCaretAdapter {
    fn read_textarea_selection_js(&self, _editor_id: &str) -> String {
        "dioxus.send([0, 0]);".to_string()
    }

    fn read_textarea_cursor_js(&self, _editor_id: &str) -> String {
        "dioxus.send(0);".to_string()
    }

    fn read_block_visual_offset_js(&self, _block_id: &str) -> String {
        "dioxus.send(\"0\");".to_string()
    }

    fn mount_active_textarea_js(&self, _textarea_id: &str, _cursor_utf16: usize) -> String {
        "dioxus.send(\"noop\");".to_string()
    }

    fn read_contenteditable_selection_js(&self, _block_id: &str) -> String {
        "dioxus.send(\"0\");".to_string()
    }

    fn read_contenteditable_selection_detailed_js(&self, _block_id: &str) -> String {
        "dioxus.send(\"0\\u001f0\\u001f1\");".to_string()
    }

    fn read_contenteditable_beforeinput_meta_js(&self, _block_id: &str) -> String {
        "dioxus.send(\"\");".to_string()
    }

    fn read_contenteditable_text_js(&self, _block_id: &str) -> String {
        "dioxus.send(\"\");".to_string()
    }

    fn set_contenteditable_selection_js(&self, _block_id: &str, _raw_utf16: usize) -> String {
        "dioxus.send(\"noop\");".to_string()
    }

    fn set_contenteditable_selection_range_js(
        &self,
        _block_id: &str,
        _start_utf16: usize,
        _end_utf16: usize,
    ) -> String {
        String::new()
    }

    fn bind_contenteditable_input_js(&self, _block_id: &str) -> String {
        "dioxus.send(\"noop\");".to_string()
    }

    fn vertical_caret_move_js(&self, _block_id: &str, _going_up: bool, _goal_x: f64) -> String {
        // No DOM line geometry on non-webview targets. Returning "none" would make
        // vertical nav fully inert; instead degrade to an escape at column 0 so the
        // Rust escape branch navigates to the adjacent block — matching pre-#75
        // behavior (vertical nav escaped at column 0 with no in-block row movement).
        "dioxus.send(\"escape:0:-1\");".to_string()
    }
}

#[cfg(any(
    target_arch = "wasm32",
    target_os = "windows",
    target_os = "macos",
    target_os = "linux",
    target_os = "ios",
    target_os = "android"
))]
static WEBVIEW_ADAPTER: WebviewCaretAdapter = WebviewCaretAdapter;
#[cfg(not(any(
    target_arch = "wasm32",
    target_os = "windows",
    target_os = "macos",
    target_os = "linux",
    target_os = "ios",
    target_os = "android"
)))]
static NOOP_ADAPTER: NoopCaretAdapter = NoopCaretAdapter;

/// Returns the platform adapter for caret/selection interop.
pub fn caret_adapter() -> &'static dyn CaretAdapter {
    #[cfg(any(
        target_arch = "wasm32",
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "ios",
        target_os = "android"
    ))]
    {
        &WEBVIEW_ADAPTER
    }

    #[cfg(not(any(
        target_arch = "wasm32",
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "ios",
        target_os = "android"
    )))]
    {
        &NOOP_ADAPTER
    }
}

/// Start an eval session.
pub fn start_eval(js: &str) -> Eval {
    document::eval(js)
}

/// Evaluate JS and ignore the result.
pub async fn eval_void(js: &str) {
    let _ = document::eval(js).await;
}

/// Receive a string from an eval session.
pub async fn recv_string(eval: &mut Eval) -> Option<String> {
    eval.recv::<String>().await.ok()
}

/// Receive `u64` from an eval session.
pub async fn recv_u64(eval: &mut Eval) -> Option<u64> {
    eval.recv::<u64>().await.ok()
}

/// Receive `f64` from an eval session.
pub async fn recv_f64(eval: &mut Eval) -> Option<f64> {
    eval.recv::<f64>().await.ok()
}

/// Receive `Vec<u64>` from an eval session.
pub async fn recv_vec_u64(eval: &mut Eval) -> Option<Vec<u64>> {
    eval.recv::<Vec<u64>>().await.ok()
}
