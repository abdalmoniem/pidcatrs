// Copyright (C) 2026 AbdAlMoniem AlHifnawy
//
// This file is part of pidcatrs.
//
// pidcatrs is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Multiline filter field layout and syntax coloring for the filter bar.

#![deny(clippy::unwrap_used)]

use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;

use crate::FilterExpr;
use crate::FilterKey;
use crate::SourceSpan;
use crate::parse_query;

use super::theme;

/// Soft-wraps `text` into lines of at most `width` columns (character-based).
///
/// Every character in `text` is preserved, including spaces and trailing whitespace.
pub fn wrap_filter_lines(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    if text.is_empty() {
        return vec![String::new()];
    }

    let mut lines = vec![String::new()];
    let mut col = 0usize;

    for ch in text.chars() {
        if col >= width {
            lines.push(String::new());
            col = 0;
        }
        if let Some(last) = lines.last_mut() {
            last.push(ch);
        }
        col += 1;
    }

    lines
}

/// First wrapped line of `text`, truncated to `width` with an ellipsis when it overflows.
pub fn collapsed_filter_display(text: &str, width: usize) -> String {
    let width = width.max(1);
    if text.is_empty() {
        return String::new();
    }
    let lines = wrap_filter_lines(text, width);
    let first = lines.first().cloned().unwrap_or_default();
    if lines.len() <= 1 && first.chars().count() <= width {
        return first;
    }
    if width <= 1 {
        return "…".to_string();
    }
    let mut out: String = first.chars().take(width.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Outer panel height for the filter bar (top border + content rows + bottom border).
pub fn filter_panel_height(content_lines: usize) -> u16 {
    (2 + content_lines.max(1)) as u16
}

/// Maps a global character cursor index to `(row, col)` in wrapped `text`.
pub fn cursor_wrapped_position(text: &str, width: usize, cursor: usize) -> (usize, usize) {
    let width = width.max(1);
    let mut row = 0usize;
    let mut col = 0usize;

    for (index, _ch) in text.chars().enumerate() {
        if index == cursor {
            return (row, col);
        }
        if col >= width {
            row += 1;
            col = 0;
        }
        col += 1;
    }

    (row, col)
}

/// Builds styled lines for the filter field (syntax-colored when `input` is non-empty).
pub fn styled_filter_content(
    input: &str,
    width: usize,
    focused: bool,
    placeholder: &str,
) -> Vec<Line<'static>> {
    if input.is_empty() {
        let text = if focused {
            String::new()
        } else {
            placeholder.to_string()
        };
        return vec![Line::from(Span::styled(text, theme::placeholder_style()))];
    }

    let width = width.max(1);
    let accent = Style::default().fg(theme::accent());
    let accent_chars = accent_char_mask(input);

    if focused {
        let mut lines_out = Vec::default();
        let mut line = String::new();
        let mut col = 0usize;
        let mut global = 0usize;

        for ch in input.chars() {
            if col >= width {
                lines_out.push(line_from_char_mask(
                    &line,
                    global - line.chars().count(),
                    &accent_chars,
                    accent,
                ));
                line.clear();
                col = 0;
            }
            line.push(ch);
            col += 1;
            global += 1;
        }
        if line.is_empty() && lines_out.is_empty() {
            lines_out.push(Line::from(String::new()));
        } else if !line.is_empty() {
            let line_start = global - line.chars().count();
            lines_out.push(line_from_char_mask(
                &line,
                line_start,
                &accent_chars,
                accent,
            ));
        }
        lines_out
    } else {
        let display = collapsed_filter_display(input, width);
        vec![line_from_char_mask(&display, 0, &accent_chars, accent)]
    }
}

fn line_from_char_mask(
    line: &str,
    global_offset: usize,
    mask: &[bool],
    accent: Style,
) -> Line<'static> {
    if line.is_empty() {
        return Line::from(String::new());
    }
    let mut pieces: Vec<Span<'static>> = Vec::default();
    let mut run = String::new();
    let mut run_accent = false;
    for (local, ch) in line.chars().enumerate() {
        let accented = mask.get(global_offset + local).copied().unwrap_or(false);
        if pieces.is_empty() && run.is_empty() {
            run_accent = accented;
        }
        if accented == run_accent {
            run.push(ch);
        } else {
            push_run(&mut pieces, run, run_accent, accent);
            run = ch.to_string();
            run_accent = accented;
        }
    }
    push_run(&mut pieces, run, run_accent, accent);
    Line::from(pieces)
}

fn push_run(pieces: &mut Vec<Span<'static>>, text: String, accented: bool, accent: Style) {
    if text.is_empty() {
        return;
    }
    if accented {
        pieces.push(Span::styled(text, accent));
    } else {
        pieces.push(Span::raw(text));
    }
}

/// Marks filter keys and operators (e.g. `tag:` / `message~`) for accent coloring — not values or bare message terms.
fn accent_char_mask(input: &str) -> Vec<bool> {
    let char_len = input.chars().count();
    let mut mask = vec![false; char_len];
    let Ok(query) = parse_query(input) else {
        return mask;
    };
    let Some(root) = query.root else {
        return mask;
    };
    let mut spans = Vec::default();
    collect_key_spans(&root, &mut spans);
    for span in spans {
        let start = input[..span.start.min(input.len())].chars().count();
        let end = input[..span.end.min(input.len())].chars().count();
        for slot in mask.iter_mut().take(end.min(char_len)).skip(start) {
            *slot = true;
        }
    }
    mask
}

fn collect_key_spans(expr: &FilterExpr, out: &mut Vec<SourceSpan>) {
    match expr {
        FilterExpr::Term(term) => {
            if let Some(span) = term.key_span {
                out.push(span);
            }
        }
        FilterExpr::And(parts) | FilterExpr::Or(parts) => {
            for part in parts {
                collect_key_spans(part, out);
            }
        }
        FilterExpr::Group(inner) | FilterExpr::Not(inner) => {
            collect_key_spans(inner, out);
        }
    }
}

/// Key-name prefix at `cursor` for autocomplete, or `Some("")` when all keys apply.
///
/// Returns `None` when the cursor is not in a key position (e.g. after `:` in a value).
pub fn completion_prefix_at_cursor(input: &str, cursor: usize) -> Option<String> {
    let byte = char_index_to_byte(input, cursor);
    let before = &input[..byte];
    let segment_start = before
        .rfind(|c: char| c.is_whitespace() || c == '(' || c == '|')
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let segment = &before[segment_start..];
    if segment.contains(':') || segment.contains('~') {
        return None;
    }
    let prefix = segment.trim_start_matches('-');
    if !prefix.is_empty()
        && !prefix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return None;
    }
    Some(prefix.to_ascii_lowercase())
}

fn char_index_to_byte(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(byte, _)| byte)
        .unwrap_or(text.len())
}

/// Returns `key:` and (when supported) `key~` suggestions matching `prefix`.
pub fn matching_filter_suggestions(prefix: &str) -> Vec<(FilterKey, bool)> {
    FilterKey::autocomplete_keys()
        .iter()
        .copied()
        .filter(|key| prefix.is_empty() || key.as_str().starts_with(prefix))
        .flat_map(|key| {
            let mut rows = vec![(key, false)];
            if key.supports_regex_operator() {
                rows.push((key, true));
            }
            rows
        })
        .collect()
}
