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

//! Legacy CLI flags to query strings and `--query` conflict checks.
//!
//! When [`CliArgs::query`] is set, package/tag/regex/level/current filters must not also be
//! passed on the command line; tag exclusions via `--ignore-tag` remain valid.

#![deny(clippy::unwrap_used)]

use std::collections::HashSet;

use crate::CliArgs;
use crate::LogLevel;
use crate::controller::util::exit_with_error;

/// Returns true when any legacy filter flag conflicts with `--query`.
///
/// # Arguments
///
/// * `args` — Parsed CLI arguments.
pub fn legacy_filter_flags_active(args: &CliArgs) -> bool {
    args.tag.is_some()
        || args.regex.is_some()
        || args.log_level != LogLevel::VERBOSE
        || !args.packages.is_empty()
        || args.current_app
}

/// Exits the process when `--query` is combined with conflicting legacy filter flags.
///
/// # Arguments
///
/// * `args` — Parsed CLI arguments; no-op when `query` is unset.
///
/// Calls [`exit_with_error`] and terminates the process when a conflict is detected.
pub fn validate_query_flag_conflicts(args: &CliArgs) {
    if args.query.is_none() {
        return;
    }
    if legacy_filter_flags_active(args) {
        exit_with_error(
            "--query cannot be combined with other filter flags \
             (tag, regex, log-level, package selection, --current); \
             use --ignore-tag / --ignore-system-tags for tag exclusions",
            true,
        );
    }
}

/// Builds a Logcat-style query string equivalent to legacy CLI filter flags.
///
/// Package catch-alls become `package:` terms; tags, regex, and level follow the same shape as
/// [`crate::controller::setup::seed_filter_input`], except regex uses `message~` form.
///
/// # Arguments
///
/// * `args` — Legacy tag, regex, and level flags.
/// * `packages` — Resolved package names (main processes only; `:` suffixes are skipped).
///
/// # Returns
///
/// A single query string suitable for [`crate::parse_query`].
pub fn compile_legacy_filter_query(args: &CliArgs, packages: &HashSet<String>) -> String {
    let mut parts = Vec::default();

    let mut packages: Vec<&String> = packages
        .iter()
        .filter(|package| !package.contains(':'))
        .collect();
    packages.sort();
    for package in packages {
        parts.push(format!("package:{package}"));
    }

    if let Some(tags) = &args.tag {
        for tag in tags {
            parts.push(format!("tag:{tag}"));
        }
    }

    if let Some(regex) = &args.regex {
        parts.push(format!("message~{regex}"));
    }

    if args.log_level != LogLevel::VERBOSE {
        parts.push(format!("level:{}", args.log_level.filter_name()));
    }

    parts.join(" ")
}
