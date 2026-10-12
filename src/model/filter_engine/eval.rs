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

//! Evaluate parsed filter AST nodes against log entries.

#![deny(clippy::unwrap_used)]

use std::sync::LazyLock;

use chrono::DateTime;
use chrono::Local;
use regex::Regex;

use crate::CliArgs;
use crate::LogEntry;
use crate::LogEntryKind;
use crate::LogLevel;
use crate::State;
use crate::package_name_contains;
use crate::passes_log_level;
use crate::passes_tag_filter;

use super::parse::FilterExpr;
use super::parse::FilterKey;
use super::parse::FilterQuery;
use super::parse::FilterTerm;

/// Runtime data required to evaluate a [`FilterQuery`] against one log line.
pub struct FilterContext<'a> {
    /// Parsed log line under test.
    pub entry: &'a LogEntry,
    /// Session maps and default log level.
    pub state: &'a State,
    /// Wall clock used for `age:` filters.
    pub now: DateTime<Local>,
    /// CLI configuration (package maps, column behavior).
    pub args: &'a CliArgs,
}

impl FilterContext<'_> {
    /// Returns true when `query` matches this context's entry.
    pub fn matches_query(&self, query: &FilterQuery) -> bool {
        match &query.root {
            None => true,
            Some(root) => self.eval_expr(root),
        }
    }
}

impl<'a> FilterContext<'a> {
    fn eval_expr(&self, expr: &FilterExpr) -> bool {
        match expr {
            FilterExpr::Term(term) => {
                let matched = self.eval_term(term);
                if term.negated { !matched } else { matched }
            }
            FilterExpr::And(parts) => {
                if parts.is_empty() {
                    return true;
                }
                parts.iter().all(|part| self.eval_expr(part))
            }
            FilterExpr::Or(parts) => parts.iter().any(|part| self.eval_expr(part)),
            FilterExpr::Group(inner) => self.eval_expr(inner),
            FilterExpr::Not(inner) => !self.eval_expr(inner),
        }
    }

    fn eval_term(&self, term: &FilterTerm) -> bool {
        if matches!(
            self.entry.kind,
            LogEntryKind::ProcessStart | LogEntryKind::ProcessDeath
        ) {
            return self.eval_lifecycle_term(term);
        }

        match term.key {
            FilterKey::Pid => self.match_equality(&self.entry.pid, term),
            FilterKey::Uid => self.match_uid(term),
            FilterKey::Package => self.match_package(term),
            FilterKey::Tag => self.match_tag(term),
            FilterKey::Level => self.match_level(term),
            FilterKey::Message => self.match_text(&self.entry.message, term),
            FilterKey::Age => self.match_age(term),
        }
    }

    fn eval_lifecycle_term(&self, term: &FilterTerm) -> bool {
        match term.key {
            FilterKey::Tag | FilterKey::Level => true,
            FilterKey::Pid => self.match_equality(&self.entry.pid, term),
            FilterKey::Uid => self.match_uid(term),
            FilterKey::Package => self.match_package(term),
            FilterKey::Message => self.match_text(&self.entry.banner_text, term),
            FilterKey::Age => self.match_age(term),
        }
    }

    fn match_equality(&self, field: &str, term: &FilterTerm) -> bool {
        if term.regex {
            regex_matches(field, &term.value)
        } else {
            field == term.value.trim()
        }
    }

    fn match_uid(&self, term: &FilterTerm) -> bool {
        if term.regex {
            regex_matches(&self.entry.uid, &term.value)
                || self
                    .state
                    .uids_map
                    .keys()
                    .any(|uid| regex_matches(uid, &term.value))
        } else {
            self.entry.uid == term.value || self.state.uids_map.contains_key(&term.value)
        }
    }

    fn match_package(&self, term: &FilterTerm) -> bool {
        if term.regex {
            let display = crate::resolve_entry_package(self.entry, self.state);
            regex_matches(&display, &term.value)
        } else {
            package_name_contains(self.entry, self.state, &term.value)
        }
    }

    fn match_tag(&self, term: &FilterTerm) -> bool {
        if term.regex {
            regex_matches(&self.entry.tag, &term.value)
        } else {
            passes_tag_filter(&self.entry.tag, std::slice::from_ref(&term.value))
        }
    }

    fn match_level(&self, term: &FilterTerm) -> bool {
        let min_level = term
            .value
            .split(',')
            .map(str::trim)
            .filter_map(LogLevel::parse_name)
            .min();
        min_level
            .map(|min| passes_log_level(self.entry.level, min))
            .unwrap_or(false)
    }

    fn match_text(&self, haystack: &str, term: &FilterTerm) -> bool {
        if term.regex {
            regex_matches(haystack, &term.value)
        } else {
            haystack
                .to_ascii_lowercase()
                .contains(&term.value.to_ascii_lowercase())
        }
    }

    fn match_age(&self, term: &FilterTerm) -> bool {
        let Some(max_age) = parse_age_duration(&term.value) else {
            return false;
        };
        let cutoff = self.now - max_age;
        self.entry.timestamp >= cutoff
    }
}

fn regex_matches(haystack: &str, pattern: &str) -> bool {
    match compile_regex(pattern) {
        Ok(re) => re.is_match(haystack),
        Err(_) => false,
    }
}

fn compile_regex(pattern: &str) -> Result<Regex, regex::Error> {
    static CACHE: LazyLock<std::sync::Mutex<std::collections::HashMap<String, Regex>>> =
        LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

    if let Ok(guard) = CACHE.lock()
        && let Some(existing) = guard.get(pattern)
    {
        return Ok(existing.clone());
    }

    let built = Regex::new(pattern)?;
    if let Ok(mut guard) = CACHE.lock() {
        guard.insert(pattern.to_string(), built.clone());
    }
    Ok(built)
}

/// Parses Logcat-style age values such as `30s`, `5m`, `1h`, `2d`.
fn parse_age_duration(value: &str) -> Option<chrono::Duration> {
    let trimmed = value.trim();
    if trimmed.len() < 2 {
        return None;
    }
    let unit = trimmed.chars().last()?;
    let num_part = &trimmed[..trimmed.len().saturating_sub(1)];
    let amount: i64 = num_part.parse().ok()?;
    let secs = match unit {
        's' | 'S' => amount,
        'm' | 'M' => amount * 60,
        'h' | 'H' => amount * 3600,
        'd' | 'D' => amount * 86400,
        _ => return None,
    };
    chrono::Duration::try_seconds(secs)
}
