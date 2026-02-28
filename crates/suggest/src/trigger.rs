/// Convert a UTF-16 code-unit index into a byte offset in `text`.
///
/// Returns `None` if the index is out of range.
pub(crate) fn utf16_to_byte_index(text: &str, utf16_idx: usize) -> Option<usize> {
    if utf16_idx == 0 {
        return Some(0);
    }
    let mut count = 0usize;
    for (byte_idx, ch) in text.char_indices() {
        if count >= utf16_idx {
            return Some(byte_idx);
        }
        count += ch.len_utf16();
    }
    if count == utf16_idx {
        Some(text.len())
    } else {
        None
    }
}

/// Returns the byte offset of `trigger_char` in `text` if the trigger conditions are met.
///
/// `cursor_utf16` is the JS `selectionStart` position (UTF-16 code units).
///
/// # Line-start mode (`line_start_only = true`)
///
/// The trigger char must appear at the start of the current line (position 0 or
/// immediately after a newline). The cursor must be positioned after it.
///
/// # Word-boundary mode (`line_start_only = false`)
///
/// The trigger char must appear at the start of the current "word" — immediately
/// after whitespace or at position 0 in the text.
pub fn detect_trigger(
    text: &str,
    cursor_utf16: usize,
    trigger_char: char,
    line_start_only: bool,
) -> Option<usize> {
    if cursor_utf16 == 0 {
        return None;
    }
    let cursor = utf16_to_byte_index(text, cursor_utf16)?;
    let before = &text[..cursor];

    if line_start_only {
        let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_content = &before[line_start..];
        if line_content.starts_with(trigger_char) {
            Some(line_start)
        } else {
            None
        }
    } else {
        // Scan backwards for the most recent occurrence of `trigger_char` that
        // is either at position 0 or immediately preceded by whitespace.
        //
        // This is intentionally independent of `allow_spaces`: `detect_trigger`
        // finds WHERE the trigger is; `extract_filter` validates the filter text
        // (rejecting it when spaces are found and `allow_spaces = false`).
        before.char_indices().rev().find_map(|(byte_pos, ch)| {
            if ch != trigger_char {
                return None;
            }
            let preceded_by_whitespace = byte_pos == 0
                || before[..byte_pos]
                    .chars()
                    .last()
                    .is_some_and(|c| c.is_whitespace());
            if preceded_by_whitespace {
                Some(byte_pos)
            } else {
                None
            }
        })
    }
}

/// Returns the text typed after `trigger_char` up to the cursor.
///
/// Returns `None` if:
/// - The trigger is not active at the cursor position (see [`detect_trigger`]).
/// - `allow_spaces = false` and the filter contains spaces or newlines.
/// - The filter text exceeds `max_filter_len` bytes.
pub fn extract_filter(
    text: &str,
    cursor_utf16: usize,
    trigger_char: char,
    line_start_only: bool,
    allow_spaces: bool,
    max_filter_len: usize,
) -> Option<String> {
    let trigger_offset = detect_trigger(text, cursor_utf16, trigger_char, line_start_only)?;
    let cursor = utf16_to_byte_index(text, cursor_utf16)?;
    let trigger_char_len = trigger_char.len_utf8();
    if trigger_offset + trigger_char_len > cursor {
        return None;
    }
    let filter = &text[trigger_offset + trigger_char_len..cursor];
    if !allow_spaces && (filter.contains(' ') || filter.contains('\n')) {
        return None;
    }
    if filter.len() > max_filter_len {
        return None;
    }
    Some(filter.to_string())
}
