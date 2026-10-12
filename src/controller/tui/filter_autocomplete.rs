// Copyright (C) 2026 AbdAlMoniem AlHifnawy
//
// This file is part of pidcatrs.
//
// pidcatrs is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Inline filter key autocomplete menu at the terminal cursor.

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
    key: crate::model::filter_engine::FilterKey,
    regex: bool,
}

impl FilterKeySuggestion {
    fn insertion_text(self) -> String {
        let name = self.key.as_str();
        if self.regex {
            format!("{name}~")
        } else {
            format!("{name}:")
        }
    }

    fn display_width(self) -> usize {
        self.key.as_str().chars().count() + 1
    }
}

/// State of the inline key autocomplete popup.
#[derive(Clone, Debug, Default)]
pub struct FilterAutocomplete {
    options: Vec<FilterKeySuggestion>,
    selected: usize,
    prefix: String,
}

impl FilterAutocomplete {
    /// Opens or refreshes the menu at `cursor` (typing or Ctrl+Space).
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

    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub fn is_open(&self) -> bool {
        !self.options.is_empty()
    }

    pub fn move_selection(&mut self, delta: isize) {
        if self.options.is_empty() {
            return;
        }
        let len = self.options.len();
        let next = (self.selected as isize + delta).rem_euclid(len as isize);
        self.selected = next as usize;
    }

    pub fn accept_key(&self) -> Option<String> {
        Some(self.options.get(self.selected)?.insertion_text())
    }

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
