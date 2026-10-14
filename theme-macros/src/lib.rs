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

//! Proc-macros for pidcatrs build-time theme embedding.
//!
//! The [`get_bundled_themes!`] macro walks `{input}/light` and `{input}/dark`, embedding every
//! `.toml` file as a `BundledTheme { name, category, source }` literal with `include_str!`.

#![deny(clippy::unwrap_used)]

use proc_macro::TokenStream;
use proc_macro2::Literal;

use quote::quote;

use std::fs::read_dir;
use std::path::Path;

use syn::LitStr;
use syn::parse_macro_input;

/// Category subdirectories scanned under the themes root passed to [`get_bundled_themes!`].
const CATEGORIES: &[&str] = &["light", "dark"];

/// Expands to a `&'static [BundledTheme]` literal for every `.toml` file under
/// `{input}/light` and `{input}/dark`.
///
/// Each literal sets `category` to `"light"` or `"dark"` and `source` via `include_str!`.
///
/// # Arguments
///
/// The macro input must be a string literal path relative to the invoking crate's manifest
/// directory (for example `"src/config/themes"`).
///
/// # Panics
///
/// Panics at compile time when a category directory cannot be read or a theme file name is
/// invalid.
#[proc_macro]
pub fn get_bundled_themes(input: TokenStream) -> TokenStream {
    let themes_root = parse_macro_input!(input as LitStr);
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
    let themes_root_path = Path::new(&manifest_dir).join(themes_root.value());

    let mut themes = Vec::new();

    for category in CATEGORIES {
        let category_dir = themes_root_path.join(category);
        let mut theme_files = read_dir(&category_dir)
            .unwrap_or_else(|err| panic!("Failed to read {}: {err}", category_dir.display()))
            .map(|entry| {
                entry
                    .unwrap_or_else(|err| panic!("Failed to read theme entry: {err}"))
                    .path()
            })
            .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
            .collect::<Vec<_>>();

        theme_files.sort();

        for path in theme_files {
            let name = path
                .file_stem()
                .unwrap_or_else(|| panic!("Invalid theme file name: {path:?}"))
                .to_string_lossy()
                .into_owned();
            let name_lit = LitStr::new(&name, proc_macro2::Span::call_site());
            let category_lit = LitStr::new(category, proc_macro2::Span::call_site());
            let path_lit = Literal::string(path.to_str().expect("Theme path is not valid UTF-8"));

            themes.push(quote! {
                BundledTheme {
                    name: #name_lit,
                    category: #category_lit,
                    source: include_str!(#path_lit),
                }
            });
        }
    }

    quote! {
        &[#(#themes),*]
    }
    .into()
}
