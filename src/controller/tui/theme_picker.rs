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

//! The "change theme" dialog: lists installed and bundled themes with search.

#![deny(clippy::unwrap_used)]

use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::Paragraph;

use super::border::render_dialog;
use super::palette::PaletteSearch;
use super::palette::matches_query;
use super::palette::render_search_field;
use super::theme;

/// Placeholder shown in the empty theme search field.
const THEME_SEARCH_PLACEHOLDER: &str = "filter themes by name...";

/// Shortcut hints shown in the bottom border of the theme dialog.
const THEME_PICKER_HINTS: &[(&str, &str)] =
    &[("↑↓", " navigate"), ("enter", " apply"), ("esc", " cancel")];

/// Filters theme names by a search query using [`matches_query`].
pub fn filter_theme_indices(theme_names: &[String], query: &str) -> Vec<usize> {
    theme_names
        .iter()
        .enumerate()
        .filter(|(_, name)| matches_query(query, name))
        .map(|(index, _)| index)
        .collect()
}

/// Renders the theme picker dialog.
pub fn render_theme_picker(
    frame: &mut Frame,
    theme_names: &[String],
    active_theme_name: &str,
    search: &mut PaletteSearch,
    area: Rect,
) {
    let inner = render_dialog(frame, area, "change theme", THEME_PICKER_HINTS, false);

    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(inner);

    render_search_field(frame, chunks[0usize], search, THEME_SEARCH_PLACEHOLDER);

    let filtered = filter_theme_indices(theme_names, &search.query);
    let list_height = chunks[1usize].height as usize;

    if theme_names.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "no themes found",
                theme::dim_style(),
            )))
            .style(theme::app_background_style()),
            chunks[1usize],
        );
    } else if filtered.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "no matching themes",
                theme::dim_style(),
            )))
            .style(theme::app_background_style()),
            chunks[1usize],
        );
    } else {
        search.clamp_selection(filtered.len());
        search.ensure_list_top_visible(list_height, filtered.len());

        let items = filtered
            .iter()
            .enumerate()
            .skip(search.list_top)
            .take(list_height)
            .map(|(list_index, &theme_index)| {
                let name = &theme_names[theme_index];
                let prefix = if list_index == search.selected {
                    "▸ "
                } else {
                    "  "
                };
                let active = name == active_theme_name;
                let line = if active {
                    format!("{prefix}{name}  (active)")
                } else {
                    format!("{prefix}{name}")
                };
                let style = if list_index == search.selected {
                    Style::default()
                        .fg(theme::background())
                        .bg(theme::accent())
                        .add_modifier(Modifier::BOLD)
                } else if active {
                    Style::default().fg(theme::accent())
                } else {
                    Style::default().fg(theme::text())
                };
                ListItem::new(Line::from(Span::styled(line, style)))
            })
            .collect::<Vec<_>>();

        let list = List::new(items).style(theme::app_background_style());
        frame.render_widget(list, chunks[1usize]);
    }
}
