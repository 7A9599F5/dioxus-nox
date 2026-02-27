//! Inline live-preview editor — Obsidian-style cursor-aware block switching.
//!
//! All content management (initial render, block switching, cursor restoration)
//! is handled via `document::eval()`.  Dioxus never writes VDOM children into
//! the `<div contenteditable>`, sidestepping the VDOM ↔ contenteditable
//! cursor-destruction problem entirely.
//!
//! Used automatically by `markdown::Editor` when the parent `markdown::Root`
//! has `live_preview_variant: LivePreviewVariant::Inline`.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_core::{Task, use_drop};

use crate::context::{escape_js, use_markdown_context};
use crate::parser::render_block_to_html_string;
use crate::types::{ActiveBlockInputEvent, BlockEntry};

// ── JS generator functions ─────────────────────────────────────────────────

/// Generate JS that initialises the inline editor:
/// - Sets `innerHTML` to `html_content`.
/// - Attaches a `selectionchange` listener that posts `[charOffset, blockIndex]`
///   to Dioxus via `dioxus.send()` on every cursor movement.
///   `blockIndex` is `-1` when the cursor is outside all block elements.
pub(crate) fn inline_editor_init_js(editor_id: &str, html_content: &str) -> String {
    let html_escaped = escape_js(html_content);
    format!(
        r#"(function() {{
    var editor = document.getElementById('{editor_id}');
    if (!editor) return;
    editor.innerHTML = '{html_escaped}';

    function sendCursorEvent() {{
        var sel = window.getSelection();
        if (!sel || sel.rangeCount === 0) {{
            dioxus.send([-1, -1]);
            return;
        }}
        var focusNode = sel.focusNode;

        // Walk up from focus node to find the nearest [data-block-index] element.
        var blockEl = (focusNode && focusNode.nodeType === 3)
            ? focusNode.parentElement
            : focusNode;
        while (blockEl && blockEl !== editor) {{
            if (blockEl.hasAttribute && blockEl.hasAttribute('data-block-index')) break;
            blockEl = blockEl.parentElement;
        }}
        var blockIndex = (blockEl && blockEl !== editor)
            ? parseInt(blockEl.getAttribute('data-block-index'), 10)
            : -1;

        // Count char offset from start of editor to focus position (text nodes only).
        var walker = document.createTreeWalker(editor, NodeFilter.SHOW_TEXT);
        var charCount = 0;
        var node;
        while ((node = walker.nextNode())) {{
            if (node === focusNode) {{
                charCount += sel.focusOffset;
                break;
            }}
            charCount += node.length;
        }}

        dioxus.send([charCount, blockIndex]);
    }}

    // Remove any previous listener so re-init doesn't double-fire.
    if (editor._noxSelHandler) {{
        document.removeEventListener('selectionchange', editor._noxSelHandler);
    }}
    editor._noxSelHandler = sendCursorEvent;
    document.addEventListener('selectionchange', sendCursorEvent);
}})();"#,
        editor_id = editor_id,
        html_escaped = html_escaped,
    )
}

/// Generate JS that switches a block from formatted HTML to raw markdown text
/// and restores the cursor to approximately `saved_char_offset` within the editor.
pub(crate) fn inline_editor_switch_block_js(
    editor_id: &str,
    block_index: usize,
    raw_text: &str,
    saved_char_offset: usize,
) -> String {
    let raw_escaped = escape_js(raw_text);
    format!(
        r#"(function() {{
    var editor = document.getElementById('{editor_id}');
    if (!editor) return;
    var block = editor.querySelector('[data-block-index="{block_index}"]');
    if (!block) return;

    // Replace block content with raw markdown as a plain text node.
    // white-space: pre-wrap preserves newlines (otherwise \n collapses to space in HTML).
    block.style.whiteSpace = 'pre-wrap';
    block.textContent = '{raw_escaped}';

    // Restore cursor using TreeWalker — walk text nodes to find saved offset.
    var saved = {saved_char_offset};
    var walker = document.createTreeWalker(editor, NodeFilter.SHOW_TEXT);
    var charCount = 0;
    var node;
    while ((node = walker.nextNode())) {{
        var next = charCount + node.length;
        if (next >= saved) {{
            try {{
                var range = document.createRange();
                range.setStart(node, Math.min(saved - charCount, node.length));
                range.collapse(true);
                var sel = window.getSelection();
                sel.removeAllRanges();
                sel.addRange(range);
            }} catch (e) {{}}
            break;
        }}
        charCount = next;
    }}
}})();"#,
        editor_id = editor_id,
        block_index = block_index,
        raw_escaped = raw_escaped,
        saved_char_offset = saved_char_offset,
    )
}

/// Generate JS that restores a block from raw text back to formatted HTML.
/// Called when the cursor leaves a block.
pub(crate) fn inline_editor_restore_block_js(
    editor_id: &str,
    block_index: usize,
    formatted_html: &str,
) -> String {
    let html_escaped = escape_js(formatted_html);
    format!(
        r#"(function() {{
    var editor = document.getElementById('{editor_id}');
    if (!editor) return;
    var block = editor.querySelector('[data-block-index="{block_index}"]');
    if (!block) return;
    // Clear pre-wrap set during raw-edit mode, then restore formatted HTML.
    block.style.whiteSpace = '';
    block.innerHTML = '{html_escaped}';
}})();"#,
        editor_id = editor_id,
        block_index = block_index,
        html_escaped = html_escaped,
    )
}

/// Render all block HTML strings into one combined HTML fragment suitable
/// for setting as the inline editor's initial `innerHTML`.
pub(crate) fn render_blocks_html(blocks: &[BlockEntry]) -> String {
    blocks.iter().map(|b| b.html.as_str()).collect::<String>()
}

/// Reconstruct full markdown from per-block raw strings.
///
/// Consecutive list items (`is_list_items[i]` and `is_list_items[i-1]` both
/// `true`) are joined with `"\n"`.  All other block boundaries use `"\n\n"`.
pub(crate) fn reconstruct_markdown(raws: &[String], is_list_items: &[bool]) -> String {
    let mut result = String::new();
    for (i, raw) in raws.iter().enumerate() {
        if i > 0 {
            let sep = if is_list_items.get(i - 1).copied().unwrap_or(false)
                && is_list_items.get(i).copied().unwrap_or(false)
            {
                "\n"
            } else {
                "\n\n"
            };
            result.push_str(sep);
        }
        result.push_str(raw);
    }
    result
}

// ── InlineEditor component ─────────────────────────────────────────────────

/// Inline live-preview editor component.
///
/// Used by `markdown::Editor` when `LivePreviewVariant::Inline` is active.
/// Renders a single `<div contenteditable="true">` managed via `eval()`.
/// All blocks render as formatted HTML; the block under the cursor reverts
/// to raw markdown so the user types directly into the source.
///
/// **Zero visual styles** — consumers style via the `class` prop and CSS targeting
/// `[data-md-inline-editor]`, `[data-block-index]`.
#[component]
pub fn InlineEditor(
    class: Option<String>,
    /// Fires on every `oninput` with the active block's raw text + block-local cursor.
    /// Used to wire inline-trigger suggestions without coupling markdown to suggest.
    on_active_block_input: Option<EventHandler<ActiveBlockInputEvent>>,
) -> Element {
    let ctx = use_markdown_context();
    let editor_id = ctx.inline_editor_id();

    // Which block index currently has the cursor (None = cursor outside all blocks).
    let mut active_block: Signal<Option<usize>> = use_signal(|| None);

    // Per-block raw text cache.  Updated when cursor moves (on depart) or user types.
    // Rc<RefCell<...>> for hot-path: not reactive, reads driven by eval results.
    let block_raws: Rc<RefCell<Vec<String>>> = use_hook(|| {
        let blocks = &(ctx.parsed_doc)().blocks;
        Rc::new(RefCell::new(
            blocks.iter().map(|b| b.raw.clone()).collect(),
        ))
    });

    // Per-block is_list_item flags — mirrors block_raws, kept in sync on doc change.
    let block_metas: Rc<RefCell<Vec<bool>>> = use_hook(|| {
        let blocks = &(ctx.parsed_doc)().blocks;
        Rc::new(RefCell::new(
            blocks.iter().map(|b| b.is_list_item).collect(),
        ))
    });

    // Handle for the selectionchange eval task; cancelled on unmount.
    let eval_task: Rc<RefCell<Option<Task>>> = use_hook(|| Rc::new(RefCell::new(None)));
    {
        let task_ref = eval_task.clone();
        use_drop(move || {
            if let Some(task) = task_ref.borrow_mut().take() {
                task.cancel();
            }
        });
    }

    // Reactive effect: when the parsed doc changes (debounced parse fired),
    // refresh inactive blocks to their latest rendered HTML.
    {
        let braws = block_raws.clone();
        let bmetas = block_metas.clone();
        let eid = editor_id.clone();
        use_effect(move || {
            let doc = (ctx.parsed_doc)();
            let active = (active_block)();

            // Sync block_raws and block_metas for all inactive blocks with freshly parsed data.
            {
                let mut raws = braws.borrow_mut();
                let new_len = doc.blocks.len();
                raws.resize(new_len, String::new());
                for block in &doc.blocks {
                    if Some(block.index) != active {
                        raws[block.index] = block.raw.clone();
                    }
                }
            }
            {
                let mut metas = bmetas.borrow_mut();
                metas.resize(doc.blocks.len(), false);
                for block in &doc.blocks {
                    metas[block.index] = block.is_list_item;
                }
            }

            // Push updated HTML to inactive blocks in the DOM.
            let eid2 = eid.clone();
            spawn(async move {
                for block in &doc.blocks {
                    if Some(block.index) != active {
                        let js = inline_editor_restore_block_js(&eid2, block.index, &block.html);
                        let _ = document::eval(&js).await;
                    }
                }
            });
        });
    }

    // Keep on_active_block_input in sync across renders (EventHandler is Copy in Dioxus 0.7).
    let oabi_sig: Signal<Option<EventHandler<ActiveBlockInputEvent>>> =
        use_signal(|| on_active_block_input);
    {
        let mut oabi = oabi_sig;
        oabi.set(on_active_block_input);
    }

    let editor_id_mount = editor_id.clone();
    let editor_id_input = editor_id.clone();
    // Pre-clone Rc handles for each closure that needs them.
    let block_raws_mount = block_raws.clone();
    let block_raws_input = block_raws;
    let block_metas_input = block_metas;
    let eval_task_mount = eval_task;

    rsx! {
        div {
            id: "{editor_id}",
            class: class.unwrap_or_default(),
            "contenteditable": "true",
            "data-md-inline-editor": "true",
            "data-state": "active",

            // ── One-time initialisation on mount ──
            onmounted: move |_| {
                let eid = editor_id_mount.clone();
                let braws = block_raws_mount.clone();
                let task_ref = eval_task_mount.clone();
                let doc = (ctx.parsed_doc)();

                let task = spawn(async move {
                    // Set innerHTML and attach selectionchange listener.
                    let init_html = render_blocks_html(&doc.blocks);
                    let init_js = inline_editor_init_js(&eid, &init_html);
                    let mut eval = document::eval(&init_js);

                    // Receive [charOffset, blockIndex] from the JS listener.
                    while let Ok(arr) = eval.recv::<Vec<i64>>().await {
                        let char_offset = arr.first().copied().unwrap_or(-1);
                        let block_idx = arr.get(1).copied().unwrap_or(-1);

                        if char_offset < 0 || block_idx < 0 {
                            // Cursor outside blocks — deactivate current active block.
                            if let Some(old) = (active_block)() {
                                let raw = {
                                    let raws = braws.borrow();
                                    raws.get(old).cloned().unwrap_or_default()
                                };
                                let restore_html = render_block_to_html_string(&raw, old);
                                let js = inline_editor_restore_block_js(&eid, old, &restore_html);
                                let _ = document::eval(&js).await;
                                active_block.set(None);
                            }
                            continue;
                        }

                        let new_block = block_idx as usize;
                        let char_off = char_offset as usize;
                        let old_active = (active_block)();

                        if old_active != Some(new_block) {
                            // Restore previously active block to formatted HTML.
                            if let Some(old) = old_active {
                                let raw = {
                                    let raws = braws.borrow();
                                    raws.get(old).cloned().unwrap_or_default()
                                };
                                let restore_html = render_block_to_html_string(&raw, old);
                                let js = inline_editor_restore_block_js(&eid, old, &restore_html);
                                let _ = document::eval(&js).await;
                            }

                            // Switch newly active block to raw markdown.
                            let new_raw = {
                                let raws = braws.borrow();
                                raws.get(new_block).cloned().unwrap_or_default()
                            };
                            let switch_js =
                                inline_editor_switch_block_js(&eid, new_block, &new_raw, char_off);
                            let _ = document::eval(&switch_js).await;

                            active_block.set(Some(new_block));
                        }
                    }
                });

                *task_ref.borrow_mut() = Some(task);
            },

            // ── oninput: sync active block's raw text → raw_content → debounced parse ──
            oninput: move |_| {
                let active = (active_block)();
                if let Some(idx) = active {
                    let eid = editor_id_input.clone();
                    let braws = block_raws_input.clone();
                    let bmetas = block_metas_input.clone();
                    spawn(async move {
                        // Read textContent + block-local cursor from the active block element.
                        let js = format!(
                            r#"(function(){{
    var ed = document.getElementById('{eid}');
    var bl = ed ? ed.querySelector('[data-block-index="{idx}"]') : null;
    var text = bl ? bl.textContent : '';
    var cursor = 0;
    if (bl) {{
        var sel = window.getSelection();
        if (sel && sel.rangeCount > 0) {{
            var range = document.createRange();
            range.setStart(bl, 0);
            try {{ range.setEnd(sel.focusNode, sel.focusOffset); cursor = range.toString().length; }}
            catch(e) {{ cursor = 0; }}
        }}
    }}
    dioxus.send([text, cursor]);
}})();"#
                        );
                        let mut ev = document::eval(&js);
                        if let Ok((text, cursor_u64)) = ev.recv::<(String, u64)>().await {
                            let cursor_utf16 = cursor_u64 as usize;
                            // Clone before moving into the slot.
                            let text_for_cb = text.clone();
                            // Update cached raw text for the active block.
                            if let Ok(mut raws) = braws.try_borrow_mut()
                                && let Some(slot) = raws.get_mut(idx)
                            {
                                *slot = text;
                            }
                            // Reconstruct the full document and trigger parse.
                            let full_md = reconstruct_markdown(
                                &braws.borrow(),
                                &bmetas.borrow(),
                            );
                            ctx.handle_value_change(full_md);
                            ctx.trigger_parse.call(());
                            // Fire the active block input callback if provided.
                            if let Some(ref cb) = *oabi_sig.read() {
                                cb.call(ActiveBlockInputEvent {
                                    text: text_for_cb,
                                    cursor_utf16,
                                    block_idx: idx,
                                });
                            }
                        }
                    });
                }
            },

            // ── onkeydown: intercept Tab to prevent focus shift ──
            onkeydown: move |evt: KeyboardEvent| {
                let key = evt.key().to_string();
                if key == "Tab" {
                    evt.prevent_default();
                }
            },
        }
    }
}
