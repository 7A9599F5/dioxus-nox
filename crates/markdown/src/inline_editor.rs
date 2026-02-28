use std::rc::Rc;

use dioxus::prelude::*;

use crate::context::{CursorContext, MarkdownContext};
use crate::inline_tokens::{
    InlineMark, InlineSegment, MarkerVisibility, SegmentKind, TokenizedBlock,
    build_tokenized_block, collect_marker_tokens, raw_offset_to_visible_utf16,
    visible_utf16_to_raw_offset,
};
use crate::interop;
use crate::reveal_engine::{RevealContext, marker_visibility};
use crate::types::{ActiveBlockInputEvent, CursorPosition, NodeType, OwnedAstNode};
use crate::viewport::{BlockOverride, EditorViewport, ViewportNode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingCaretRestore {
    Raw(usize),
    Visible(usize),
    /// Visible UTF-16 offsets for a non-collapsed selection.
    Selection { start: usize, end: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SelectionDetails {
    start: usize,
    end: usize,
    collapsed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BeforeInputMeta {
    input_type: String,
    data: String,
    pre_visible_caret_utf16: usize,
    pre_visible_selection_end_utf16: usize,
    is_collapsed: bool,
}

/// Obsidian-style inline markdown editor surface.
///
/// In inline mode, this component renders fully formatted markdown blocks and
/// swaps only the active block into a raw-markdown `<textarea>`.
#[component]
pub fn InlineEditor(on_active_block_input: Option<EventHandler<ActiveBlockInputEvent>>) -> Element {
    let cursor_ctx = try_use_context::<CursorContext>();
    let cursor_offset = cursor_ctx
        .map(|c| c.cursor_position.read().offset)
        .unwrap_or(0);

    let overrides = vec![BlockOverride {
        matches: Rc::new(is_editable_block),
        component: Rc::new(move |node: OwnedAstNode| {
            rsx! {
                InlineBlockNode {
                    node: node,
                    cursor_offset: cursor_offset,
                    on_active_block_input: on_active_block_input
                }
            }
        }),
    }];

    rsx! {
        div {
            "data-md-inline-editor": "true",
            "data-state": "active",
            EditorViewport { overrides: overrides }
        }
    }
}

fn is_editable_block(node: &OwnedAstNode) -> bool {
    matches!(
        node.node_type,
        NodeType::Paragraph
            | NodeType::Heading(_)
            | NodeType::BlockQuote
            | NodeType::CodeBlock(_)
            | NodeType::Item
    )
}

#[component]
fn InlineBlockNode(
    node: OwnedAstNode,
    cursor_offset: usize,
    on_active_block_input: Option<EventHandler<ActiveBlockInputEvent>>,
) -> Element {
    let is_active = cursor_offset >= node.range.start && cursor_offset < node.range.end;
    if !is_active {
        return rsx! { InactiveBlockView { node: node } };
    }

    if matches!(node.node_type, NodeType::CodeBlock(_)) {
        rsx! {
            ActiveBlockEditor {
                node: node,
                on_active_block_input: on_active_block_input
            }
        }
    } else {
        rsx! {
            TokenAwareBlockEditor {
                node: node,
                cursor_offset: cursor_offset,
                on_active_block_input: on_active_block_input,
            }
        }
    }
}

#[cfg(test)]
fn uses_token_aware_surface(node: &OwnedAstNode) -> bool {
    match node.node_type {
        NodeType::CodeBlock(_) => false,
        NodeType::Paragraph | NodeType::Heading(_) | NodeType::BlockQuote | NodeType::Item => {
            block_has_inline_markup(node) || has_block_prefix_marker(node)
        }
        _ => false,
    }
}

#[cfg(test)]
fn has_block_prefix_marker(node: &OwnedAstNode) -> bool {
    matches!(
        node.node_type,
        NodeType::Heading(_) | NodeType::BlockQuote | NodeType::Item
    )
}

#[cfg(test)]
fn block_has_inline_markup(node: &OwnedAstNode) -> bool {
    is_markup_inline_node_type(&node.node_type) || node.children.iter().any(block_has_inline_markup)
}

#[cfg(test)]
fn cursor_within_inline_markup(node: &OwnedAstNode, cursor_offset: usize) -> bool {
    let in_this = is_markup_inline_node_type(&node.node_type)
        && cursor_offset >= node.range.start
        && cursor_offset <= node.range.end;

    if in_this {
        return true;
    }

    node.children
        .iter()
        .any(|child| cursor_within_inline_markup(child, cursor_offset))
}

#[cfg(test)]
fn is_markup_inline_node_type(node_type: &NodeType) -> bool {
    matches!(
        node_type,
        NodeType::Emphasis
            | NodeType::Strong
            | NodeType::Strikethrough
            | NodeType::Code(_)
            | NodeType::Link { .. }
            | NodeType::Image { .. }
            | NodeType::Wikilink(_)
            | NodeType::Tag(_)
    )
}

#[component]
fn InactiveBlockView(node: OwnedAstNode) -> Element {
    let ctx = use_context::<MarkdownContext>();
    let cursor_ctx = try_use_context::<CursorContext>();
    let block_id = format!("nox-md-inline-block-{}", node.range.start);
    let safe_start = node.range.start;
    let safe_end = node.range.end;

    match &node.node_type {
        // Keep list structure valid: ul/ol children must be li.
        NodeType::Item => {
            let block_id_for_click = block_id.clone();
            let node_for_click = node.clone();
            let node_for_render = node.clone();
            rsx! {
                li {
                    id: "{block_id}",
                    "data-md-inline-block": "true",
                    onclick: move |_| {
                        handle_inactive_block_click(
                            cursor_ctx,
                            ctx.raw_value(),
                            node_for_click.clone(),
                            block_id_for_click.clone(),
                            safe_start,
                            safe_end,
                        );
                    },
                    for child in node_for_render.children {
                        ViewportNode {
                            node: child,
                            overrides: vec![]
                        }
                    }
                }
            }
        }
        _ => {
            let block_id_for_click = block_id.clone();
            let node_for_click = node.clone();
            let node_for_render = node.clone();
            rsx! {
                div {
                    id: "{block_id}",
                    "data-md-inline-block": "true",
                    onclick: move |_| {
                        handle_inactive_block_click(
                            cursor_ctx,
                            ctx.raw_value(),
                            node_for_click.clone(),
                            block_id_for_click.clone(),
                            safe_start,
                            safe_end,
                        );
                    },
                    ViewportNode {
                        node: node_for_render,
                        overrides: vec![]
                    }
                }
            }
        }
    }
}

#[component]
fn TokenAwareBlockEditor(
    node: OwnedAstNode,
    cursor_offset: usize,
    on_active_block_input: Option<EventHandler<ActiveBlockInputEvent>>,
) -> Element {
    let ctx = use_context::<MarkdownContext>();
    let cursor_ctx = try_use_context::<CursorContext>();
    let raw = ctx.raw_value();

    let node_end = node.range.end.min(raw.len());
    let safe_start = node.range.start.min(node_end);
    let safe_end = trim_editable_block_end(&raw, safe_start, node_end);
    let local_cursor = cursor_offset
        .saturating_sub(safe_start)
        .min(last_caret_offset(&(0..safe_end.saturating_sub(safe_start))));
    let block_id = format!("nox-md-token-{}", safe_start);
    let current_len = use_signal(|| safe_end.saturating_sub(safe_start));
    let mut is_composing = use_signal(|| false);
    let mut input_revision = use_signal(|| 0u64);
    let applied_revision = use_signal(|| 0u64);
    let mut caret_generation = use_signal(|| 0u64);
    let restore_generation = use_signal(|| 0u64);
    let mut pending_restore_raw = use_signal(|| None::<PendingCaretRestore>);
    let block_id_input = block_id.clone();
    let block_id_comp_end = block_id.clone();
    let block_id_nav = block_id.clone();
    let block_id_keyup = block_id.clone();
    let block_id_mouseup = block_id.clone();
    let block_id_mount = block_id.clone();
    let block_id_effect = block_id.clone();

    let block_raw = raw[safe_start..safe_end].to_string();
    let mut model_node = node.clone();
    model_node.range = safe_start..safe_end;
    let marker_tokens = collect_marker_tokens(&model_node, &block_raw, safe_start);
    let visibility_flags = marker_visibility(
        &marker_tokens,
        RevealContext {
            caret_raw_offset: local_cursor,
            selection: None,
        },
    );
    let visibility = visibility_flags
        .iter()
        .enumerate()
        .map(|(idx, visible)| MarkerVisibility {
            marker_idx: idx,
            visible: *visible,
        })
        .collect::<Vec<_>>();
    let model = build_tokenized_block(&model_node, &raw, &visibility);
    let model_for_input = model.clone();
    let model_for_comp_end = model.clone();
    let model_for_effect = model.clone();
    let model_for_nav = model.clone();
    let model_for_keyup = model.clone();
    let model_for_mouseup = model.clone();
    let node_for_input = model_node.clone();
    let node_for_comp_end = node.clone();
    let target_visible_cursor = raw_offset_to_visible_utf16(&model, local_cursor);
    let target_visible_cursor_mount = target_visible_cursor;
    let inline_input_cursor = byte_to_utf16_index(&model.raw_text, local_cursor).unwrap_or(0);
    let visible_input_cursor = target_visible_cursor;
    let inline_visible_text = model.visible_text.clone();
    let inline_raw_text = model.raw_text.clone();
    let is_single_line_block = !model.raw_text.contains('\n');
    let pending_restore_for_keyup = pending_restore_raw;
    let pending_restore_for_mouseup = pending_restore_raw;
    let pending_restore_for_nav = pending_restore_raw;
    let caret_generation_for_keyup = caret_generation;
    let caret_generation_for_mouseup = caret_generation;
    let caret_generation_for_nav = caret_generation;

    use_effect(move || {
        let pending = *pending_restore_raw.read();
        if is_composing() {
            return;
        }
        let Some(pending) = pending else {
            return;
        };

        let js = match pending {
            PendingCaretRestore::Raw(abs_raw) => {
                let local_raw = abs_raw
                    .saturating_sub(safe_start)
                    .min(last_caret_offset(&(0..model_for_effect.raw_text.len())));
                let v = raw_offset_to_visible_utf16(&model_for_effect, local_raw);
                interop::caret_adapter().set_contenteditable_selection_js(&block_id_effect, v)
            }
            PendingCaretRestore::Visible(visible) => {
                let v = visible.min(utf16_len(&model_for_effect.visible_text));
                interop::caret_adapter().set_contenteditable_selection_js(&block_id_effect, v)
            }
            PendingCaretRestore::Selection { start, end } => interop::caret_adapter()
                .set_contenteditable_selection_range_js(&block_id_effect, start, end),
        };
        pending_restore_raw.set(None);
        let restore_gen = restore_generation();
        let restore_gen_sig = restore_generation;
        spawn(async move {
            // If oninput/oncompositionend fired since this restore was queued,
            // it is stale — skip to prevent overriding the correct cursor.
            if *restore_gen_sig.read() != restore_gen {
                return;
            }
            interop::eval_void(&js).await;
        });
    });

    let token_view = rsx! {
        div {
            id: "{block_id}",
            "data-md-token-editor": "true",
            contenteditable: "true",
            style: "width:100%;min-width:100%;max-width:100%;box-sizing:border-box;outline:none;white-space:pre-wrap;word-break:break-word;",
            onkeydown: move |evt: KeyboardEvent| {
                let key = evt.key().to_string();
                if is_single_line_block && (key == "ArrowUp" || key == "ArrowDown") {
                    evt.prevent_default();
                    if let Some(mut cctx) = cursor_ctx {
                        let parsed = (ctx.parsed_doc)();
                        let direction = if key == "ArrowUp" {
                            NavDirection::Prev
                        } else {
                            NavDirection::Next
                        };
                        let target = adjacent_editable_offset(
                            &parsed.ast,
                            safe_start,
                            safe_end,
                            direction,
                        );
                        cctx.cursor_position.set(CursorPosition {
                            offset: target,
                            line: 0,
                            column: 0,
                        });
                    }
                }
                if key == "ArrowLeft" || key == "ArrowRight" {
                    evt.prevent_default();
                    if let Some(mut cctx) = cursor_ctx {
                        let block_id = block_id_nav.clone();
                        let model = model_for_nav.clone();
                        let mut pending_restore = pending_restore_for_nav;
                        let generation = caret_generation_for_nav();
                        let generation_sig = caret_generation_for_nav;
                        spawn(async move {
                            let visible_now = {
                                let js =
                                    interop::caret_adapter().read_contenteditable_selection_js(&block_id);
                                let mut eval = interop::start_eval(&js);
                                interop::recv_string(&mut eval)
                                    .await
                                    .and_then(|s| s.parse::<usize>().ok())
                                    .unwrap_or(0)
                            };
                            if !is_latest_revision(generation, *generation_sig.read()) {
                                return;
                            }
                            let max_visible = utf16_len(&model.visible_text);
                            let target_visible = if key == "ArrowLeft" {
                                visible_now.saturating_sub(1)
                            } else {
                                visible_now.saturating_add(1).min(max_visible)
                            };
                            let local_raw = visible_utf16_to_raw_offset(&model, target_visible)
                                .min(last_caret_offset(&(0..model.raw_text.len())));
                            let abs_raw = safe_start.saturating_add(local_raw);
                            cctx.cursor_position.set(CursorPosition {
                                offset: abs_raw,
                                line: 0,
                                column: 0,
                            });
                            pending_restore.set(Some(PendingCaretRestore::Raw(abs_raw)));
                        });
                    }
                }
            },
            oninput: move |_| {
                if is_composing() {
                    return;
                }
                // Drop stale restore requests from earlier click/nav events.
                // Text mutations own caret placement via the input pipeline.
                pending_restore_raw.set(None);
                // Bump restore_generation so any in-flight use_effect spawn that
                // already captured the old generation will self-cancel.
                { let mut rg = restore_generation; rg.set(rg().saturating_add(1)); }
                let next_generation = caret_generation().saturating_add(1);
                caret_generation.set(next_generation);
                let next_revision = input_revision().saturating_add(1);
                input_revision.set(next_revision);
                let model = model_for_input.clone();
                let block_id = block_id_input.clone();
                let cursor_ctx_local = cursor_ctx;
                let handler = on_active_block_input;
                let node_local = node_for_input.clone();
                let len_sig = current_len;
                spawn_token_editor_sync(
                    block_id,
                    model,
                    ctx,
                    safe_start,
                    node_local,
                    len_sig,
                    cursor_ctx_local,
                    handler,
                    next_revision,
                    input_revision,
                    applied_revision,
                    pending_restore_raw,
                );
            },
            oncompositionstart: move |_| {
                is_composing.set(true);
            },
            oncompositionend: move |_| {
                is_composing.set(false);
                pending_restore_raw.set(None);
                // Bump restore_generation so any in-flight use_effect spawn that
                // already captured the old generation will self-cancel.
                { let mut rg = restore_generation; rg.set(rg().saturating_add(1)); }
                let next_generation = caret_generation().saturating_add(1);
                caret_generation.set(next_generation);
                let next_revision = input_revision().saturating_add(1);
                input_revision.set(next_revision);
                let model = model_for_comp_end.clone();
                let block_id = block_id_comp_end.clone();
                let cursor_ctx_local = cursor_ctx;
                let handler = on_active_block_input;
                let mut node_local = node_for_comp_end.clone();
                node_local.range = safe_start..safe_end;
                let len_sig = current_len;
                spawn_token_editor_sync(
                    block_id,
                    model,
                    ctx,
                    safe_start,
                    node_local,
                    len_sig,
                    cursor_ctx_local,
                    handler,
                    next_revision,
                    input_revision,
                    applied_revision,
                    pending_restore_raw,
                );
            },
            onkeyup: move |evt: KeyboardEvent| {
                let key = evt.key().to_string();
                if !is_navigation_key(&key) {
                    return;
                }
                if key == "ArrowLeft" || key == "ArrowRight" {
                    return;
                }
                if is_single_line_block && (key == "ArrowUp" || key == "ArrowDown") {
                    return;
                }
                if let Some(mut cctx) = cursor_ctx {
                    let block_id = block_id_keyup.clone();
                    let model = model_for_keyup.clone();
                    let mut pending_restore = pending_restore_for_keyup;
                    let generation = caret_generation_for_keyup();
                    let generation_sig = caret_generation_for_keyup;
                    spawn(async move {
                        let cursor_visible_utf16 = {
                            let js = interop::caret_adapter()
                                .read_contenteditable_selection_js(&block_id);
                            let mut eval = interop::start_eval(&js);
                            interop::recv_string(&mut eval)
                                .await
                                .and_then(|s| s.parse::<usize>().ok())
                                .unwrap_or(0)
                        };
                        if !is_latest_revision(generation, *generation_sig.read()) {
                            return;
                        }
                        let local_raw = visible_utf16_to_raw_offset(&model, cursor_visible_utf16)
                            .min(last_caret_offset(&(0..model.raw_text.len())));
                        let abs_raw = safe_start.saturating_add(local_raw);
                        cctx.cursor_position.set(CursorPosition {
                            offset: abs_raw,
                            line: 0,
                            column: 0,
                        });
                        pending_restore.set(Some(PendingCaretRestore::Raw(abs_raw)));
                    });
                }
            },
            onmouseup: move |_| {
                if let Some(mut cctx) = cursor_ctx {
                    let block_id = block_id_mouseup.clone();
                    let model = model_for_mouseup.clone();
                    let mut pending_restore = pending_restore_for_mouseup;
                    let generation = caret_generation_for_mouseup();
                    let generation_sig = caret_generation_for_mouseup;
                    spawn(async move {
                        let sel = {
                            let js = interop::caret_adapter()
                                .read_contenteditable_selection_detailed_js(&block_id);
                            let mut eval = interop::start_eval(&js);
                            interop::recv_string(&mut eval)
                                .await
                                .and_then(|s| parse_selection_details(&s))
                        };
                        if !is_latest_revision(generation, *generation_sig.read()) {
                            return;
                        }
                        let (cursor_visible_utf16, restore) = match sel {
                            Some(ref d) if !d.collapsed => {
                                // Non-collapsed: restore the full range; cursor context = end.
                                (
                                    d.end,
                                    PendingCaretRestore::Selection {
                                        start: d.start,
                                        end: d.end,
                                    },
                                )
                            }
                            Some(ref d) => {
                                let local_raw = visible_utf16_to_raw_offset(&model, d.end)
                                    .min(last_caret_offset(&(0..model.raw_text.len())));
                                let abs = safe_start.saturating_add(local_raw);
                                (d.end, PendingCaretRestore::Raw(abs))
                            }
                            None => (0, PendingCaretRestore::Raw(safe_start)),
                        };
                        let local_raw = visible_utf16_to_raw_offset(&model, cursor_visible_utf16)
                            .min(last_caret_offset(&(0..model.raw_text.len())));
                        let abs_raw = safe_start.saturating_add(local_raw);
                        cctx.cursor_position.set(CursorPosition {
                            offset: abs_raw,
                            line: 0,
                            column: 0,
                        });
                        pending_restore.set(Some(restore));
                    });
                }
            },
            onmounted: move |_| {
                let set_js = interop::caret_adapter()
                    .set_contenteditable_selection_js(&block_id_mount, target_visible_cursor_mount);
                let bind_js = interop::caret_adapter().bind_contenteditable_input_js(&block_id_mount);
                spawn(async move {
                    interop::eval_void(&set_js).await;
                    interop::eval_void(&bind_js).await;
                });
                if let Some(handler) = on_active_block_input {
                    handler.call(ActiveBlockInputEvent {
                        raw_text: inline_raw_text.clone(),
                        visible_text: inline_visible_text.clone(),
                        cursor_raw_utf16: inline_input_cursor,
                        cursor_visible_utf16: visible_input_cursor,
                        block_start: safe_start,
                        block_end: safe_end,
                    });
                }
            },
            for seg in model.segments.clone() {
                { render_inline_segment(seg) }
            }
        }
    };

    match &node.node_type {
        NodeType::Heading(1) => rsx! { h1 { {token_view} } },
        NodeType::Heading(2) => rsx! { h2 { {token_view} } },
        NodeType::Heading(3) => rsx! { h3 { {token_view} } },
        NodeType::Heading(4) => rsx! { h4 { {token_view} } },
        NodeType::Heading(5) => rsx! { h5 { {token_view} } },
        NodeType::Heading(6) => rsx! { h6 { {token_view} } },
        NodeType::BlockQuote => rsx! { blockquote { {token_view} } },
        NodeType::Item => rsx! { li { {token_view} } },
        _ => rsx! { p { {token_view} } },
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_token_editor_sync(
    block_id: String,
    model: TokenizedBlock,
    ctx: MarkdownContext,
    safe_start: usize,
    node_local: OwnedAstNode,
    mut len_sig: Signal<usize>,
    cursor_ctx_local: Option<CursorContext>,
    handler: Option<EventHandler<ActiveBlockInputEvent>>,
    captured_revision: u64,
    latest_revision: Signal<u64>,
    mut applied_revision: Signal<u64>,
    mut pending_restore: Signal<Option<PendingCaretRestore>>,
) {
    spawn(async move {
        let new_visible = {
            let js = interop::caret_adapter().read_contenteditable_text_js(&block_id);
            let mut eval = interop::start_eval(&js);
            interop::recv_string(&mut eval).await.unwrap_or_default()
        };
        let selection_details = {
            let js = interop::caret_adapter().read_contenteditable_selection_detailed_js(&block_id);
            let mut eval = interop::start_eval(&js);
            interop::recv_string(&mut eval)
                .await
                .and_then(|s| parse_selection_details(&s))
        };
        let cursor_visible_utf16 = selection_details.as_ref().map_or(0, |s| s.start);
        let before_input_meta = {
            let js = interop::caret_adapter().read_contenteditable_beforeinput_meta_js(&block_id);
            let mut eval = interop::start_eval(&js);
            interop::recv_string(&mut eval)
                .await
                .and_then(|s| parse_before_input_meta(&s))
        };

        // If a newer oninput has fired since this sync was spawned, bail out —
        // we are stale and would stomp the correct cursor position.
        if *latest_revision.read() != captured_revision {
            return;
        }

        let current_global = ctx.raw_value();
        let start = safe_start.min(current_global.len());
        let old_len = *len_sig.read();
        let end = (start + old_len).min(current_global.len());
        let block_raw_current = current_global[start..end].to_string();

        let mut candidate_models = vec![
            model.clone(),
            build_plain_text_model(&block_raw_current, start),
        ];
        let mut current_node = node_local.clone();
        current_node.range = start..end;
        let current_markers = collect_marker_tokens(&current_node, &block_raw_current, start);
        if !current_markers.is_empty() {
            let hidden_visibility = current_markers
                .iter()
                .enumerate()
                .map(|(idx, _)| MarkerVisibility {
                    marker_idx: idx,
                    visible: false,
                })
                .collect::<Vec<_>>();
            candidate_models.push(build_tokenized_block(
                &current_node,
                &current_global,
                &hidden_visibility,
            ));

            let visible_visibility = current_markers
                .iter()
                .enumerate()
                .map(|(idx, _)| MarkerVisibility {
                    marker_idx: idx,
                    visible: true,
                })
                .collect::<Vec<_>>();
            candidate_models.push(build_tokenized_block(
                &current_node,
                &current_global,
                &visible_visibility,
            ));
        }

        let (model_idx, edit) =
            select_best_input_projection(&candidate_models, &new_visible, cursor_visible_utf16);
        let selected_model = &candidate_models[model_idx];
        let effective_cursor_visible = compute_post_visible_caret(
            before_input_meta.as_ref(),
            &edit,
            cursor_visible_utf16,
            utf16_len(&new_visible),
        );
        let old_raw_start =
            visible_utf16_to_raw_offset(selected_model, edit.old_start_utf16).min(block_raw_current.len());
        let old_raw_end =
            visible_utf16_to_raw_offset(selected_model, edit.old_end_utf16).min(block_raw_current.len());
        let rebuilt_local = format!(
            "{}{}{}",
            &block_raw_current[..old_raw_start],
            edit.replacement,
            &block_raw_current[old_raw_end..]
        );
        let rebuilt_global = format!(
            "{}{}{}",
            &current_global[..start],
            rebuilt_local,
            &current_global[end..]
        );
        len_sig.set(rebuilt_local.len());
        ctx.handle_value_change(rebuilt_global.clone());
        ctx.trigger_parse.call(());
        applied_revision.set(captured_revision);

        let raw_cursor_local = cursor_after_visible_edit(
            selected_model,
            effective_cursor_visible,
            &edit,
            old_raw_start,
            old_raw_end,
        )
        .min(last_caret_offset(&(0..rebuilt_local.len())));

        if let Some(mut cctx) = cursor_ctx_local {
            cctx.cursor_position.set(CursorPosition {
                offset: start.saturating_add(raw_cursor_local),
                line: 0,
                column: 0,
            });
        }
        pending_restore.set(Some(PendingCaretRestore::Visible(
            effective_cursor_visible,
        )));

        if let Some(handler) = handler {
            let mut fresh_node = node_local.clone();
            fresh_node.range = start..start.saturating_add(rebuilt_local.len());
            let fresh_tokens = collect_marker_tokens(&fresh_node, &rebuilt_local, start);
            let fresh_visibility_flags = marker_visibility(
                &fresh_tokens,
                RevealContext {
                    caret_raw_offset: raw_cursor_local,
                    selection: None,
                },
            );
            let fresh_visibility = fresh_visibility_flags
                .iter()
                .enumerate()
                .map(|(idx, visible)| MarkerVisibility {
                    marker_idx: idx,
                    visible: *visible,
                })
                .collect::<Vec<_>>();
            let fresh_model =
                build_tokenized_block(&fresh_node, &rebuilt_global, &fresh_visibility);
            handler.call(ActiveBlockInputEvent {
                raw_text: fresh_model.raw_text.clone(),
                visible_text: fresh_model.visible_text.clone(),
                cursor_raw_utf16: byte_to_utf16_index(&fresh_model.raw_text, raw_cursor_local)
                    .unwrap_or(0),
                cursor_visible_utf16: effective_cursor_visible,
                block_start: start,
                block_end: start.saturating_add(fresh_model.raw_text.len()),
            });
        }
    });
}

#[component]
fn ActiveBlockEditor(
    node: OwnedAstNode,
    on_active_block_input: Option<EventHandler<ActiveBlockInputEvent>>,
) -> Element {
    let ctx = use_context::<MarkdownContext>();
    let cursor_ctx = try_use_context::<CursorContext>();
    let raw = ctx.raw_value();

    let safe_end = node.range.end.min(raw.len());
    let safe_start = node.range.start.min(safe_end);
    let initial_text = raw[safe_start..safe_end].trim_end_matches('\n').to_string();
    let block_id = format!("nox-md-active-{}", safe_start);
    let mut current_len = use_signal(|| initial_text.len());

    let target_cursor = cursor_ctx
        .map(|c| c.cursor_position.read().offset.saturating_sub(safe_start))
        .unwrap_or(0);
    let block_id_input = block_id.clone();
    let block_id_keyup = block_id.clone();
    let block_id_mouseup = block_id.clone();
    let block_id_mount = block_id.clone();

    let wrapper = match &node.node_type {
        NodeType::Heading(1) => "h1",
        NodeType::Heading(2) => "h2",
        NodeType::Heading(3) => "h3",
        NodeType::Heading(4) => "h4",
        NodeType::Heading(5) => "h5",
        NodeType::Heading(6) => "h6",
        NodeType::BlockQuote => "blockquote",
        NodeType::CodeBlock(_) => "pre",
        NodeType::Item => "li",
        _ => "p",
    };

    let input_view = rsx! {
        textarea {
            id: "{block_id}",
            "data-md-active-block-editor": "true",
            rows: "1",
            // Ensure active raw editing matches the rendered block width.
            // Without an explicit width, browser default textarea cols can collapse
            // to a narrow measure and cause visual line-wrap jumps.
            style: "width:100%;min-width:100%;max-width:100%;box-sizing:border-box;resize:none;overflow:hidden;font:inherit;color:inherit;background:transparent;border:none;margin:0;padding:0;outline:none;line-height:inherit;display:block;",
            initial_value: "{initial_text}",
            oninput: move |evt: FormEvent| {
                let new_local = evt.value();
                let current_global = ctx.raw_value();
                let start = safe_start.min(current_global.len());
                let old_len = *current_len.read();
                let end = (start + old_len).min(current_global.len());
                let before = &current_global[..start];
                let after = &current_global[end..];
                let new_global = format!("{before}{new_local}{after}");
                current_len.set(new_local.len());
                ctx.handle_value_change(new_global);
                ctx.trigger_parse.call(());

                if cursor_ctx.is_some() || on_active_block_input.is_some() {
                    let block_id = block_id_input.clone();
                    let text_clone = new_local.clone();
                    let block_idx = safe_start;
                    let cursor_ctx_local = cursor_ctx;
                    let handler = on_active_block_input;
                    spawn(async move {
                        let cursor_utf16 = {
                            let js = interop::caret_adapter().read_textarea_cursor_js(&block_id);
                            let mut eval = interop::start_eval(&js);
                            interop::recv_u64(&mut eval).await.unwrap_or(0) as usize
                        };
                        if let Some(mut cctx) = cursor_ctx_local {
                            let local_byte = utf16_to_byte_index(&text_clone, cursor_utf16)
                                .unwrap_or(text_clone.len());
                            let raw_offset = block_idx.saturating_add(local_byte);
                            let max_offset =
                                block_idx.saturating_add(last_caret_offset(&(0..text_clone.len())));
                            cctx.cursor_position.set(CursorPosition {
                                offset: raw_offset.min(max_offset),
                                line: 0,
                                column: 0,
                            });
                        }
                        if let Some(handler) = handler {
                            let text_len = text_clone.len();
                            handler.call(ActiveBlockInputEvent {
                                raw_text: text_clone.clone(),
                                visible_text: text_clone,
                                cursor_raw_utf16: cursor_utf16,
                                cursor_visible_utf16: cursor_utf16,
                                block_start: block_idx,
                                block_end: block_idx.saturating_add(text_len),
                            });
                        }
                    });
                }
            },
            onkeyup: move |_| {
                if let Some(mut cctx) = cursor_ctx {
                    let block_id = block_id_keyup.clone();
                    let current_global = ctx.raw_value();
                    let start = safe_start.min(current_global.len());
                    let old_len = *current_len.read();
                    let end = (start + old_len).min(current_global.len());
                    let block_text = current_global[start..end].to_string();
                    spawn(async move {
                        let cursor_utf16 = {
                            let js = interop::caret_adapter().read_textarea_cursor_js(&block_id);
                            let mut eval = interop::start_eval(&js);
                            interop::recv_u64(&mut eval).await.unwrap_or(0) as usize
                        };
                        let local_byte =
                            utf16_to_byte_index(&block_text, cursor_utf16).unwrap_or(block_text.len());
                        let raw_offset = start.saturating_add(local_byte);
                        let max_offset =
                            start.saturating_add(last_caret_offset(&(0..block_text.len())));
                        cctx.cursor_position.set(CursorPosition {
                            offset: raw_offset.min(max_offset),
                            line: 0,
                            column: 0,
                        });
                    });
                }
            },
            onmouseup: move |_| {
                if let Some(mut cctx) = cursor_ctx {
                    let block_id = block_id_mouseup.clone();
                    let current_global = ctx.raw_value();
                    let start = safe_start.min(current_global.len());
                    let old_len = *current_len.read();
                    let end = (start + old_len).min(current_global.len());
                    let block_text = current_global[start..end].to_string();
                    spawn(async move {
                        let cursor_utf16 = {
                            let js = interop::caret_adapter().read_textarea_cursor_js(&block_id);
                            let mut eval = interop::start_eval(&js);
                            interop::recv_u64(&mut eval).await.unwrap_or(0) as usize
                        };
                        let local_byte =
                            utf16_to_byte_index(&block_text, cursor_utf16).unwrap_or(block_text.len());
                        let raw_offset = start.saturating_add(local_byte);
                        let max_offset =
                            start.saturating_add(last_caret_offset(&(0..block_text.len())));
                        cctx.cursor_position.set(CursorPosition {
                            offset: raw_offset.min(max_offset),
                            line: 0,
                            column: 0,
                        });
                    });
                }
            },
            onmounted: move |_| {
                let js = interop::caret_adapter().mount_active_textarea_js(&block_id_mount, target_cursor);
                let mut len_sig = current_len;
                if let Some(mut cctx) = cursor_ctx {
                    spawn(async move {
                        let mut eval = interop::start_eval(&js);
                        while let Some(msg) = interop::recv_string(&mut eval).await {
                            if msg == "prev" {
                                let parsed = (ctx.parsed_doc)();
                                let target = adjacent_editable_offset(
                                    &parsed.ast,
                                    safe_start,
                                    safe_end,
                                    NavDirection::Prev,
                                );
                                cctx.cursor_position.set(CursorPosition {
                                    offset: target,
                                    line: 0,
                                    column: 0,
                                });
                                continue;
                            }
                            if msg == "next" {
                                let parsed = (ctx.parsed_doc)();
                                let target = adjacent_editable_offset(
                                    &parsed.ast,
                                    safe_start,
                                    safe_end,
                                    NavDirection::Next,
                                );
                                cctx.cursor_position.set(CursorPosition {
                                    offset: target,
                                    line: 0,
                                    column: 0,
                                });
                                continue;
                            }
                            if let Some(rest) = msg.strip_prefix("split:")
                                && let Ok(split_utf16) = rest.parse::<usize>()
                            {
                                let current_global = ctx.raw_value();
                                let start = safe_start.min(current_global.len());
                                let old_len = *len_sig.read();
                                let end = (start + old_len).min(current_global.len());
                                let before = &current_global[..start];
                                let block = &current_global[start..end];
                                let after = &current_global[end..];
                                let split_byte = utf16_to_byte_index(block, split_utf16).unwrap_or(block.len());
                                let split_at = split_byte.min(block.len());
                                let left = &block[..split_at];
                                let right = &block[split_at..];
                                let rebuilt = format!("{before}{left}\n\n{right}{after}");
                                ctx.handle_value_change(rebuilt);
                                ctx.trigger_parse.call(());
                                len_sig.set(left.len());
                                cctx.cursor_position.set(CursorPosition {
                                    offset: start + split_at + 2,
                                    line: 0,
                                    column: 0,
                                });
                            }
                        }
                    });
                } else {
                    spawn(async move {
                        interop::eval_void(&js).await;
                    });
                }
            }
        }
    };

    match wrapper {
        "h1" => rsx! { h1 { {input_view} } },
        "h2" => rsx! { h2 { {input_view} } },
        "h3" => rsx! { h3 { {input_view} } },
        "h4" => rsx! { h4 { {input_view} } },
        "h5" => rsx! { h5 { {input_view} } },
        "h6" => rsx! { h6 { {input_view} } },
        "blockquote" => rsx! { blockquote { {input_view} } },
        "pre" => rsx! { pre { {input_view} } },
        "li" => rsx! { li { {input_view} } },
        _ => rsx! { p { {input_view} } },
    }
}

fn handle_inactive_block_click(
    cursor_ctx: Option<CursorContext>,
    raw: String,
    node: OwnedAstNode,
    block_id: String,
    safe_start: usize,
    safe_end: usize,
) {
    if let Some(mut cctx) = cursor_ctx {
        spawn(async move {
            let js = interop::caret_adapter().read_block_visual_offset_js(&block_id);
            let mut eval = interop::start_eval(&js);
            let visual_utf16 = interop::recv_string(&mut eval)
                .await
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);

            let slice_end = safe_end.min(raw.len());
            let slice_start = safe_start.min(slice_end);
            let editable_end = trim_editable_block_end(&raw, slice_start, slice_end);
            let block_raw = &raw[slice_start..editable_end];
            let mut model_node = node.clone();
            model_node.range = slice_start..editable_end;
            let markers = collect_marker_tokens(&model_node, block_raw, slice_start);
            let visibility = markers
                .iter()
                .enumerate()
                .map(|(idx, _)| MarkerVisibility {
                    marker_idx: idx,
                    visible: false,
                })
                .collect::<Vec<_>>();
            let model = build_tokenized_block(&model_node, &raw, &visibility);
            let visible_byte = visible_utf16_to_raw_offset(&model, visual_utf16);
            let new_offset = slice_start.saturating_add(visible_byte);
            let clamped_offset = new_offset.min(last_caret_offset(&(slice_start..editable_end)));

            cctx.cursor_position.set(CursorPosition {
                offset: clamped_offset,
                line: 0,
                column: 0,
            });
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NavDirection {
    Prev,
    Next,
}

fn adjacent_editable_offset(
    ast: &[OwnedAstNode],
    current_start: usize,
    current_end: usize,
    direction: NavDirection,
) -> usize {
    let mut ranges = Vec::new();
    collect_editable_ranges(ast, &mut ranges);

    match direction {
        NavDirection::Prev => ranges
            .iter()
            .rev()
            .find(|range| range.start < current_start)
            .map(last_caret_offset)
            .unwrap_or(current_start),
        NavDirection::Next => ranges
            .iter()
            .find(|range| range.start > current_start)
            .map(|range| range.start)
            .unwrap_or(last_caret_offset(&(current_start..current_end))),
    }
}

fn collect_editable_ranges(nodes: &[OwnedAstNode], out: &mut Vec<std::ops::Range<usize>>) {
    for node in nodes {
        if is_editable_block(node) {
            out.push(node.range.clone());
            continue;
        }
        collect_editable_ranges(&node.children, out);
    }
}

fn last_caret_offset(range: &std::ops::Range<usize>) -> usize {
    if range.end > range.start {
        range.end.saturating_sub(1)
    } else {
        range.start
    }
}

fn is_navigation_key(key: &str) -> bool {
    matches!(
        key,
        "ArrowLeft"
            | "ArrowRight"
            | "ArrowUp"
            | "ArrowDown"
            | "Home"
            | "End"
            | "PageUp"
            | "PageDown"
    )
}

fn utf16_len(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

fn is_latest_revision(captured: u64, latest: u64) -> bool {
    captured == latest
}

fn trim_editable_block_end(raw: &str, start: usize, end: usize) -> usize {
    let mut trimmed_end = end.min(raw.len());
    while trimmed_end > start {
        let byte = raw.as_bytes()[trimmed_end - 1];
        if byte == b'\n' || byte == b'\r' {
            trimmed_end -= 1;
        } else {
            break;
        }
    }
    trimmed_end
}

fn utf16_to_byte_index(s: &str, utf16_idx: usize) -> Option<usize> {
    let mut utf16_count = 0usize;
    for (byte_idx, ch) in s.char_indices() {
        if utf16_count == utf16_idx {
            return Some(byte_idx);
        }
        utf16_count += ch.len_utf16();
    }
    if utf16_count == utf16_idx {
        Some(s.len())
    } else {
        None
    }
}

fn byte_to_utf16_index(s: &str, byte_idx: usize) -> Option<usize> {
    if byte_idx > s.len() {
        return None;
    }
    let mut utf16_count = 0usize;
    for (idx, ch) in s.char_indices() {
        if idx == byte_idx {
            return Some(utf16_count);
        }
        if idx > byte_idx {
            break;
        }
        utf16_count += ch.len_utf16();
    }
    if byte_idx == s.len() {
        Some(utf16_count)
    } else {
        None
    }
}

fn render_inline_segment(seg: InlineSegment) -> Element {
    match seg.kind {
        SegmentKind::Marker(kind) => {
            let marker_kind = match kind {
                crate::inline_tokens::MarkerKind::Inline => "inline",
                crate::inline_tokens::MarkerKind::BlockPrefix => "block-prefix",
            };
            let text = seg.text.clone();
            rsx! {
                span {
                    "data-md-marker": "{marker_kind}",
                    "data-md-marker-start": "{seg.raw_range.start}",
                    "data-md-marker-end": "{seg.raw_range.end}",
                    "{text}"
                }
            }
        }
        SegmentKind::Text => render_text_with_marks(seg.text, &seg.marks),
    }
}

fn render_text_with_marks(text: String, marks: &[InlineMark]) -> Element {
    if marks.is_empty() {
        return rsx! { "{text}" };
    }

    let mut sorted = marks.to_vec();
    sorted.sort();
    let inner = render_text_with_marks(text, &sorted[1..]);
    match sorted[0] {
        InlineMark::Strong => rsx! { strong { {inner} } },
        InlineMark::Emphasis => rsx! { em { {inner} } },
        InlineMark::Strikethrough => rsx! { del { {inner} } },
        InlineMark::Code => rsx! { code { {inner} } },
        InlineMark::Link | InlineMark::Wikilink => rsx! { a { {inner} } },
        InlineMark::Image | InlineMark::Tag => rsx! { span { {inner} } },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisibleEdit {
    old_start_utf16: usize,
    old_end_utf16: usize,
    replacement: String,
}

fn diff_visible_text(old_text: &str, new_text: &str) -> VisibleEdit {
    let old_chars: Vec<char> = old_text.chars().collect();
    let new_chars: Vec<char> = new_text.chars().collect();

    let mut prefix = 0usize;
    while prefix < old_chars.len()
        && prefix < new_chars.len()
        && old_chars[prefix] == new_chars[prefix]
    {
        prefix += 1;
    }

    let mut old_suffix = old_chars.len();
    let mut new_suffix = new_chars.len();
    while old_suffix > prefix
        && new_suffix > prefix
        && old_chars[old_suffix - 1] == new_chars[new_suffix - 1]
    {
        old_suffix -= 1;
        new_suffix -= 1;
    }

    let old_start_utf16 = old_chars[..prefix]
        .iter()
        .map(|ch| ch.len_utf16())
        .sum::<usize>();
    let old_end_utf16 = old_chars[..old_suffix]
        .iter()
        .map(|ch| ch.len_utf16())
        .sum::<usize>();
    let replacement = new_chars[prefix..new_suffix].iter().collect::<String>();

    VisibleEdit {
        old_start_utf16,
        old_end_utf16,
        replacement,
    }
}

fn build_plain_text_model(block_raw: &str, block_start: usize) -> TokenizedBlock {
    let visible_utf16_end = utf16_len(block_raw);
    TokenizedBlock {
        raw_text: block_raw.to_string(),
        block_start,
        block_end: block_start.saturating_add(block_raw.len()),
        segments: vec![InlineSegment {
            raw_range: 0..block_raw.len(),
            text: block_raw.to_string(),
            marks: vec![],
            kind: SegmentKind::Text,
            visible_utf16_start: 0,
            visible_utf16_end,
        }],
        visible_text: block_raw.to_string(),
    }
}

fn select_best_input_projection(
    candidates: &[TokenizedBlock],
    new_visible: &str,
    cursor_visible_utf16: usize,
) -> (usize, VisibleEdit) {
    let first = diff_visible_text(&candidates[0].visible_text, new_visible);
    let mut best_idx = 0usize;
    let mut best_edit = first;
    let mut best_rank = visible_edit_rank(&best_edit, cursor_visible_utf16);

    for (idx, candidate) in candidates.iter().enumerate().skip(1) {
        let candidate_edit = diff_visible_text(&candidate.visible_text, new_visible);
        let candidate_rank = visible_edit_rank(&candidate_edit, cursor_visible_utf16);
        if candidate_rank < best_rank {
            best_idx = idx;
            best_edit = candidate_edit;
            best_rank = candidate_rank;
        }
    }

    (best_idx, best_edit)
}

fn visible_edit_rank(edit: &VisibleEdit, cursor_visible_utf16: usize) -> (usize, usize, usize) {
    let removed = edit.old_end_utf16.saturating_sub(edit.old_start_utf16);
    let inserted = utf16_len(&edit.replacement);
    let span = removed.saturating_add(inserted);
    let distance = if cursor_visible_utf16 == 0 {
        0
    } else {
        cursor_visible_utf16.abs_diff(edit.old_start_utf16)
    };
    (span, distance, edit.old_start_utf16)
}

fn parse_selection_details(raw: &str) -> Option<SelectionDetails> {
    let mut parts = raw.splitn(3, '\u{1f}');
    let start = parts.next()?.parse::<usize>().ok()?;
    let end = parts.next()?.parse::<usize>().ok()?;
    let collapsed = matches!(parts.next()?, "1" | "true");
    Some(SelectionDetails {
        start,
        end,
        collapsed,
    })
}

fn parse_before_input_meta(raw: &str) -> Option<BeforeInputMeta> {
    if raw.is_empty() {
        return None;
    }
    let mut parts = raw.splitn(5, '\u{1f}');
    let start = parts.next()?.parse::<usize>().ok()?;
    let end = parts.next()?.parse::<usize>().ok()?;
    let is_collapsed = matches!(parts.next()?, "1" | "true");
    let input_type = parts.next()?.to_string();
    let data = parts.next().unwrap_or_default().to_string();

    Some(BeforeInputMeta {
        input_type,
        data,
        pre_visible_caret_utf16: start,
        pre_visible_selection_end_utf16: end,
        is_collapsed,
    })
}

fn compute_post_visible_caret(
    meta: Option<&BeforeInputMeta>,
    edit: &VisibleEdit,
    fallback_cursor_visible_utf16: usize,
    new_visible_utf16_len: usize,
) -> usize {
    let inserted_utf16 = utf16_len(&edit.replacement);

    if let Some(meta) = meta {
        if meta.is_collapsed {
            if meta.input_type == "insertText" {
                let typed_utf16 = utf16_len(&meta.data);
                if typed_utf16 > 0 {
                    return meta
                        .pre_visible_caret_utf16
                        .saturating_add(typed_utf16)
                        .min(new_visible_utf16_len);
                }
            }
            if meta.input_type.starts_with("deleteContent") {
                return edit
                    .old_start_utf16
                    .saturating_add(inserted_utf16)
                    .min(new_visible_utf16_len);
            }
        } else {
            return edit
                .old_start_utf16
                .saturating_add(inserted_utf16)
                .min(new_visible_utf16_len);
        }
    }

    normalize_cursor_visible_for_edit(fallback_cursor_visible_utf16, edit, new_visible_utf16_len)
}

fn normalize_cursor_visible_for_edit(
    cursor_visible_utf16: usize,
    edit: &VisibleEdit,
    new_visible_utf16_len: usize,
) -> usize {
    let replacement_utf16 = utf16_len(&edit.replacement);
    let inferred = edit.old_start_utf16.saturating_add(replacement_utf16);

    // For insertion-style edits, pin caret to end-of-replacement deterministically.
    // Browser-reported offsets can drift by one around adjacent text-node merges.
    if edit.old_start_utf16 == edit.old_end_utf16 && replacement_utf16 > 0 {
        return inferred.min(new_visible_utf16_len);
    }

    cursor_visible_utf16.min(new_visible_utf16_len)
}

fn cursor_after_visible_edit(
    old_model: &TokenizedBlock,
    new_cursor_visible_utf16: usize,
    edit: &VisibleEdit,
    old_raw_start: usize,
    old_raw_end: usize,
) -> usize {
    let old_visible_utf16 = old_model
        .visible_text
        .chars()
        .map(char::len_utf16)
        .sum::<usize>();
    let replacement_utf16 = edit.replacement.chars().map(char::len_utf16).sum::<usize>();
    let new_visible_utf16 = old_visible_utf16
        .saturating_sub(edit.old_end_utf16.saturating_sub(edit.old_start_utf16))
        .saturating_add(replacement_utf16);
    let visible_delta = new_visible_utf16 as isize - old_visible_utf16 as isize;
    let raw_delta =
        edit.replacement.len() as isize - (old_raw_end.saturating_sub(old_raw_start)) as isize;

    if new_cursor_visible_utf16 <= edit.old_start_utf16 {
        return visible_utf16_to_raw_offset(old_model, new_cursor_visible_utf16);
    }

    let replacement_end_utf16 = edit.old_start_utf16.saturating_add(replacement_utf16);
    if new_cursor_visible_utf16 < replacement_end_utf16 {
        let in_repl_utf16 = new_cursor_visible_utf16.saturating_sub(edit.old_start_utf16);
        let in_repl_byte =
            utf16_to_byte_index(&edit.replacement, in_repl_utf16).unwrap_or(edit.replacement.len());
        return old_raw_start.saturating_add(in_repl_byte);
    }

    let old_cursor_visible = (new_cursor_visible_utf16 as isize - visible_delta)
        .max(edit.old_end_utf16 as isize) as usize;
    let old_raw_cursor = visible_utf16_to_raw_offset(old_model, old_cursor_visible);
    (old_raw_cursor as isize + raw_delta).max(0) as usize
}

#[cfg(test)]
mod tests {
    use super::{
        BeforeInputMeta, VisibleEdit, compute_post_visible_caret,
        NavDirection, adjacent_editable_offset, block_has_inline_markup,
        cursor_within_inline_markup, is_latest_revision, normalize_cursor_visible_for_edit,
        select_best_input_projection, uses_token_aware_surface,
    };
    use crate::inline_tokens::TokenizedBlock;
    use crate::types::{NodeType, OwnedAstNode};

    fn text_node(start: usize, end: usize, text: &str) -> OwnedAstNode {
        OwnedAstNode {
            node_type: NodeType::Text(text.to_string()),
            range: start..end,
            children: vec![],
        }
    }

    #[test]
    fn plain_paragraph_is_always_editable() {
        let node = OwnedAstNode {
            node_type: NodeType::Paragraph,
            range: 0..18,
            children: vec![text_node(0, 18, "plain text only")],
        };
        assert!(!block_has_inline_markup(&node));
        assert!(!uses_token_aware_surface(&node));
    }

    #[test]
    fn mixed_paragraph_uses_token_aware_surface() {
        let strong = OwnedAstNode {
            node_type: NodeType::Strong,
            range: 19..27, // **er**
            children: vec![text_node(21, 23, "er")],
        };
        let node = OwnedAstNode {
            node_type: NodeType::Paragraph,
            range: 0..55,
            children: vec![
                text_node(0, 19, "Borrowing lets you ref"),
                strong,
                text_node(27, 55, "ence data without taking ownership."),
            ],
        };

        assert!(block_has_inline_markup(&node));
        assert!(cursor_within_inline_markup(&node, 22)); // inside "er"
        assert!(!cursor_within_inline_markup(&node, 10)); // plain text
        assert!(uses_token_aware_surface(&node));
    }

    #[test]
    fn nav_next_skips_non_editable_gap() {
        let ast = vec![
            OwnedAstNode {
                node_type: NodeType::Paragraph,
                range: 0..5,
                children: vec![text_node(0, 5, "first")],
            },
            OwnedAstNode {
                node_type: NodeType::Paragraph,
                range: 7..12,
                children: vec![text_node(7, 12, "second")],
            },
        ];

        let next = adjacent_editable_offset(&ast, 0, 5, NavDirection::Next);
        assert_eq!(next, 7);
    }

    #[test]
    fn nav_prev_targets_previous_editable_block_end() {
        let ast = vec![
            OwnedAstNode {
                node_type: NodeType::Paragraph,
                range: 0..5,
                children: vec![text_node(0, 5, "first")],
            },
            OwnedAstNode {
                node_type: NodeType::Paragraph,
                range: 7..12,
                children: vec![text_node(7, 12, "second")],
            },
        ];

        let prev = adjacent_editable_offset(&ast, 7, 12, NavDirection::Prev);
        assert_eq!(prev, 4);
    }

    #[test]
    fn revision_guard_accepts_latest_only() {
        assert!(is_latest_revision(4, 4));
        assert!(!is_latest_revision(3, 4));
    }

    #[test]
    fn projection_selector_prefers_closest_visible_model() {
        let plain = TokenizedBlock {
            raw_text: "ref**er**ence".to_string(),
            block_start: 0,
            block_end: 12,
            segments: vec![],
            visible_text: "reference".to_string(),
        };
        let raw_like = TokenizedBlock {
            raw_text: "ref**er**ence".to_string(),
            block_start: 0,
            block_end: 12,
            segments: vec![],
            visible_text: "ref**er**ence".to_string(),
        };
        let (idx, _) = select_best_input_projection(
            &[plain, raw_like],
            "ref**er**ence!",
            "ref**er**ence!".chars().count(),
        );
        assert_eq!(idx, 1);
    }

    #[test]
    fn insertion_cursor_normalization_avoids_transient_zero_jump() {
        let edit = VisibleEdit {
            old_start_utf16: 10,
            old_end_utf16: 10,
            replacement: "*".to_string(),
        };
        let normalized = normalize_cursor_visible_for_edit(0, &edit, 24);
        assert_eq!(normalized, 11);
    }

    #[test]
    fn compute_post_caret_uses_beforeinput_for_collapsed_star_insert() {
        let edit = VisibleEdit {
            old_start_utf16: 57,
            old_end_utf16: 57,
            replacement: "*".to_string(),
        };
        let meta = BeforeInputMeta {
            input_type: "insertText".to_string(),
            data: "*".to_string(),
            pre_visible_caret_utf16: 57,
            pre_visible_selection_end_utf16: 57,
            is_collapsed: true,
        };
        let post = compute_post_visible_caret(Some(&meta), &edit, 56, 80);
        assert_eq!(post, 58);
    }

    #[test]
    fn compute_post_caret_collapses_noncollapsed_insert_to_end_of_replacement() {
        let edit = VisibleEdit {
            old_start_utf16: 20,
            old_end_utf16: 22,
            replacement: "**".to_string(),
        };
        let meta = BeforeInputMeta {
            input_type: "insertText".to_string(),
            data: "**".to_string(),
            pre_visible_caret_utf16: 20,
            pre_visible_selection_end_utf16: 22,
            is_collapsed: false,
        };
        let post = compute_post_visible_caret(Some(&meta), &edit, 0, 80);
        assert_eq!(post, 22);
    }

    #[test]
    fn compute_post_caret_delete_prefers_edit_window_start() {
        let edit = VisibleEdit {
            old_start_utf16: 11,
            old_end_utf16: 12,
            replacement: String::new(),
        };
        let meta = BeforeInputMeta {
            input_type: "deleteContentBackward".to_string(),
            data: String::new(),
            pre_visible_caret_utf16: 12,
            pre_visible_selection_end_utf16: 12,
            is_collapsed: true,
        };
        let post = compute_post_visible_caret(Some(&meta), &edit, 12, 80);
        assert_eq!(post, 11);
    }
}
