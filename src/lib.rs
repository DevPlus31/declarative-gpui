//! Declarative `ui!` macro for building GPUI element trees.
//!
//! All parsing and expansion logic lives in `declarative-gpui-core` — a
//! normal library crate, so it can be unit-tested, benchmarked, and fuzzed
//! (proc-macro crates export nothing but their macros, and the `proc_macro`
//! API is only usable inside a real expansion). This crate is just the entry
//! point converting at the `proc_macro` / `proc_macro2` boundary.

use proc_macro::TokenStream;

/// Build a GPUI element tree declaratively.
///
/// Elements take style tokens, `key = value` builder calls, and call-style
/// args; children go in braces, and `if`/`for`/`match` expand to real Rust
/// control flow. See the crate README for the full token reference.
///
/// ```ignore
/// use declarative_gpui::ui;
/// use gpui::prelude::*;
///
/// let view = ui! {
///     col(gap_4 p_16 bg_1c1a17 rounded_lg) {
///         text("Hello" text_lg semibold text_f5f0e8)
///         for item in items {
///             row(gap_2) { text(item.name) }
///         }
///     }
/// };
/// ```
#[proc_macro]
pub fn ui(input: TokenStream) -> TokenStream {
    declarative_gpui_core::ui_impl(input.into()).into()
}
