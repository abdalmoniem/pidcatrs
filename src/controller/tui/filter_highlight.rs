// Copyright (C) 2026 AbdAlMoniem AlHifnawy
//
// This file is part of pidcatrs.
//
// pidcatrs is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// pidcatrs is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with pidcatrs.  If not, see <https://www.gnu.org/licenses/>.
//
// Author: AbdAlMoniem AlHifnawy

//! Highlighting filter tokens inside rendered log lines.
//!
//! The display cache renders each log entry to one or more ratatui [`Line`]s (wrapped messages
//! become multiple rows). When the TUI filter bar is active and colors are enabled, this module
//! locates filter needles in header columns and in the message body, including matches that span
//! a wrap boundary, and repaints those character ranges with a contrasting background.

#![deny(clippy::unwrap_used)]

use std::ops::Range;

use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;

use crate::CliArgs;
use crate::LogEntry;
use crate::LogEntryKind;
use crate::State;
use crate::TuiFilterSet;
use crate::controller::log_processor::log_line_column_ranges_for_entry;
use crate::model::tui_filter::FilterHighlightField;
use crate::model::tui_filter::FilterHighlightRule;

use super::theme;

/// One rendered row's contribution to a wrapped message body.
///
/// Used with [`WrappedMessageLayout`] to map a filter match in the full plain message back to
/// visible-character indices on a specific output row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WrappedMessageRowLayout {
    /// Index of this row in the entry's rendered output.
    pub line_index: usize,
    /// `[start, end)` in [`WrappedMessageLayout::plain_message`].
    pub global_range: Range<usize>,
    /// Message column within the rendered row (visible character indices).
    pub message_field: Range<usize>,
}

/// Plain message text for one log entry, stitched across all wrapped output rows.
///
/// [`Self::plain_message`] is the message field text in wrap order. Each [`WrappedMessageRowLayout`]
/// records where that row's slice sits inside `plain_message` and which column range on the row
/// contains the slice.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WrappedMessageLayout {
    /// Message plain text in display order across wrapped rows.
    pub plain_message: String,
    /// Per-row slices aligned with rendered output lines.
    pub rows: Vec<WrappedMessageRowLayout>,
}

impl WrappedMessageLayout {
    /// Builds a layout from ANSI-stripped rows and per-row message column ranges.
    ///
    /// # Arguments
    ///
    /// * `plain_lines` - One plain-text row per rendered output line (no escape sequences).
    /// * `message_fields` - Visible-character range of the message on each row; must align with
    ///   `plain_lines` by index. Empty ranges are skipped.
    ///
    /// # Returns
    ///
    /// A layout whose `plain_message` is the concatenation of message slices taken from each row.
    pub fn from_rendered_plain_lines(
        plain_lines: &[String],
        message_fields: &[Range<usize>],
    ) -> Self {
        let mut rows = Vec::default();
        let mut plain_message = String::new();
        let mut global = 0usize;

        for (line_index, plain_line) in plain_lines.iter().enumerate() {
            let Some(message_field) = message_fields.get(line_index) else {
                continue;
            };
            if message_field.is_empty() {
                continue;
            }

            let line_chars: Vec<char> = plain_line.chars().collect();
            let field_end = message_field.end.min(line_chars.len());
            let field_start = message_field.start.min(field_end);
            if field_start >= field_end {
                continue;
            }

            let chunk: String = line_chars[field_start..field_end].iter().collect();
            let chunk_len = chunk.chars().count();
            if chunk_len == 0usize {
                continue;
            }

            rows.push(WrappedMessageRowLayout {
                line_index,
                global_range: global..global + chunk_len,
                message_field: field_start..field_end,
            });
            plain_message.push_str(&chunk);
            global += chunk_len;
        }

        Self {
            plain_message,
            rows,
        }
    }
}

/// Builds highlight scopes for one rendered row of `entry`.
///
/// Each scope is a `(rule, character_range)` pair. Ranges are visible-character indices into the
/// plain text of that row (the same indexing used for [`apply_filter_highlights`]).
///
/// # Arguments
///
/// * `filters` - Active TUI filters; drives which needles and columns are considered.
/// * `entry` - The log entry being rendered on this row.
/// * `args` - CLI layout options (column widths, timestamps, etc.).
/// * `state` - Render state (package maps, tag de-duplication layout).
/// * `line_index` - Index of this row within the entry's rendered lines (`0` = first row).
/// * `plain_line` - ANSI-stripped text of this row.
/// * `message_layout` - Wrapped message layout for the whole entry; required for message-keyword
///   highlights on normal entries. `None` skips message-body highlighting.
///
/// # Returns
///
/// Scoped rules for `line_index`. Empty when there are no rules, the row is empty, or no needles
/// match on this row.
pub fn entry_line_highlight_scopes(
    filters: &TuiFilterSet,
    entry: &LogEntry,
    args: &CliArgs,
    state: &State,
    line_index: usize,
    plain_line: &str,
    message_layout: Option<&WrappedMessageLayout>,
) -> Vec<(FilterHighlightRule, Range<usize>)> {
    let line_char_len = plain_line.chars().count();
    let rules = filters.highlight_rules(entry);
    if rules.is_empty() || line_char_len == 0usize {
        return Vec::default();
    }

    match entry.kind {
        LogEntryKind::ProcessStart | LogEntryKind::ProcessDeath => {
            lifecycle_line_scopes(&rules, line_char_len)
        }
        LogEntryKind::Normal => normal_line_scopes(
            &rules,
            entry,
            args,
            state,
            line_index,
            plain_line,
            message_layout,
        ),
    }
}

/// Highlight scopes for process start/death banner rows.
///
/// Matching filters highlight the entire visible row (lifecycle banners ignore per-column layout).
///
/// # Arguments
///
/// * `rules` - Highlight rules derived from the active filter set.
/// * `line_char_len` - Visible character length of the banner row.
///
/// # Returns
///
/// One full-line scope per applicable rule.
fn lifecycle_line_scopes(
    rules: &[FilterHighlightRule],
    line_char_len: usize,
) -> Vec<(FilterHighlightRule, Range<usize>)> {
    let scope = 0usize..line_char_len;
    rules
        .iter()
        .filter(|rule| {
            matches!(
                rule.field,
                FilterHighlightField::Message
                    | FilterHighlightField::Package
                    | FilterHighlightField::Pid
                    | FilterHighlightField::Uid
            )
        })
        .map(|rule| (rule.clone(), scope.clone()))
        .collect()
}

/// Highlight scopes for one row of a normal log entry.
///
/// Header columns (PID, UID, package, tag) are only scanned on the first row. Message keywords
/// use `message_layout` so needles split by wrapping still highlight on every affected row.
///
/// # Arguments
///
/// * `rules` - Highlight rules derived from the active filter set.
/// * `entry` - The log entry being rendered.
/// * `args` - CLI layout options.
/// * `state` - Render state for column range calculation.
/// * `line_index` - Index of this row within the entry's rendered lines.
/// * `plain_line` - ANSI-stripped text of this row.
/// * `message_layout` - Wrapped message layout for the entry, if message highlighting applies.
///
/// # Returns
///
/// Scoped rules that intersect this row.
fn normal_line_scopes(
    rules: &[FilterHighlightRule],
    entry: &LogEntry,
    args: &CliArgs,
    state: &State,
    line_index: usize,
    plain_line: &str,
    message_layout: Option<&WrappedMessageLayout>,
) -> Vec<(FilterHighlightRule, Range<usize>)> {
    let columns = log_line_column_ranges_for_entry(entry, args, state);
    let mut scoped = Vec::default();

    for rule in rules {
        match rule.field {
            FilterHighlightField::Pid if line_index == 0usize => {
                push_column_scope(&mut scoped, rule, columns.pid.as_ref(), plain_line);
            }
            FilterHighlightField::Uid if line_index == 0usize => {
                push_column_scope(&mut scoped, rule, columns.uid.as_ref(), plain_line);
            }
            FilterHighlightField::Package if line_index == 0usize => {
                push_column_scope(&mut scoped, rule, columns.package.as_ref(), plain_line);
            }
            FilterHighlightField::Tag if line_index == 0usize => {
                push_column_scope(&mut scoped, rule, columns.tag.as_ref(), plain_line);
            }
            FilterHighlightField::Message => {
                if let Some(layout) = message_layout {
                    push_wrapped_message_scopes(&mut scoped, rule, line_index, layout);
                }
            }
            _ => {}
        }
    }

    scoped
}

/// Appends scopes for every needle occurrence inside a fixed column on one row.
///
/// # Arguments
///
/// * `scoped` - Destination list of scoped rules for the current row.
/// * `rule` - Needle and match options to locate in the column.
/// * `column` - Visible-character range of the column on `plain_line`, or `None` to skip.
/// * `plain_line` - ANSI-stripped text of the row.
fn push_column_scope(
    scoped: &mut Vec<(FilterHighlightRule, Range<usize>)>,
    rule: &FilterHighlightRule,
    column: Option<&Range<usize>>,
    plain_line: &str,
) {
    let Some(column) = column else {
        return;
    };

    let plain_chars: Vec<char> = plain_line.chars().collect();
    let line_len = plain_chars.len();
    let col_start = column.start.min(line_len);
    let col_end = column.end.min(line_len);
    if col_start >= col_end {
        return;
    }

    let haystack: String = plain_chars[col_start..col_end].iter().collect();
    for (rel_start, rel_end) in find_match_ranges(&haystack, &rule.needle, rule.case_insensitive) {
        scoped.push((rule.clone(), col_start + rel_start..col_start + rel_end));
    }
}

/// Appends scopes for the portion of a message needle that appears on one wrapped row.
///
/// Matches are searched in [`WrappedMessageLayout::plain_message`], then clipped to the row's
/// segment so a single needle spanning a wrap boundary produces scopes on every row it touches.
///
/// # Arguments
///
/// * `scoped` - Destination list of scoped rules for the current row.
/// * `rule` - Message keyword rule to locate.
/// * `line_index` - Rendered row index within the entry.
/// * `layout` - Wrapped message layout for the full entry.
fn push_wrapped_message_scopes(
    scoped: &mut Vec<(FilterHighlightRule, Range<usize>)>,
    rule: &FilterHighlightRule,
    line_index: usize,
    layout: &WrappedMessageLayout,
) {
    let Some(row) = layout.rows.iter().find(|row| row.line_index == line_index) else {
        return;
    };

    let seg_start = row.global_range.start;
    let seg_end = row.global_range.end;
    let message_field = &row.message_field;

    for (match_start, match_end) in
        find_match_ranges(&layout.plain_message, &rule.needle, rule.case_insensitive)
    {
        let overlap_start = match_start.max(seg_start);
        let overlap_end = match_end.min(seg_end);
        if overlap_start >= overlap_end {
            continue;
        }

        let local_start = message_field.start + (overlap_start - seg_start);
        let local_end = message_field.start + (overlap_end - seg_start);
        if local_start < local_end && local_end <= message_field.end {
            scoped.push((rule.clone(), local_start..local_end));
        }
    }
}

/// Applies filter-token highlights to a parsed log line within the given scopes.
///
/// Each scope range is painted directly (scopes are already the exact character intervals to
/// emphasize). Highlighted runs use the token foreground as background and a contrasting
/// foreground for readability.
///
/// # Arguments
///
/// * `line` - Parsed ratatui line (typically from ANSI-colored render output).
/// * `scoped_rules` - Character ranges on this line to highlight, paired with the rule that
///   produced them (the rule is not re-evaluated here).
///
/// # Returns
///
/// A new line with styled spans, or `line` unchanged when `scoped_rules` is empty or no scope
/// intersects visible text.
pub fn apply_filter_highlights(
    line: Line<'static>,
    scoped_rules: &[(FilterHighlightRule, Range<usize>)],
) -> Line<'static> {
    if scoped_rules.is_empty() {
        return line;
    }

    let plain: String = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect();
    let char_len = plain.chars().count();
    if char_len == 0 {
        return line;
    }

    let mut marked = vec![false; char_len];
    for (_rule, scope) in scoped_rules {
        let scope_start = scope.start.min(char_len);
        let scope_end = scope.end.min(char_len);
        if scope_start >= scope_end {
            continue;
        }

        for slot in marked.iter_mut().take(scope_end).skip(scope_start) {
            *slot = true;
        }
    }

    if !marked.iter().any(|flag| *flag) {
        return line;
    }

    split_line_by_marks(line, &marked)
}

/// Splits styled spans so each marked character run can use the highlight style.
///
/// # Arguments
///
/// * `line` - Source line whose span styles are preserved for unmarked runs.
/// * `marked` - One flag per visible character in the line; `true` means highlight.
///
/// # Returns
///
/// A line with additional spans at highlight boundaries.
fn split_line_by_marks(line: Line<'static>, marked: &[bool]) -> Line<'static> {
    let mut new_spans = Vec::default();
    let mut char_index = 0usize;

    for span in line.spans {
        let chars: Vec<char> = span.content.chars().collect();
        let mut local = 0usize;
        while local < chars.len() {
            let highlight = marked[char_index + local];
            let mut run_end = local + 1usize;
            while run_end < chars.len() && marked[char_index + run_end] == highlight {
                run_end += 1;
            }

            let text: String = chars[local..run_end].iter().collect();
            let style = if highlight {
                let token_fg = effective_foreground(span.style);
                Style::default()
                    .fg(contrasting_foreground(token_fg))
                    .bg(token_fg)
            } else {
                span.style
            };
            new_spans.push(Span::styled(text, style));
            local = run_end;
        }
        char_index += chars.len();
    }

    Line::from(new_spans)
}

/// Finds every non-overlapping occurrence of `needle` in `haystack` by visible character index.
///
/// # Arguments
///
/// * `haystack` - Text to search (Unicode scalar values; no normalization).
/// * `needle` - Substring to find; empty needles yield no matches.
/// * `case_insensitive` - When set, ASCII case folding is applied per character.
///
/// # Returns
///
/// `(start, end)` ranges with `start` inclusive and `end` exclusive, measured in character
/// indices consistent with the rest of this module.
fn find_match_ranges(haystack: &str, needle: &str, case_insensitive: bool) -> Vec<(usize, usize)> {
    if needle.is_empty() {
        return Vec::default();
    }

    let hay_chars: Vec<char> = haystack.chars().collect();
    let needle_chars: Vec<char> = needle.chars().collect();
    let needle_len = needle_chars.len();
    if needle_len == 0 || needle_len > hay_chars.len() {
        return Vec::default();
    }

    let mut ranges = Vec::default();
    let last_start = hay_chars.len() - needle_len;

    for start in 0..=last_start {
        let mut matches = true;
        for offset in 0..needle_len {
            let hay = hay_chars[start + offset];
            let ndl = needle_chars[offset];
            if case_insensitive {
                if !hay.eq_ignore_ascii_case(&ndl) {
                    matches = false;
                    break;
                }
            } else if hay != ndl {
                matches = false;
                break;
            }
        }
        if matches {
            ranges.push((start, start + needle_len));
        }
    }

    ranges
}

/// Resolves the foreground color used as the highlight background for a span style.
///
/// # Arguments
///
/// * `style` - Span style from the rendered log line.
///
/// # Returns
///
/// The span foreground, or the theme text color when unset or reset.
fn effective_foreground(style: Style) -> Color {
    match style.fg {
        None | Some(Color::Reset) => theme::text(),
        Some(Color::Indexed(index)) => indexed_color(index),
        Some(color) => color,
    }
}

/// Picks black or white text for readability on `background`.
///
/// Uses relative luminance (sRGB) and compares against a fixed threshold.
///
/// # Arguments
///
/// * `background` - Background color the highlight text will sit on.
///
/// # Returns
///
/// [`Color::Rgb`] black or white for maximum contrast on `background`.
fn contrasting_foreground(background: Color) -> Color {
    let (r, g, b) = color_to_rgb(background);
    let linear = |channel: u8| {
        let value = f64::from(channel) / 255.0;
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
    if luminance > 0.179 {
        Color::Rgb(0, 0, 0)
    } else {
        Color::Rgb(255, 255, 255)
    }
}

/// Converts a ratatui [`Color`] to 8-bit RGB channels.
///
/// # Arguments
///
/// * `color` - Input color (indexed colors are expanded via [`indexed_color`]).
///
/// # Returns
///
/// `(r, g, b)` components in `0..=255`.
fn color_to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Indexed(index) => color_to_rgb(indexed_color(index)),
        Color::Black => (0, 0, 0),
        Color::Gray => (128, 128, 128),
        Color::DarkGray => (128, 128, 128),
        Color::Red => (128, 0, 0),
        Color::LightRed => (255, 0, 0),
        Color::Green => (0, 128, 0),
        Color::LightGreen => (0, 255, 0),
        Color::Yellow => (128, 128, 0),
        Color::LightYellow => (255, 255, 0),
        Color::Blue => (0, 0, 128),
        Color::LightBlue => (0, 0, 255),
        Color::Magenta => (128, 0, 128),
        Color::LightMagenta => (255, 0, 255),
        Color::Cyan => (0, 128, 128),
        Color::LightCyan => (0, 255, 255),
        Color::White => (255, 255, 255),
        Color::Reset => color_to_rgb(theme::text()),
    }
}

/// Maps an xterm 256-color index to a ratatui [`Color`].
///
/// # Arguments
///
/// * `index` - ANSI 256-color palette index (`0..=255`).
///
/// # Returns
///
/// The corresponding RGB or palette color.
fn indexed_color(index: u8) -> Color {
    match index {
        0..=15 => basic_sgr_foreground(match index {
            0 => 30,
            1 => 31,
            2 => 32,
            3 => 33,
            4 => 34,
            5 => 35,
            6 => 36,
            7 => 37,
            8 => 90,
            9 => 91,
            10 => 92,
            11 => 93,
            12 => 94,
            13 => 95,
            14 => 96,
            _ => 97,
        }),
        232..=255 => {
            let gray = 8 + (index - 232) * 10;
            Color::Rgb(gray, gray, gray)
        }
        _ => {
            let index = index - 16;
            let r = index / 36;
            let g = (index / 6) % 6;
            let b = index % 6;
            let channel = |value: u8| {
                if value == 0 { 0 } else { 55 + value * 40 }
            };
            Color::Rgb(channel(r), channel(g), channel(b))
        }
    }
}

/// Maps a classic 16-color SGR foreground code to a ratatui [`Color`].
///
/// # Arguments
///
/// * `code` - ANSI foreground SGR number (e.g. `30`..`37`, `90`..`97`).
///
/// # Returns
///
/// The corresponding palette color, or the theme text color for unrecognized codes.
fn basic_sgr_foreground(code: u8) -> Color {
    match code {
        30 => Color::Black,
        31 => Color::Red,
        32 => Color::Green,
        33 => Color::Yellow,
        34 => Color::Blue,
        35 => Color::Magenta,
        36 => Color::Cyan,
        37 => Color::White,
        90 => Color::DarkGray,
        91 => Color::LightRed,
        92 => Color::LightGreen,
        93 => Color::LightYellow,
        94 => Color::LightBlue,
        95 => Color::LightMagenta,
        96 => Color::LightCyan,
        97 => Color::White,
        _ => theme::text(),
    }
}
