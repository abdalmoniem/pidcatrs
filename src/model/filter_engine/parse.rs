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

//! Lexer and parser for Logcat-style filter query strings.
//!
//! The parser produces a [`FilterQuery`] AST with source spans for TUI syntax coloring.
//! Whitespace-separated terms are ANDed; `|` builds OR nodes; `-` prefixes terms or parenthesized
//! groups for negation. Quoted strings are always message literals.

#![deny(clippy::unwrap_used)]

use thiserror::Error;

/// Supported filter field keys (Logcat key-value search subset).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilterKey {
    /// Process id column.
    Pid,
    /// User id column.
    Uid,
    /// Resolved package display name.
    Package,
    /// Log tag column.
    Tag,
    /// Minimum log level (`level:warn` keeps warn and above).
    Level,
    /// Message body (implicit for bare terms).
    Message,
    /// Maximum age relative to evaluation time (`age:5m`).
    Age,
}

impl FilterKey {
    /// Parses a key name from user input (ASCII case-insensitive).
    ///
    /// # Returns
    ///
    /// `None` when `name` is not a known filter key.
    pub fn parse_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "pid" => Some(Self::Pid),
            "uid" => Some(Self::Uid),
            "package" => Some(Self::Package),
            "tag" => Some(Self::Tag),
            "level" => Some(Self::Level),
            "message" => Some(Self::Message),
            "age" => Some(Self::Age),
            _ => None,
        }
    }

    /// All keys offered in the TUI autocomplete menu.
    pub fn autocomplete_keys() -> &'static [Self] {
        &[
            Self::Pid,
            Self::Uid,
            Self::Package,
            Self::Tag,
            Self::Level,
            Self::Message,
            Self::Age,
        ]
    }

    /// Returns true when `key~pattern` is meaningful for this field (not `age` / `level`).
    pub fn supports_regex_operator(self) -> bool {
        !matches!(self, Self::Age | Self::Level)
    }

    /// Canonical spelling for autocomplete insertion.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pid => "pid",
            Self::Uid => "uid",
            Self::Package => "package",
            Self::Tag => "tag",
            Self::Level => "level",
            Self::Message => "message",
            Self::Age => "age",
        }
    }
}

/// Byte range in the original query source (for syntax highlighting).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSpan {
    /// Inclusive start byte index.
    pub start: usize,
    /// Exclusive end byte index.
    pub end: usize,
}

/// One key/value predicate in the filter AST.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilterTerm {
    /// Field key for this predicate.
    pub key: FilterKey,
    /// Raw value text (quotes stripped when present).
    pub value: String,
    /// When true, match is inverted (`-` prefix).
    pub negated: bool,
    /// When true, `value` is a regular expression (`key~pattern`).
    pub regex: bool,
    /// Span covering the key name in the source query.
    pub key_span: Option<SourceSpan>,
    /// Span covering the value in the source query.
    pub value_span: Option<SourceSpan>,
    /// Span covering `:` or `~` between key and value.
    pub op_span: Option<SourceSpan>,
    /// Span covering a leading `-` negation operator.
    pub neg_span: Option<SourceSpan>,
}

/// Parsed filter expression tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FilterExpr {
    /// Leaf predicate.
    Term(FilterTerm),
    /// Logical AND (whitespace between terms).
    And(Vec<FilterExpr>),
    /// Logical OR (`|`).
    Or(Vec<FilterExpr>),
    /// Parenthesized sub-expression.
    Group(Box<FilterExpr>),
    /// Negated sub-expression (`-` before a parenthesized group).
    Not(Box<FilterExpr>),
}

/// Successfully parsed user or implicit query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilterQuery {
    /// Original source text.
    pub source: String,
    /// Root expression; `None` when the source is empty or whitespace-only.
    pub root: Option<FilterExpr>,
}

/// Parse failure with a human-readable message and source location.
#[derive(Clone, Debug, Eq, PartialEq, Error)]
#[error("{message} (at character {char_index})")]
pub struct ParseError {
    /// One-line description of the problem.
    pub message: String,
    /// Character index into the source where the error was detected (0-based).
    pub char_index: usize,
}

/// Parses `input` into a [`FilterQuery`].
///
/// # Arguments
///
/// * `input` — Raw query text from the filter bar or `--query`.
///
/// # Returns
///
/// A query whose `source` preserves `input`. Empty or whitespace-only input yields `root: None`
/// (matches all entries when no other constraints apply).
///
/// # Errors
///
/// Returns [`ParseError`] with a character index when the syntax is invalid or trailing garbage
/// remains after a successful parse.
pub fn parse_query(input: &str) -> Result<FilterQuery, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(FilterQuery {
            source: input.to_string(),
            root: None,
        });
    }

    let mut parser = Parser {
        source: input,
        chars: input.chars().peekable(),
        byte_offset: 0,
        char_index: 0,
    };
    let root = parser.parse_or()?;
    parser.skip_ws();
    if parser.peek().is_some() {
        return Err(parser.error("unexpected trailing input"));
    }
    Ok(FilterQuery {
        source: input.to_string(),
        root: Some(root),
    })
}

/// Returns true when `query` has no root (accepts every entry before implicit constraints).
///
/// # Arguments
///
/// * `query` — Parsed user or combined query.
pub fn query_accepts_all_entries(query: &FilterQuery) -> bool {
    query.root.is_none()
}

/// Incremental parser over one query string (byte and character cursors for error locations).
struct Parser<'a> {
    /// Original source slice (for span calculation and error indices).
    source: &'a str,
    /// Remaining characters to consume.
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    /// Current byte offset into `source`.
    byte_offset: usize,
    /// Current character index into `source` (for [`ParseError::char_index`]).
    char_index: usize,
}

impl<'a> Parser<'a> {
    /// Parses a chain of AND expressions separated by `|`.
    fn parse_or(&mut self) -> Result<FilterExpr, ParseError> {
        let mut parts = vec![self.parse_and()?];
        loop {
            self.skip_ws();
            if self.peek() == Some('|') {
                self.next_char();
                parts.push(self.parse_and()?);
            } else {
                break;
            }
        }
        Ok(flatten_or(parts))
    }

    /// Parses whitespace-separated NOT/primary terms until `|`, `)`, or end of input.
    fn parse_and(&mut self) -> Result<FilterExpr, ParseError> {
        let mut parts = Vec::default();
        loop {
            self.skip_ws();
            if matches!(self.peek(), None | Some(')') | Some('|')) {
                break;
            }
            parts.push(self.parse_not()?);
        }
        Ok(flatten_and(parts))
    }

    /// Parses an optional leading `-` and applies negation to the following primary.
    fn parse_not(&mut self) -> Result<FilterExpr, ParseError> {
        self.skip_ws();
        let neg_span = if self.peek() == Some('-') {
            let start = self.byte_offset;
            self.next_char();
            Some(SourceSpan {
                start,
                end: self.byte_offset,
            })
        } else {
            None
        };
        let mut expr = self.parse_primary()?;
        if let Some(span) = neg_span {
            expr = negate_expr(expr, span);
        }
        Ok(expr)
    }

    /// Parses a parenthesized sub-expression or a single [`FilterTerm`].
    fn parse_primary(&mut self) -> Result<FilterExpr, ParseError> {
        self.skip_ws();
        match self.peek() {
            Some('(') => {
                self.next_char();
                let inner = self.parse_or()?;
                self.skip_ws();
                if self.next_char() != Some(')') {
                    return Err(self.error("expected ')'"));
                }
                Ok(FilterExpr::Group(Box::new(inner)))
            }
            _ => self.parse_term().map(FilterExpr::Term),
        }
    }

    /// Parses one leaf predicate (quoted message, `key:value`, `key~regex`, or bare message word).
    fn parse_term(&mut self) -> Result<FilterTerm, ParseError> {
        let term_start = self.byte_offset;
        self.skip_ws();
        let first = self
            .peek()
            .ok_or_else(|| self.error("expected filter term"))?;

        if first == '"' {
            let (value, value_span) = self.read_quoted()?;
            return Ok(FilterTerm {
                key: FilterKey::Message,
                value,
                negated: false,
                regex: false,
                key_span: None,
                value_span: Some(value_span),
                op_span: None,
                neg_span: None,
            });
        }

        let (raw, raw_span) = self.read_bare_token()?;
        if raw.is_empty() {
            return Err(self.error("expected filter term"));
        }

        if let Some((key_name, rest)) = raw.split_once(':') {
            let key = FilterKey::parse_name(key_name).ok_or_else(|| {
                self.error_at(raw_span.start, &format!("unknown filter key '{key_name}'"))
            })?;
            let value = strip_quotes(rest.trim());
            let colon_byte = raw_span.start + key_name.len();
            return Ok(FilterTerm {
                key,
                value,
                negated: false,
                regex: false,
                key_span: Some(SourceSpan {
                    start: raw_span.start,
                    end: colon_byte + 1,
                }),
                op_span: Some(SourceSpan {
                    start: colon_byte,
                    end: colon_byte + 1,
                }),
                value_span: Some(SourceSpan {
                    start: colon_byte + 1,
                    end: raw_span.end,
                }),
                neg_span: None,
            });
        }

        if let Some((key_name, rest)) = raw.split_once('~') {
            let key = FilterKey::parse_name(key_name).ok_or_else(|| {
                self.error_at(raw_span.start, &format!("unknown filter key '{key_name}'"))
            })?;
            let value = strip_quotes(rest.trim());
            let tilde_byte = raw_span.start + key_name.len();
            return Ok(FilterTerm {
                key,
                value,
                negated: false,
                regex: true,
                key_span: Some(SourceSpan {
                    start: raw_span.start,
                    end: tilde_byte + 1,
                }),
                op_span: Some(SourceSpan {
                    start: tilde_byte,
                    end: tilde_byte + 1,
                }),
                value_span: Some(SourceSpan {
                    start: tilde_byte + 1,
                    end: raw_span.end,
                }),
                neg_span: None,
            });
        }

        if FilterKey::parse_name(&raw).is_some()
            && (self.peek() == Some(':') || self.peek() == Some('~'))
        {
            return Err(self.error_at(term_start, "expected ':' or '~' after key"));
        }

        Ok(FilterTerm {
            key: FilterKey::Message,
            value: raw,
            negated: false,
            regex: false,
            key_span: None,
            value_span: Some(raw_span),
            op_span: None,
            neg_span: None,
        })
    }

    /// Consumes a double-quoted string literal and returns the unescaped value and span.
    fn read_quoted(&mut self) -> Result<(String, SourceSpan), ParseError> {
        let start = self.byte_offset;
        if self.next_char() != Some('"') {
            return Err(self.error("expected '\"'"));
        }
        let mut value = String::new();
        while let Some(ch) = self.next_char() {
            if ch == '"' {
                return Ok((
                    value,
                    SourceSpan {
                        start,
                        end: self.byte_offset,
                    },
                ));
            }
            value.push(ch);
        }
        Err(self.error("unclosed double quote"))
    }

    /// Reads one unquoted token up to whitespace, `|`, `(`, or `)`.
    fn read_bare_token(&mut self) -> Result<(String, SourceSpan), ParseError> {
        let start = self.byte_offset;
        let mut out = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() || ch == '|' || ch == ')' || ch == '(' {
                break;
            }
            if let Some(ch) = self.next_char() {
                out.push(ch);
            } else {
                break;
            }
        }
        Ok((
            out,
            SourceSpan {
                start,
                end: self.byte_offset,
            },
        ))
    }

    /// Advances past ASCII whitespace.
    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.next_char();
        }
    }

    /// Returns the next character without consuming it.
    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    /// Consumes and returns the next character, updating byte and character cursors.
    fn next_char(&mut self) -> Option<char> {
        let ch = self.chars.next()?;
        self.byte_offset += ch.len_utf8();
        self.char_index += 1;
        Some(ch)
    }

    /// Builds a [`ParseError`] at the current character cursor.
    fn error(&self, message: &str) -> ParseError {
        ParseError {
            message: message.to_string(),
            char_index: self.char_index,
        }
    }

    /// Builds a [`ParseError`] at a specific byte offset in the source.
    fn error_at(&self, byte_offset: usize, message: &str) -> ParseError {
        ParseError {
            message: message.to_string(),
            char_index: byte_offset_to_char_index(self.source, byte_offset),
        }
    }
}

/// Converts a byte offset in `source` to a character index for error reporting.
fn byte_offset_to_char_index(source: &str, byte_offset: usize) -> usize {
    source[..byte_offset.min(source.len())].chars().count()
}

/// Removes surrounding `"` from a value token when present.
fn strip_quotes(value: &str) -> String {
    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

/// Normalizes a vector of AND operands into a single [`FilterExpr::And`] or one child.
fn flatten_and(parts: Vec<FilterExpr>) -> FilterExpr {
    if parts.is_empty() {
        FilterExpr::And(Vec::default())
    } else if parts.len() == 1 {
        parts.into_iter().next().unwrap_or_panic("len checked")
    } else {
        FilterExpr::And(parts)
    }
}

/// Normalizes a vector of OR operands into a single [`FilterExpr::Or`] or one child.
fn flatten_or(parts: Vec<FilterExpr>) -> FilterExpr {
    if parts.len() == 1 {
        parts.into_iter().next().unwrap_or_panic("len checked")
    } else {
        FilterExpr::Or(parts)
    }
}

/// Applies `-` negation to `expr`, toggling leaf terms or wrapping non-leaves in [`FilterExpr::Not`].
fn negate_expr(expr: FilterExpr, neg_span: SourceSpan) -> FilterExpr {
    match expr {
        FilterExpr::Term(mut term) => {
            term.negated = !term.negated;
            if term.neg_span.is_none() {
                term.neg_span = Some(neg_span);
            }
            FilterExpr::Term(term)
        }
        other => FilterExpr::Not(Box::new(other)),
    }
}

/// Local helper to avoid `Option::unwrap` under `#![deny(clippy::unwrap_used)]`.
trait UnwrapOrPanic {
    /// The contained value type.
    type Item;
    /// Returns the value or panics with `msg` (parser invariants only).
    fn unwrap_or_panic(self, msg: &str) -> Self::Item;
}

impl<T> UnwrapOrPanic for Option<T> {
    type Item = T;

    fn unwrap_or_panic(self, msg: &str) -> T {
        match self {
            Some(value) => value,
            None => panic!("{msg}"),
        }
    }
}
