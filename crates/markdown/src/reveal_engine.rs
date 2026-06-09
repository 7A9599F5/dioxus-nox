use std::ops::Range;

use crate::inline_tokens::{MarkerKind, MarkerToken};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionAnchor {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevealContext {
    pub caret_raw_offset: usize,
    pub selection: Option<SelectionAnchor>,
}

/// Returns whether a marker should be visible under the current caret/selection.
///
/// Obsidian parity rules encoded here:
/// - Inline delimiter markers are revealed when caret/selection is inside the token envelope.
/// - Block-prefix markers are revealed only in a tight marker-adjacent window.
pub fn marker_visible(marker: &MarkerToken, ctx: RevealContext) -> bool {
    if let Some(sel) = ctx.selection {
        let selection_range = sel.start.min(sel.end)..sel.start.max(sel.end);
        if ranges_overlap(&selection_range, &marker.token_range) {
            return true;
        }
    }

    match marker.kind {
        MarkerKind::Inline => in_range(ctx.caret_raw_offset, &marker.token_range),
        MarkerKind::BlockPrefix => {
            let start = marker.raw_range.start.saturating_sub(1);
            let end = marker.raw_range.end.saturating_add(1);
            ctx.caret_raw_offset >= start && ctx.caret_raw_offset <= end
        }
    }
}

/// Compute visibility for a marker set in one pass, enforcing a single active
/// inline token envelope (the innermost token containing caret/selection).
///
/// This avoids revealing unrelated inline markers on the same line.
pub fn marker_visibility(markers: &[MarkerToken], ctx: RevealContext) -> Vec<bool> {
    let active_inline = active_inline_token(markers, ctx);
    markers
        .iter()
        .map(|marker| match marker.kind {
            MarkerKind::Inline => active_inline
                .as_ref()
                .is_some_and(|active| marker.token_range == *active),
            MarkerKind::BlockPrefix => {
                if let Some(sel) = ctx.selection {
                    let selection_range = sel.start.min(sel.end)..sel.start.max(sel.end);
                    if ranges_overlap(&selection_range, &marker.token_range) {
                        return true;
                    }
                }
                let start = marker.raw_range.start.saturating_sub(1);
                let end = marker.raw_range.end.saturating_add(1);
                ctx.caret_raw_offset >= start && ctx.caret_raw_offset <= end
            }
        })
        .collect()
}

fn active_inline_token(markers: &[MarkerToken], ctx: RevealContext) -> Option<Range<usize>> {
    let mut candidates: Vec<Range<usize>> = Vec::new();
    for marker in markers {
        if marker.kind != MarkerKind::Inline {
            continue;
        }
        let hit = if let Some(sel) = ctx.selection {
            let selection_range = sel.start.min(sel.end)..sel.start.max(sel.end);
            ranges_overlap(&selection_range, &marker.token_range)
        } else {
            in_range(ctx.caret_raw_offset, &marker.token_range)
        };
        if !hit {
            continue;
        }
        if !candidates.contains(&marker.token_range) {
            candidates.push(marker.token_range.clone());
        }
    }

    candidates
        .into_iter()
        .min_by_key(|range| (range.end.saturating_sub(range.start), range.start))
}

fn in_range(offset: usize, range: &Range<usize>) -> bool {
    // Half-open at the trailing edge: a caret exactly at `range.end` (just past the
    // closing marker, e.g. `**bold**|`) is OUTSIDE the token, so its markers stay
    // hidden and Backspace deletes content, not a literal `*`. The leading edge
    // stays inclusive (caret just before the opening marker reveals). This also
    // makes two adjacent tokens sharing a boundary resolve to the one being entered.
    offset >= range.start && offset < range.end
}

fn ranges_overlap(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < b.end && b.start < a.end
}

#[cfg(test)]
mod tests {
    use super::{RevealContext, SelectionAnchor, marker_visibility, marker_visible};
    use crate::inline_tokens::{MarkerKind, MarkerToken};

    #[test]
    fn inline_marker_visible_inside_token() {
        let marker = MarkerToken {
            raw_range: 10..12,
            token_range: 10..20,
            kind: MarkerKind::Inline,
        };
        let ctx = RevealContext {
            caret_raw_offset: 15,
            selection: None,
        };
        assert!(marker_visible(&marker, ctx));
    }

    #[test]
    fn inline_marker_hidden_outside_token() {
        let marker = MarkerToken {
            raw_range: 10..12,
            token_range: 10..20,
            kind: MarkerKind::Inline,
        };
        let ctx = RevealContext {
            caret_raw_offset: 4,
            selection: None,
        };
        assert!(!marker_visible(&marker, ctx));
    }

    #[test]
    fn block_marker_visible_in_adjacent_window() {
        let marker = MarkerToken {
            raw_range: 2..4,
            token_range: 2..4,
            kind: MarkerKind::BlockPrefix,
        };
        let visible = RevealContext {
            caret_raw_offset: 1,
            selection: None,
        };
        let hidden = RevealContext {
            caret_raw_offset: 8,
            selection: None,
        };
        assert!(marker_visible(&marker, visible));
        assert!(!marker_visible(&marker, hidden));
    }

    #[test]
    fn selection_inside_token_reveals_inline_marker() {
        let marker = MarkerToken {
            raw_range: 10..12,
            token_range: 10..20,
            kind: MarkerKind::Inline,
        };
        let ctx = RevealContext {
            caret_raw_offset: 0,
            selection: Some(SelectionAnchor { start: 13, end: 14 }),
        };
        assert!(marker_visible(&marker, ctx));
    }

    #[test]
    fn caret_in_second_token_reveals_only_second_markers() {
        let markers = vec![
            MarkerToken {
                raw_range: 3..5,
                token_range: 3..11,
                kind: MarkerKind::Inline,
            },
            MarkerToken {
                raw_range: 9..11,
                token_range: 3..11,
                kind: MarkerKind::Inline,
            },
            MarkerToken {
                raw_range: 24..26,
                token_range: 24..32,
                kind: MarkerKind::Inline,
            },
            MarkerToken {
                raw_range: 30..32,
                token_range: 24..32,
                kind: MarkerKind::Inline,
            },
        ];
        let vis = marker_visibility(
            &markers,
            RevealContext {
                caret_raw_offset: 27,
                selection: None,
            },
        );
        assert_eq!(vis, vec![false, false, true, true]);
    }

    #[test]
    fn unrelated_inline_markers_stay_hidden() {
        let markers = vec![
            MarkerToken {
                raw_range: 3..5,
                token_range: 3..11,
                kind: MarkerKind::Inline,
            },
            MarkerToken {
                raw_range: 9..11,
                token_range: 3..11,
                kind: MarkerKind::Inline,
            },
            MarkerToken {
                raw_range: 16..18,
                token_range: 16..24,
                kind: MarkerKind::Inline,
            },
            MarkerToken {
                raw_range: 22..24,
                token_range: 16..24,
                kind: MarkerKind::Inline,
            },
        ];
        let vis = marker_visibility(
            &markers,
            RevealContext {
                caret_raw_offset: 5,
                selection: None,
            },
        );
        assert_eq!(vis, vec![true, true, false, false]);
    }

    #[test]
    fn caret_just_past_closing_marker_hides_it() {
        // `**bold**` envelope 0..8; caret at 8 (just past the closing `**`) is
        // OUTSIDE the token → markers hidden, so Backspace there deletes content.
        let markers = vec![
            MarkerToken { raw_range: 0..2, token_range: 0..8, kind: MarkerKind::Inline },
            MarkerToken { raw_range: 6..8, token_range: 0..8, kind: MarkerKind::Inline },
        ];
        let at_end = marker_visibility(&markers, RevealContext { caret_raw_offset: 8, selection: None });
        assert_eq!(at_end, vec![false, false]);
        // Caret just inside the closing content (offset 6, end of "bold") still reveals.
        let inside = marker_visibility(&markers, RevealContext { caret_raw_offset: 6, selection: None });
        assert_eq!(inside, vec![true, true]);
    }

    #[test]
    fn adjacent_tokens_resolve_to_the_one_being_entered() {
        // Token A 0..8 and token B 8..16 share boundary 8. Caret at 8 reveals B, not A.
        let markers = vec![
            MarkerToken { raw_range: 0..2, token_range: 0..8, kind: MarkerKind::Inline },
            MarkerToken { raw_range: 6..8, token_range: 0..8, kind: MarkerKind::Inline },
            MarkerToken { raw_range: 8..10, token_range: 8..16, kind: MarkerKind::Inline },
            MarkerToken { raw_range: 14..16, token_range: 8..16, kind: MarkerKind::Inline },
        ];
        let vis = marker_visibility(&markers, RevealContext { caret_raw_offset: 8, selection: None });
        assert_eq!(vis, vec![false, false, true, true]);
    }
}
