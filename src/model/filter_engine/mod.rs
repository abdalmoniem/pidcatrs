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

//! Logcat-style filter query parsing and evaluation for plain mode and the TUI.
//!
//! User-visible query text (`--query`, the TUI filter bar) is parsed into an AST ([`FilterQuery`]),
//! optionally combined with implicit constraints from the CLI (such as [`CliArgs::ignore_tag`]),
//! and evaluated against [`LogEntry`] values through [`EffectiveFilter`].
//!
//! # Query syntax (summary)
//!
//! - **Key predicates** — `tag:Main`, `message~Error.*`, bare words as implicit `message:` terms.
//! - **Boolean logic** — whitespace is AND; `|` is OR; parentheses group; `-` negates a term or group.
//! - **Special keys** — `level:warn`, `age:5m` (duration suffix `s`/`m`/`h`/`d`).
//!
//! Legacy CLI filter flags compile to the same syntax via [`compile_legacy_filter_query`].
//! Highlight rules for the TUI log table are derived from the user-visible AST only
//! ([`highlight_rules_for_user_query`]).

#![deny(clippy::unwrap_used)]

mod compile;
mod effective;
mod eval;
mod highlight;
mod parse;

pub use compile::compile_legacy_filter_query;
pub use compile::validate_query_flag_conflicts;
pub use effective::EffectiveFilter;
pub use effective::build_implicit_filter_query;
pub use eval::FilterContext;
pub use highlight::highlight_rules_for_user_query;
pub use highlight::user_query_has_token_highlights;
pub use parse::FilterExpr;
pub use parse::FilterKey;
pub use parse::FilterQuery;
pub use parse::ParseError;
pub use parse::SourceSpan;
pub use parse::parse_query;
use std::collections::HashSet;

use crate::CliArgs;

/// User-visible filter text at startup (`--query` or legacy flags compiled to a query).
///
/// # Arguments
///
/// * `args` — CLI configuration; when [`CliArgs::query`] is set it is returned verbatim.
/// * `packages` — Resolved package set used when compiling legacy package filters.
///
/// # Returns
///
/// The query string shown in the TUI filter bar and parsed by [`EffectiveFilter::from_user_source`].
pub fn startup_user_filter_source(args: &CliArgs, packages: &HashSet<String>) -> String {
    if let Some(query) = &args.query {
        return query.clone();
    }
    compile_legacy_filter_query(args, packages)
}

/// Builds the session [`EffectiveFilter`] from CLI args and resolved packages.
///
/// # Arguments
///
/// * `args` — CLI configuration (implicit `ignore_tag` patterns, layout for evaluation).
/// * `packages` — Resolved packages for legacy query compilation when `--query` is unset.
///
/// # Errors
///
/// Returns [`ParseError`] when the user query string is invalid.
pub fn build_startup_effective_filter(
    args: &CliArgs,
    packages: &HashSet<String>,
) -> Result<EffectiveFilter, ParseError> {
    let source = startup_user_filter_source(args, packages);
    EffectiveFilter::from_user_source(&source, args)
}
