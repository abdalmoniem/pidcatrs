// Copyright (c) AbdAlMoniem AlHifnawy <hifnawy_moniem@hotmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

//! Highlight rules derived from the user-visible filter query AST.
//!
//! Only non-negated terms except `level:` and `age:` produce [`FilterHighlightRule`] values for
//! the TUI display cache. Implicit ignore-tag clauses are excluded.

#![deny(clippy::unwrap_used)]

use crate::FilterHighlightField;
use crate::FilterHighlightRule;
use crate::LogEntry;
use crate::controller::log_processor::tag_filter_match_text;

use super::parse::FilterExpr;
use super::parse::FilterKey;
use super::parse::FilterQuery;
use super::parse::FilterTerm;

/// Returns true when `query` contains terms that can drive log-line token highlighting.
///
/// # Arguments
///
/// * `query` — User-visible parsed query (not implicit ignores).
pub fn user_query_has_token_highlights(query: &FilterQuery) -> bool {
    query.root.as_ref().is_some_and(expr_has_token_highlights)
}

/// Returns true when any leaf under `expr` can produce a highlight rule.
fn expr_has_token_highlights(expr: &FilterExpr) -> bool {
    match expr {
        FilterExpr::Term(term) => {
            !term.negated && !matches!(term.key, FilterKey::Level | FilterKey::Age)
        }
        FilterExpr::And(parts) | FilterExpr::Or(parts) => {
            parts.iter().any(expr_has_token_highlights)
        }
        FilterExpr::Group(inner) => expr_has_token_highlights(inner),
        FilterExpr::Not(_) => false,
    }
}

/// Collects highlight rules from `query` (user query only — no implicit ignores).
///
/// # Arguments
///
/// * `query` — Parsed user filter AST.
/// * `entry` — Log line used to resolve tag substring needles.
///
/// # Returns
///
/// De-duplicated rules in discovery order.
pub fn highlight_rules_for_user_query(
    query: &FilterQuery,
    entry: &LogEntry,
) -> Vec<FilterHighlightRule> {
    let mut rules = Vec::default();
    if let Some(root) = &query.root {
        collect_highlight_rules(root, entry, &mut rules);
    }
    dedupe_highlight_rules(rules)
}

/// Walks `expr` and appends highlight rules for matching leaf terms.
fn collect_highlight_rules(
    expr: &FilterExpr,
    entry: &LogEntry,
    rules: &mut Vec<FilterHighlightRule>,
) {
    match expr {
        FilterExpr::Term(term) => {
            if term.negated || term.key == FilterKey::Level || term.key == FilterKey::Age {
                return;
            }
            if let Some(rule) = term_highlight_rule(term, entry) {
                rules.push(rule);
            }
        }
        FilterExpr::And(parts) | FilterExpr::Or(parts) => {
            for part in parts {
                collect_highlight_rules(part, entry, rules);
            }
        }
        FilterExpr::Group(inner) => collect_highlight_rules(inner, entry, rules),
        FilterExpr::Not(_) => {}
    }
}

/// Builds one highlight rule for `term`, or `None` when the needle would be empty.
fn term_highlight_rule(term: &FilterTerm, entry: &LogEntry) -> Option<FilterHighlightRule> {
    let field = match term.key {
        FilterKey::Pid => FilterHighlightField::Pid,
        FilterKey::Uid => FilterHighlightField::Uid,
        FilterKey::Package => FilterHighlightField::Package,
        FilterKey::Tag => FilterHighlightField::Tag,
        FilterKey::Message => FilterHighlightField::Message,
        FilterKey::Level | FilterKey::Age => return None,
    };

    if term.regex {
        if term.value.is_empty() {
            return None;
        }
        return Some(FilterHighlightRule {
            needle: term.value.clone(),
            case_insensitive: false,
            field,
            regex: true,
        });
    }

    let case_insensitive = matches!(
        term.key,
        FilterKey::Package | FilterKey::Tag | FilterKey::Message
    );

    let needle = match term.key {
        FilterKey::Tag => tag_filter_match_text(&entry.tag, &term.value)?,
        _ => term.value.clone(),
    };

    if needle.is_empty() {
        return None;
    }

    Some(FilterHighlightRule {
        needle,
        case_insensitive,
        field,
        regex: false,
    })
}

/// Removes duplicate rules (same needle, field, case flag, and regex flag).
fn dedupe_highlight_rules(rules: Vec<FilterHighlightRule>) -> Vec<FilterHighlightRule> {
    let mut unique: Vec<FilterHighlightRule> = Vec::default();
    'outer: for rule in rules {
        for existing in &unique {
            if existing.needle == rule.needle
                && existing.case_insensitive == rule.case_insensitive
                && existing.field == rule.field
                && existing.regex == rule.regex
            {
                continue 'outer;
            }
        }
        unique.push(rule);
    }
    unique
}
