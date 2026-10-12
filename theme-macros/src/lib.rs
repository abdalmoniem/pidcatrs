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
//! The [`get_bundled_themes!`] macro expands to a static slice of bundled theme names and
//! `include_str!` sources at compile time.

#![deny(clippy::unwrap_used)]

use proc_macro::TokenStream;
use proc_macro2::Literal;

use quote::quote;

use std::fs::read_dir;
use std::path::Path;

use syn::LitStr;
use syn::parse_macro_input;

/// Expands to a `&'static [BundledTheme]` literal for every `.toml` file under a themes directory.
///
/// # Arguments
///
/// The macro input must be a string literal path relative to the invoking crate's manifest
/// directory (for example `"themes"`).
///
/// # Panics
///
/// Panics at compile time when the directory cannot be read or a theme file name is invalid.
#[proc_macro]
pub fn get_bundled_themes(input: TokenStream) -> TokenStream {
    let themes_dir = parse_macro_input!(input as LitStr);
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
    let themes_dir_path = Path::new(&manifest_dir).join(themes_dir.value());

    let mut theme_files = read_dir(&themes_dir_path)
        .unwrap_or_else(|err| panic!("Failed to read {}: {err}", themes_dir_path.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|err| panic!("Failed to read theme entry: {err}"))
                .path()
        })
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect::<Vec<_>>();

    theme_files.sort();

    let themes = theme_files.iter().map(|path| {
        let name = path
            .file_stem()
            .unwrap_or_else(|| panic!("Invalid theme file name: {path:?}"))
            .to_string_lossy()
            .into_owned();
        let name = LitStr::new(&name, proc_macro2::Span::call_site());
        let path = Literal::string(path.to_str().expect("Theme path is not valid UTF-8"));

        quote! {
            BundledTheme {
                name: #name,
                source: include_str!(#path),
            }
        }
    });

    quote! {
        &[#(#themes),*]
    }
    .into()
}
