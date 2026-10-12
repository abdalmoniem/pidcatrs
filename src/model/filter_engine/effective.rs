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

//! Combines user-visible queries with implicit CLI constraints.
//!
//! [`EffectiveFilter`] is the session-wide filter object used by plain mode, TUI recompute, and
//! log ingest. Implicit `-i` / `ignore_tag` patterns are parsed into a separate AST and ANDed
//! with the user query; they are not shown in the filter bar.

#![deny(clippy::unwrap_used)]

use chrono::DateTime;
use chrono::Local;

use crate::CliArgs;
use crate::LogEntry;
use crate::State;

use super::eval::FilterContext;
use super::parse::FilterExpr;
use super::parse::FilterKey;
use super::parse::FilterQuery;
use super::parse::FilterTerm;
use super::parse::ParseError;
use super::parse::parse_query;
use super::parse::query_accepts_all_entries;

/// User query plus implicit tag exclusions, evaluated as a single AND expression.
#[derive(Clone, Debug)]
pub struct EffectiveFilter {
    /// Visible query text (TUI filter bar / `--query`); may be empty.
    pub user_source: String,
    /// Parsed user query (no implicit clauses).
    pub user_query: FilterQuery,
    /// Parsed implicit constraints from `ignore_tag` / config.
    pub implicit_query: FilterQuery,
    /// Combined AST used for matching (`user AND implicit` when both are present).
    combined: FilterQuery,
}

impl EffectiveFilter {
    /// Builds an effective filter from visible user text and CLI args.
    ///
    /// # Arguments
    ///
    /// * `user_source` — filter bar or `--query` string (implicit ignores are not included).
    /// * `args` — supplies `ignore_tag` patterns merged into the implicit query.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] when `user_source` is not valid query syntax.
    pub fn from_user_source(user_source: &str, args: &CliArgs) -> Result<Self, ParseError> {
        let user_query = parse_query(user_source)?;
        let implicit_query = build_implicit_filter_query(args);
        let combined = combine_queries(&user_query, &implicit_query);
        Ok(Self {
            user_source: user_source.to_string(),
            user_query,
            implicit_query,
            combined,
        })
    }

    /// Returns true when the entry passes both user and implicit constraints.
    ///
    /// # Arguments
    ///
    /// * `entry` — Parsed log line to test.
    /// * `state` — Session maps (UID names, package resolution).
    /// * `now` — Wall clock for `age:` predicates.
    /// * `args` — CLI options passed through to [`FilterContext`].
    pub fn matches(
        &self,
        entry: &LogEntry,
        state: &State,
        now: DateTime<Local>,
        args: &CliArgs,
    ) -> bool {
        let ctx = FilterContext {
            entry,
            state,
            now,
            args,
        };
        ctx.matches_query(&self.combined)
    }

    /// Returns true when the user-visible query alone would accept every entry.
    pub fn user_accepts_all(&self) -> bool {
        query_accepts_all_entries(&self.user_query)
    }

    /// Borrowed user query for highlight generation (implicit clauses excluded).
    pub fn user_query(&self) -> &FilterQuery {
        &self.user_query
    }
}

/// Builds a query that excludes tags matching `args.ignore_tag` patterns.
///
/// Each configured ignore pattern becomes a negated `tag:` term ANDed together.
///
/// # Arguments
///
/// * `args` — CLI configuration; when `ignore_tag` is unset the result has `root: None`.
///
/// # Returns
///
/// A [`FilterQuery`] with an empty `source` string (implicit only, not shown in the TUI).
pub fn build_implicit_filter_query(args: &CliArgs) -> FilterQuery {
    let Some(tags) = &args.ignore_tag else {
        return FilterQuery {
            source: String::default(),
            root: None,
        };
    };

    let terms: Vec<FilterExpr> = tags
        .iter()
        .filter(|tag| !tag.is_empty())
        .map(|tag| {
            FilterExpr::Term(FilterTerm {
                key: FilterKey::Tag,
                value: tag.clone(),
                negated: true,
                regex: false,
                key_span: None,
                value_span: None,
                op_span: None,
                neg_span: None,
            })
        })
        .collect();

    if terms.is_empty() {
        return FilterQuery {
            source: String::default(),
            root: None,
        };
    }

    FilterQuery {
        source: String::default(),
        root: Some(FilterExpr::And(terms)),
    }
}

/// AND-combines user and implicit roots into one [`FilterQuery`] for evaluation.
fn combine_queries(user: &FilterQuery, implicit: &FilterQuery) -> FilterQuery {
    let root = match (&user.root, &implicit.root) {
        (None, None) => None,
        (Some(user_root), None) => Some(user_root.clone()),
        (None, Some(implicit_root)) => Some(implicit_root.clone()),
        (Some(user_root), Some(implicit_root)) => Some(FilterExpr::And(vec![
            user_root.clone(),
            implicit_root.clone(),
        ])),
    };

    FilterQuery {
        source: user.source.clone(),
        root,
    }
}
