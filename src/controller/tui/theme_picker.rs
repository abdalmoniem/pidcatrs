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

//! The "change theme" dialog: searchable lists with light/dark tabs and live preview.
//!
//! Layout (top to bottom inside the dialog): filter field, centered light/dark tab row,
//! scrollable theme list. Tab changes are handled in [`super::app::TuiApp`]; this module
//! only renders state passed in from the app.

#![deny(clippy::unwrap_used)]

use ratatui::Frame;
use ratatui::layout::Alignment;
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

use crate::ThemeCategory;

use super::border::render_dialog;
use super::palette::PaletteSearch;
use super::palette::matches_query;
use super::palette::render_search_field;
use super::theme;

/// Placeholder shown in the empty theme search field.
const THEME_SEARCH_PLACEHOLDER: &str = "filter themes by name...";

/// Shortcut hints shown in the bottom border of the theme dialog.
const THEME_PICKER_HINTS: &[(&str, &str)] = &[
    ("[ / ]", " light / dark"),
    ("↑↓", " preview"),
    ("enter", " apply"),
    ("esc", " revert"),
];

/// Filters theme names by a search query using [`matches_query`].
///
/// # Arguments
///
/// * `theme_names` - Full list for the active tab (light or dark).
/// * `query` - Text from the dialog search field.
///
/// # Returns
///
/// Indices into `theme_names` of matching names, in their original order.
pub fn filter_theme_indices(theme_names: &[String], query: &str) -> Vec<usize> {
    theme_names
        .iter()
        .enumerate()
        .filter(|(_, name)| matches_query(query, name))
        .map(|(index, _)| index)
        .collect()
}

/// Draws the centered light/dark tab row.
///
/// # Arguments
///
/// * `frame` - Frame to draw on.
/// * `area` - Single-row region below the search field.
/// * `active_tab` - Tab index (`0` = light, `1` = dark); see [`ThemeCategory::tab_index`].
fn render_theme_tabs(frame: &mut Frame, area: Rect, active_tab: usize) {
    let light_style = if active_tab == ThemeCategory::Light.tab_index() {
        Style::default()
            .fg(theme::background())
            .bg(theme::accent())
            .add_modifier(Modifier::BOLD)
    } else {
        theme::dim_style()
    };
    let dark_style = if active_tab == ThemeCategory::Dark.tab_index() {
        Style::default()
            .fg(theme::background())
            .bg(theme::accent())
            .add_modifier(Modifier::BOLD)
    } else {
        theme::dim_style()
    };

    let line = Line::from(vec![
        Span::styled("light", light_style),
        Span::styled("  ", theme::app_background_style()),
        Span::styled("dark", dark_style),
    ]);
    frame.render_widget(
        Paragraph::new(line)
            .style(theme::app_background_style())
            .alignment(Alignment::Center),
        area,
    );
}

/// Renders the theme picker dialog.
///
/// Shows a search field, centered light/dark tabs, and a filtered list for `theme_names`
/// (the caller supplies the list for the active tab only). Updates `search` selection and
/// scroll so the highlighted row stays visible. Marks the configured theme with `(active)`.
///
/// # Arguments
///
/// * `frame` - Frame to draw on.
/// * `theme_names` - Theme names for the currently selected tab.
/// * `active_theme_name` - Persisted theme from config / CLI ([`crate::CliArgs::theme`]).
/// * `active_tab` - Light/dark tab index (`0` or `1`).
/// * `search` - Search and list selection state (mutated to clamp and scroll).
/// * `area` - Outer rectangle of the dialog.
pub fn render_theme_picker(
    frame: &mut Frame,
    theme_names: &[String],
    active_theme_name: &str,
    active_tab: usize,
    search: &mut PaletteSearch,
    area: Rect,
) {
    let inner = render_dialog(frame, area, "change theme", THEME_PICKER_HINTS, false);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(inner);

    render_search_field(frame, chunks[0usize], search, THEME_SEARCH_PLACEHOLDER);
    render_theme_tabs(frame, chunks[1usize], active_tab);

    let filtered = filter_theme_indices(theme_names, &search.query);
    let list_height = chunks[2usize].height as usize;

    if theme_names.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "no themes found",
                theme::dim_style(),
            )))
            .style(theme::app_background_style()),
            chunks[2usize],
        );
    } else if filtered.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "no matching themes",
                theme::dim_style(),
            )))
            .style(theme::app_background_style()),
            chunks[2usize],
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
        frame.render_widget(list, chunks[2usize]);
    }
}
