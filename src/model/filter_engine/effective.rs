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
    /// Combined AST used for matching (`user AND implicit`).
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
    /// # Returns
    ///
    /// Parsed filter ready for [`Self::matches`], or a parse error from the user query.
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
