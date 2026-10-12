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

//! Inline filter key autocomplete menu at the terminal cursor.
//!
//! While the filter bar is focused, typing a key name (or pressing Ctrl+Space) opens a floating
//! popup listing `key:` and `key~` completions. The popup is drawn over the log table without
//! changing filter bar height ([`FilterAutocomplete::render_popup`]).

#![deny(clippy::unwrap_used)]

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;

use super::border::render_accent_popup_frame;
use super::filter_input::matching_filter_suggestions;
use super::theme;

/// One autocomplete row: `key:` or `key~`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FilterKeySuggestion {
    /// Filter field for this row.
    key: crate::model::filter_engine::FilterKey,
    /// When true, the row inserts `~` (regex operator) instead of `:`.
    regex: bool,
}

impl FilterKeySuggestion {
    /// Text inserted into the filter bar when this row is accepted.
    fn insertion_text(self) -> String {
        let name = self.key.as_str();
        if self.regex {
            format!("{name}~")
        } else {
            format!("{name}:")
        }
    }

    /// Display width in terminal columns (`key` plus one operator character).
    fn display_width(self) -> usize {
        self.key.as_str().chars().count() + 1
    }
}

/// State of the inline key autocomplete popup.
#[derive(Clone, Debug, Default)]
pub struct FilterAutocomplete {
    /// Visible completion rows for the current key prefix.
    options: Vec<FilterKeySuggestion>,
    /// Highlighted row index in `options`.
    selected: usize,
    /// Lowercase key prefix being completed (empty means all keys).
    prefix: String,
}

impl FilterAutocomplete {
    /// Opens or refreshes the menu at `cursor` (typing or Ctrl+Space).
    ///
    /// # Arguments
    ///
    /// * `input` — Full filter bar text.
    /// * `cursor` — Cursor position in characters.
    pub fn refresh_at_cursor(&mut self, input: &str, cursor: usize) {
        let previous = self.options.get(self.selected).copied();
        let Some(prefix) = super::filter_input::completion_prefix_at_cursor(input, cursor) else {
            self.options.clear();
            self.prefix.clear();
            self.selected = 0;
            return;
        };
        self.prefix = prefix.clone();
        self.options = matching_filter_suggestions(&prefix)
            .into_iter()
            .map(|(key, regex)| FilterKeySuggestion { key, regex })
            .collect();
        self.selected = previous
            .and_then(|prev| self.options.iter().position(|&opt| opt == prev))
            .unwrap_or_default();
    }

    /// Returns the key prefix used to build the current `options` list.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Returns true when at least one completion row is available.
    pub fn is_open(&self) -> bool {
        !self.options.is_empty()
    }

    /// Moves the highlighted row by `delta`, wrapping at the list ends.
    ///
    /// # Arguments
    ///
    /// * `delta` — Row delta (`-1` / `+1` for arrow keys).
    pub fn move_selection(&mut self, delta: isize) {
        if self.options.is_empty() {
            return;
        }
        let len = self.options.len();
        let next = (self.selected as isize + delta).rem_euclid(len as isize);
        self.selected = next as usize;
    }

    /// Returns the insertion text for the highlighted row, or `None` when the menu is empty.
    pub fn accept_key(&self) -> Option<String> {
        Some(self.options.get(self.selected)?.insertion_text())
    }

    /// Draws the popup near `(cursor_x, cursor_y)` inside the filter panel.
    ///
    /// # Arguments
    ///
    /// * `frame` — Target frame.
    /// * `inner` — Inner rectangle of the filter panel (used to clamp horizontal position).
    /// * `cursor_x` — Terminal column of the text cursor.
    /// * `cursor_y` — Terminal row of the text cursor.
    pub fn render_popup(
        &self,
        frame: &mut ratatui::Frame,
        inner: Rect,
        cursor_x: u16,
        cursor_y: u16,
    ) {
        if !self.is_open() {
            return;
        }

        let row_count = self.options.len() as u16;
        let frame_area = frame.area();
        let content_width = self
            .options
            .iter()
            .map(|opt| opt.display_width())
            .max()
            .unwrap_or(4) as u16;

        let outer_width = content_width.saturating_add(2);
        let outer_height = row_count.saturating_add(2);

        let mut x = cursor_x;
        if x.saturating_add(outer_width) > frame_area.right() {
            x = frame_area.right().saturating_sub(outer_width);
        }
        x = x.max(inner.x);

        let mut y = cursor_y.saturating_add(1);
        if y.saturating_add(outer_height) > frame_area.bottom() {
            y = cursor_y.saturating_sub(outer_height);
        }

        let outer = Rect {
            x,
            y,
            width: outer_width.min(frame_area.right().saturating_sub(x)),
            height: outer_height.min(frame_area.bottom().saturating_sub(y)),
        };
        if outer.width < 4 || outer.height < 3 {
            return;
        }

        frame.render_widget(Clear, outer);
        let content_area = render_accent_popup_frame(frame, outer);

        let visible = content_area.height as usize;
        let scroll = self.selected.saturating_sub(visible.saturating_sub(1));

        let accent = Style::default().fg(theme::accent());
        let items: Vec<Line> = self
            .options
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible)
            .map(|(index, opt)| {
                let absolute = scroll + index;
                let selected = absolute == self.selected;
                let key_style = if selected {
                    theme::help_selected_style()
                } else {
                    accent
                };
                let op = if opt.regex { "~" } else { ":" };
                Line::from(vec![
                    Span::styled(opt.key.as_str(), key_style),
                    Span::styled(op, key_style),
                ])
            })
            .collect();

        frame.render_widget(
            Paragraph::new(items).style(theme::app_background_style()),
            content_area,
        );
    }
}
