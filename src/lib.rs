//! Declarative DSL for GPUI.
//!
//! This crate provides a small, Tailwind‑inspired macro DSL that expands to GPUI’s
//! native builder API. It is intentionally minimal and focuses on ergonomics.
//!
//! # Example
//!
//! ```no_run
//! use declarative_ui::*;
//!
//! ui! {
//!     col(size_full bg_0b0f14) {
//!         row(items_center justify_between p_12) {
//!             text("Title" text_lg bold)
//!             { right_panel }
//!         }
//!
//!         list(
//!             id="items"
//!             count=(items.len())
//!             render=|ix| {
//!                 let item = &items[ix];
//!                 text((item.name.clone()) text_sm)
//!             }
//!             gap_8
//!             p_12
//!         )
//!     }
//! }
//! ```
//!
//! # Colors
//!
//! `bg_` / `text_` / `border_` accept:
//!
//! - CSS‑like named colors: `black`, `white`, `gray`, `red`, `green`, `blue`,
//!   `yellow`, `cyan`, `magenta`, `orange`, `purple`
//! - Hex tokens: `bg_0b0f14`, `text_f2f0e9`, `border_ff00aa`, `bg_0ff`
//!
//! # Notes
//!
//! - Use parentheses for complex expressions in `text(...)` and in prop assignments.
//! - `if` supports `if flag { ... }` or `if (a && b) { ... }`.
//! - `for` supports `for item in items { ... }` or `for item in (items.iter()) { ... }`.

use gpui::{AnyElement, Div, FontWeight, Styled, div, prelude::*, px, rgb, uniform_list};
use std::borrow::Cow;
use std::sync::Arc;

fn parse_hex_color(name: &str) -> Option<u32> {
    let raw = name.strip_prefix("0x").unwrap_or(name);
    let is_hex = raw.chars().all(|c| c.is_ascii_hexdigit());
    if !is_hex {
        return None;
    }

    match raw.len() {
        6 => u32::from_str_radix(raw, 16).ok(),
        3 => {
            let mut digits = raw.chars().filter_map(|c| c.to_digit(16));
            let r = digits.next()? * 17;
            let g = digits.next()? * 17;
            let b = digits.next()? * 17;
            Some((r << 16) | (g << 8) | b)
        }
        _ => None,
    }
}

fn palette_color(name: &str) -> Option<u32> {
    if let Some(hex) = parse_hex_color(name) {
        return Some(hex);
    }

    match name {
        "black" => Some(0x000000),
        "white" => Some(0xffffff),
        "gray" => Some(0x808080),
        "red" => Some(0xff0000),
        "green" => Some(0x00ff00),
        "blue" => Some(0x0000ff),
        "yellow" => Some(0xffff00),
        "cyan" => Some(0x00ffff),
        "magenta" => Some(0xff00ff),
        "orange" => Some(0xffa500),
        "purple" => Some(0x800080),
        _ => None,
    }
}

fn parse_f32(value: &str) -> Option<f32> {
    value.parse::<f32>().ok()
}

fn parse_usize(value: &str) -> Option<usize> {
    value.parse::<usize>().ok()
}

fn parse_px(style: &str, prefix: &str) -> Option<f32> {
    style.strip_prefix(prefix).and_then(parse_f32)
}

/// Apply a single style string to a Styled element
pub fn apply_style<T: Styled>(el: T, style: &str) -> T {
    if let Some(name) = style.strip_prefix("bg-") {
        if let Some(color) = palette_color(name) {
            return el.bg(rgb(color));
        }
    }
    if let Some(name) = style.strip_prefix("text-") {
        if let Some(color) = palette_color(name) {
            return el.text_color(rgb(color));
        }
    }
    if let Some(name) = style.strip_prefix("border-") {
        if let Some(color) = palette_color(name) {
            return el.border_color(rgb(color));
        }
    }

    match style {
        "shadow" => return el.shadow_sm(),
        "shadow-none" => return el.shadow_none(),
        "shadow-2xs" => return el.shadow_2xs(),
        "shadow-xs" => return el.shadow_xs(),
        "shadow-sm" => return el.shadow_sm(),
        "shadow-md" => return el.shadow_md(),
        "shadow-lg" => return el.shadow_lg(),
        "shadow-xl" => return el.shadow_xl(),
        "shadow-2xl" => return el.shadow_2xl(),

        "rounded" => return el.rounded_sm(),
        "rounded-none" => return el.rounded_none(),
        "rounded-xs" => return el.rounded_xs(),
        "rounded-sm" => return el.rounded_sm(),
        "rounded-md" => return el.rounded_md(),
        "rounded-lg" => return el.rounded_lg(),
        "rounded-xl" => return el.rounded_xl(),
        "rounded-2xl" => return el.rounded_2xl(),
        "rounded-3xl" => return el.rounded_3xl(),
        "rounded-full" => return el.rounded_full(),

        "flex" => return el.flex(),
        "block" => return el.block(),
        "grid" => return el.grid(),
        "hidden" => return el.hidden(),
        "flex-col" | "col" => return el.flex_col(),
        "flex-row" | "row" => return el.flex_row(),
        "flex-wrap" => return el.flex_wrap(),
        "flex-wrap-reverse" => return el.flex_wrap_reverse(),
        "flex-nowrap" => return el.flex_nowrap(),
        "flex-1" => return el.flex_1(),
        "flex-auto" => return el.flex_auto(),
        "flex-initial" => return el.flex_initial(),
        "flex-none" => return el.flex_none(),
        "flex-grow" => return el.flex_grow(),
        "flex-shrink" => return el.flex_shrink(),
        "flex-shrink-0" => return el.flex_shrink_0(),
        "justify-start" => return el.justify_start(),
        "justify-end" => return el.justify_end(),
        "justify-center" => return el.justify_center(),
        "justify-between" => return el.justify_between(),
        "justify-around" => return el.justify_around(),
        "items-start" => return el.items_start(),
        "items-end" => return el.items_end(),
        "items-center" => return el.items_center(),
        "items-baseline" => return el.items_baseline(),
        "content-normal" => return el.content_normal(),
        "content-start" => return el.content_start(),
        "content-end" => return el.content_end(),
        "content-center" => return el.content_center(),
        "content-between" => return el.content_between(),
        "content-around" => return el.content_around(),
        "content-evenly" => return el.content_evenly(),
        "content-stretch" => return el.content_stretch(),
        "font-bold" | "bold" => return el.font_weight(FontWeight::BOLD),
        "font-semibold" | "semibold" => return el.font_weight(FontWeight::SEMIBOLD),
        "font-medium" | "medium" => return el.font_weight(FontWeight::MEDIUM),
        "italic" => return el.italic(),
        "not-italic" => return el.not_italic(),
        "underline" => return el.underline(),
        "line-through" => return el.line_through(),
        "no-underline" | "text-decoration-none" => return el.text_decoration_none(),
        "cursor-pointer" => return el.cursor_pointer(),
        "cursor-default" => return el.cursor_default(),
        "cursor-text" => return el.cursor_text(),
        "cursor-move" => return el.cursor_move(),
        "cursor-not-allowed" => return el.cursor_not_allowed(),
        "cursor-none" => return el.cursor(gpui::CursorStyle::None),
        "visible" => return el.visible(),
        "invisible" => return el.invisible(),
        "absolute" => return el.absolute(),
        "relative" => return el.relative(),
        "overflow-hidden" => return el.overflow_hidden(),
        "overflow-x-hidden" => return el.overflow_x_hidden(),
        "overflow-y-hidden" => return el.overflow_y_hidden(),
        "h-full" => return el.h_full(),
        "w-full" => return el.w_full(),
        "text-left" => return el.text_left(),
        "text-center" => return el.text_center(),
        "text-right" => return el.text_right(),
        "truncate" => return el.truncate(),
        "text-ellipsis" => return el.text_ellipsis(),
        "whitespace-nowrap" => return el.whitespace_nowrap(),
        "whitespace-normal" => return el.whitespace_normal(),
        "size-full" => return el.size_full(),
        "text-xs" => return el.text_xs(),
        "text-sm" => return el.text_sm(),
        "text-base" => return el.text_base(),
        "text-lg" => return el.text_lg(),
        "text-xl" => return el.text_xl(),
        "text-2xl" => return el.text_2xl(),
        "text-3xl" => return el.text_3xl(),
        "border" => return el.border(px(1.0)),
        "border-x" => return el.border_x(px(1.0)),
        "border-y" => return el.border_y(px(1.0)),
        "border-t" => return el.border_t(px(1.0)),
        "border-b" => return el.border_b(px(1.0)),
        "border-l" => return el.border_l(px(1.0)),
        "border-r" => return el.border_r(px(1.0)),
        _ => {}
    }

    if let Some(v) = parse_px(style, "gap-") {
        return el.gap(px(v));
    }
    if let Some(v) = parse_px(style, "p-") {
        return el.p(px(v));
    }
    if let Some(v) = parse_px(style, "pt-") {
        return el.pt(px(v));
    }
    if let Some(v) = parse_px(style, "pb-") {
        return el.pb(px(v));
    }
    if let Some(v) = parse_px(style, "pl-") {
        return el.pl(px(v));
    }
    if let Some(v) = parse_px(style, "pr-") {
        return el.pr(px(v));
    }
    if let Some(v) = parse_px(style, "px-") {
        return el.px(px(v));
    }
    if let Some(v) = parse_px(style, "py-") {
        return el.py(px(v));
    }
    if let Some(v) = parse_px(style, "m-") {
        return el.m(px(v));
    }
    if let Some(v) = parse_px(style, "mt-") {
        return el.mt(px(v));
    }
    if let Some(v) = parse_px(style, "mb-") {
        return el.mb(px(v));
    }
    if let Some(v) = parse_px(style, "ml-") {
        return el.ml(px(v));
    }
    if let Some(v) = parse_px(style, "mr-") {
        return el.mr(px(v));
    }
    if let Some(v) = parse_px(style, "mx-") {
        return el.mx(px(v));
    }
    if let Some(v) = parse_px(style, "my-") {
        return el.my(px(v));
    }
    if let Some(v) = parse_px(style, "w-") {
        return el.w(px(v));
    }
    if let Some(v) = parse_px(style, "h-") {
        return el.h(px(v));
    }
    if let Some(v) = parse_px(style, "size-") {
        return el.size(px(v));
    }
    if let Some(v) = parse_px(style, "min-w-") {
        return el.min_w(px(v));
    }
    if let Some(v) = parse_px(style, "max-w-") {
        return el.max_w(px(v));
    }
    if let Some(v) = parse_px(style, "min-h-") {
        return el.min_h(px(v));
    }
    if let Some(v) = parse_px(style, "max-h-") {
        return el.max_h(px(v));
    }
    if let Some(v) = parse_px(style, "top-") {
        return el.top(px(v));
    }
    if let Some(v) = parse_px(style, "bottom-") {
        return el.bottom(px(v));
    }
    if let Some(v) = parse_px(style, "left-") {
        return el.left(px(v));
    }
    if let Some(v) = parse_px(style, "right-") {
        return el.right(px(v));
    }
    if let Some(v) = parse_px(style, "inset-") {
        return el.inset(px(v));
    }
    if let Some(v) = parse_px(style, "border-") {
        return el.border(px(v));
    }
    if let Some(v) = parse_px(style, "border-x-") {
        return el.border_x(px(v));
    }
    if let Some(v) = parse_px(style, "border-y-") {
        return el.border_y(px(v));
    }
    if let Some(v) = parse_px(style, "border-t-") {
        return el.border_t(px(v));
    }
    if let Some(v) = parse_px(style, "border-b-") {
        return el.border_b(px(v));
    }
    if let Some(v) = parse_px(style, "border-l-") {
        return el.border_l(px(v));
    }
    if let Some(v) = parse_px(style, "border-r-") {
        return el.border_r(px(v));
    }
    if let Some(v) = parse_px(style, "rounded-") {
        return el.rounded(px(v));
    }
    if let Some(v) = parse_px(style, "text-") {
        return el.text_size(px(v));
    }
    if let Some(lines) = style.strip_prefix("line-clamp-").and_then(parse_usize) {
        return el.line_clamp(lines);
    }
    if let Some(mut v) = style.strip_prefix("opacity-").and_then(parse_f32) {
        if v > 1.0 {
            v /= 100.0;
        }
        let v = v.clamp(0.0, 1.0);
        return el.opacity(v);
    }

    #[cfg(debug_assertions)]
    if !style.is_empty() {
        eprintln!("[declarative_ui] unknown style token: `{}`", style);
    }
    el
}

/// Apply multiple space-separated styles to a Styled element
pub fn apply_styles<T: Styled>(mut el: T, styles: &str) -> T {
    for style in styles.split_whitespace() {
        el = apply_style(el, style);
    }
    el
}

/// Normalize a style token like `gap_12` into `gap-12`
pub fn normalize_style_token<'a>(token: &'a str) -> Cow<'a, str> {
    if token.contains('_') {
        Cow::Owned(token.replace('_', "-"))
    } else {
        Cow::Borrowed(token)
    }
}

/// Apply a single style token like `gap_12` or `bg_0b0f14`
pub fn apply_style_token<T: Styled>(el: T, token: &str) -> T {
    let normalized = normalize_style_token(token);
    apply_style(el, normalized.as_ref())
}

/// Join style tokens into a space-separated style string.
#[allow(dead_code)]
pub fn join_style_tokens(tokens: &[&str]) -> String {
    let mut out = String::new();
    for (idx, token) in tokens.iter().enumerate() {
        if idx > 0 {
            out.push(' ');
        }
        let normalized = normalize_style_token(token);
        out.push_str(normalized.as_ref());
    }
    out
}

/// Create a styled div from a style string
pub fn styled_div(styles: &str) -> Div {
    apply_styles(div(), styles)
}

// Base layout helpers for the new DSL
pub fn row() -> Div {
    styled_div("flex row")
}

pub fn col() -> Div {
    styled_div("flex col")
}

pub fn center() -> Div {
    styled_div("flex row items-center justify-center")
}

pub fn card() -> Div {
    styled_div("flex col")
}

/// Create a uniform_list with styling
pub fn styled_list<F>(id: &'static str, count: usize, styles: &str, renderer: F) -> AnyElement
where
    F: Fn(usize) -> AnyElement + Send + Sync + 'static,
{
    let renderer = Arc::new(renderer);
    let list = uniform_list(id, count, move |range, _window, _cx| {
        range.map(|ix| renderer(ix)).collect()
    });
    apply_styles(list, styles).into_any_element()
}

// ============================================================================
// Macros - work directly with GPUI Div
// ============================================================================

/// Main declarative UI macro
///
/// Example:
/// ```
/// ui! {
///     col(size_full bg_0b0f14) {
///         row(items_center justify_between) {
///             text(title text_lg bold)
///         }
///         { footer_panel }
///     }
/// }
/// ```
pub use declarative_macros::ui;

/// Build a node from the new DSL
#[macro_export]
macro_rules! ui_node {
    ( row, ( $($args:tt)* ), { $($children:tt)* } ) => {
        $crate::ui_build_node!($crate::row(), ( $($args)* ), { $($children)* })
    };
    ( row, ( $($args:tt)* ) ) => {
        $crate::ui_build_node!($crate::row(), ( $($args)* ))
    };

    ( col, ( $($args:tt)* ), { $($children:tt)* } ) => {
        $crate::ui_build_node!($crate::col(), ( $($args)* ), { $($children)* })
    };
    ( col, ( $($args:tt)* ) ) => {
        $crate::ui_build_node!($crate::col(), ( $($args)* ))
    };

    ( center, ( $($args:tt)* ), { $($children:tt)* } ) => {
        $crate::ui_build_node!($crate::center(), ( $($args)* ), { $($children)* })
    };
    ( center, ( $($args:tt)* ) ) => {
        $crate::ui_build_node!($crate::center(), ( $($args)* ))
    };

    ( card, ( $($args:tt)* ), { $($children:tt)* } ) => {
        $crate::ui_build_node!($crate::card(), ( $($args)* ), { $($children)* })
    };
    ( card, ( $($args:tt)* ) ) => {
        $crate::ui_build_node!($crate::card(), ( $($args)* ))
    };

    ( div, ( $($args:tt)* ), { $($children:tt)* } ) => {
        $crate::ui_build_node!($crate::styled_div(""), ( $($args)* ), { $($children)* })
    };
    ( div, ( $($args:tt)* ) ) => {
        $crate::ui_build_node!($crate::styled_div(""), ( $($args)* ))
    };

    // Fallback: call a function named after the node
    ( $name:ident, ( $($args:tt)* ), { $($children:tt)* } ) => {
        $crate::ui_build_node!($name(), ( $($args)* ), { $($children)* })
    };
    ( $name:ident, ( $($args:tt)* ) ) => {
        $crate::ui_build_node!($name(), ( $($args)* ))
    };
}

/// Build a uniform list from the DSL
#[macro_export]
macro_rules! ui_list {
    ( $($args:tt)* ) => {
        {
            #[allow(unused_assignments)]
            let mut id: Option<&'static str> = None;
            #[allow(unused_assignments)]
            let mut count: Option<usize> = None;
            #[allow(unused_assignments)]
            let mut render: Option<Box<dyn Fn(usize) -> gpui::AnyElement + Send + Sync + 'static>> = None;
            let mut styles: Vec<&'static str> = Vec::new();
            $crate::ui_list_args!(id, count, render, styles, $($args)*);
            if id.is_none() {
                panic!("list: missing id");
            }
            if count.is_none() {
                panic!("list: missing count");
            }
            if render.is_none() {
                panic!("list: missing render");
            }
            let styles = $crate::join_style_tokens(&styles);
            $crate::styled_list(
                id.unwrap(),
                count.unwrap(),
                &styles,
                render.expect("list: missing render"),
            )
        }
    };
}

/// Parse list(...) args
#[macro_export]
macro_rules! ui_list_args {
    ( $id:ident, $count:ident, $render:ident, $styles:ident, ) => {};

    ( $id:ident, $count:ident, $render:ident, $styles:ident, id = ( $value:expr ) $($rest:tt)* ) => {
        $id = Some($value);
        $crate::ui_list_args!($id, $count, $render, $styles, $($rest)*);
    };
    ( $id:ident, $count:ident, $render:ident, $styles:ident, id = $value:tt $($rest:tt)* ) => {
        $id = Some($value);
        $crate::ui_list_args!($id, $count, $render, $styles, $($rest)*);
    };

    ( $id:ident, $count:ident, $render:ident, $styles:ident, count = ( $value:expr ) $($rest:tt)* ) => {
        $count = Some($value);
        $crate::ui_list_args!($id, $count, $render, $styles, $($rest)*);
    };
    ( $id:ident, $count:ident, $render:ident, $styles:ident, count = $value:tt $($rest:tt)* ) => {
        $count = Some($value);
        $crate::ui_list_args!($id, $count, $render, $styles, $($rest)*);
    };

    ( $id:ident, $count:ident, $render:ident, $styles:ident, render = ( $value:expr ) $($rest:tt)* ) => {
        $render = Some(Box::new($value));
        $crate::ui_list_args!($id, $count, $render, $styles, $($rest)*);
    };
    ( $id:ident, $count:ident, $render:ident, $styles:ident, render = | $ix:ident | $body:block $($rest:tt)* ) => {
        $render = Some(Box::new(move |$ix| $body));
        $crate::ui_list_args!($id, $count, $render, $styles, $($rest)*);
    };
    ( $id:ident, $count:ident, $render:ident, $styles:ident, render = $value:tt $($rest:tt)* ) => {
        $render = Some(Box::new($value));
        $crate::ui_list_args!($id, $count, $render, $styles, $($rest)*);
    };

    ( $id:ident, $count:ident, $render:ident, $styles:ident, renderer = ( $value:expr ) $($rest:tt)* ) => {
        $render = Some(Box::new($value));
        $crate::ui_list_args!($id, $count, $render, $styles, $($rest)*);
    };
    ( $id:ident, $count:ident, $render:ident, $styles:ident, renderer = | $ix:ident | $body:block $($rest:tt)* ) => {
        $render = Some(Box::new(move |$ix| $body));
        $crate::ui_list_args!($id, $count, $render, $styles, $($rest)*);
    };
    ( $id:ident, $count:ident, $render:ident, $styles:ident, renderer = $value:tt $($rest:tt)* ) => {
        $render = Some(Box::new($value));
        $crate::ui_list_args!($id, $count, $render, $styles, $($rest)*);
    };

    ( $id:ident, $count:ident, $render:ident, $styles:ident, $style:ident $($rest:tt)* ) => {
        $styles.push(stringify!($style));
        $crate::ui_list_args!($id, $count, $render, $styles, $($rest)*);
    };
}

/// Apply args and children to a node
#[macro_export]
macro_rules! ui_build_node {
    ( $el:expr, ( $($args:tt)* ), { $($children:tt)* } ) => {
        {
            let mut el = $el;
            $crate::ui_apply_args!(el, $($args)*);
            $crate::ui_children!(el, $($children)*);
            el
        }
    };

    ( $el:expr, ( $($args:tt)* ) ) => {
        {
            let mut el = $el;
            $crate::ui_apply_args!(el, $($args)*);
            el
        }
    };
}

#[macro_export]
macro_rules! ui_apply_event_handler {
    ($el:ident, $method:ident, $handler:ident, $($rest:tt)*) => {
        $el = $el.$method(cx.listener(|this, _ev, _window, cx| {
            this.$handler(cx);
        }));
        $crate::ui_apply_args!($el, $($rest)*);
    };
}

#[macro_export]
macro_rules! ui_apply_event_closure {
    ($el:ident, $method:ident, $ev:ident, $window:ident, $cx:ident, $body:block, $($rest:tt)*) => {
        $el = $el.$method(|$ev, $window, $cx| $body);
        $crate::ui_apply_args!($el, $($rest)*);
    };
}

#[macro_export]
macro_rules! ui_apply_action_handler {
    ($el:ident, $handler:ident, $($rest:tt)*) => {
        $el = $el.on_action(cx.listener(|this, _action, _window, cx| {
            this.$handler(cx);
        }));
        $crate::ui_apply_args!($el, $($rest)*);
    };
}

#[macro_export]
macro_rules! ui_apply_action_closure {
    ($el:ident, $action:ty, $ev:ident, $window:ident, $cx:ident, $body:block, $($rest:tt)*) => {
        $el = $el.on_action::<$action>(|$ev, $window, $cx| $body);
        $crate::ui_apply_args!($el, $($rest)*);
    };
}

#[macro_export]
macro_rules! ui_apply_drop_handler {
    ($el:ident, $handler:ident, $($rest:tt)*) => {
        $el = $el.on_drop(cx.listener(|this, _value, _window, cx| {
            this.$handler(cx);
        }));
        $crate::ui_apply_args!($el, $($rest)*);
    };
}

#[macro_export]
macro_rules! ui_apply_drop_closure {
    ($el:ident, $value_ty:ty, $value:ident, $window:ident, $cx:ident, $body:block, $($rest:tt)*) => {
        $el = $el.on_drop::<$value_ty>(|$value, $window, $cx| $body);
        $crate::ui_apply_args!($el, $($rest)*);
    };
}

#[macro_export]
macro_rules! ui_apply_button_event_handler {
    ($el:ident, $method:ident, $handler:ident, $($rest:tt)*) => {
        $el = $el.$method(gpui::MouseButton::Left, cx.listener(|this, _ev, _window, cx| {
            this.$handler(cx);
        }));
        $crate::ui_apply_args!($el, $($rest)*);
    };
}

#[macro_export]
macro_rules! ui_apply_button_event_closure {
    ($el:ident, $method:ident, $ev:ident, $window:ident, $cx:ident, $body:block, $($rest:tt)*) => {
        $el = $el.$method(gpui::MouseButton::Left, |$ev, $window, $cx| $body);
        $crate::ui_apply_args!($el, $($rest)*);
    };
}

/// Apply style tokens and property assignments inside parentheses
#[macro_export]
macro_rules! ui_apply_args {
    ( $el:ident, ) => {};

    ( $el:ident, on_click = $handler:ident $($rest:tt)* ) => {
        $crate::ui_apply_event_handler!($el, on_click, $handler, $($rest)*);
    };
    ( $el:ident, on_click = | $ev:ident , $window:ident , $cx:ident | $body:block $($rest:tt)* ) => {
        $crate::ui_apply_event_closure!($el, on_click, $ev, $window, $cx, $body, $($rest)*);
    };

    ( $el:ident, on_any_mouse_down = $handler:ident $($rest:tt)* ) => {
        $crate::ui_apply_event_handler!($el, on_any_mouse_down, $handler, $($rest)*);
    };
    ( $el:ident, on_any_mouse_down = | $ev:ident , $window:ident , $cx:ident | $body:block $($rest:tt)* ) => {
        $crate::ui_apply_event_closure!($el, on_any_mouse_down, $ev, $window, $cx, $body, $($rest)*);
    };

    ( $el:ident, on_any_mouse_up = $handler:ident $($rest:tt)* ) => {
        $crate::ui_apply_event_handler!($el, on_any_mouse_up, $handler, $($rest)*);
    };
    ( $el:ident, on_any_mouse_up = | $ev:ident , $window:ident , $cx:ident | $body:block $($rest:tt)* ) => {
        $crate::ui_apply_event_closure!($el, on_any_mouse_up, $ev, $window, $cx, $body, $($rest)*);
    };

    ( $el:ident, on_mouse_move = $handler:ident $($rest:tt)* ) => {
        $crate::ui_apply_event_handler!($el, on_mouse_move, $handler, $($rest)*);
    };
    ( $el:ident, on_mouse_move = | $ev:ident , $window:ident , $cx:ident | $body:block $($rest:tt)* ) => {
        $crate::ui_apply_event_closure!($el, on_mouse_move, $ev, $window, $cx, $body, $($rest)*);
    };

    ( $el:ident, on_scroll_wheel = $handler:ident $($rest:tt)* ) => {
        $crate::ui_apply_event_handler!($el, on_scroll_wheel, $handler, $($rest)*);
    };
    ( $el:ident, on_scroll_wheel = | $ev:ident , $window:ident , $cx:ident | $body:block $($rest:tt)* ) => {
        $crate::ui_apply_event_closure!($el, on_scroll_wheel, $ev, $window, $cx, $body, $($rest)*);
    };

    ( $el:ident, on_key_down = $handler:ident $($rest:tt)* ) => {
        $crate::ui_apply_event_handler!($el, on_key_down, $handler, $($rest)*);
    };
    ( $el:ident, on_key_down = | $ev:ident , $window:ident , $cx:ident | $body:block $($rest:tt)* ) => {
        $crate::ui_apply_event_closure!($el, on_key_down, $ev, $window, $cx, $body, $($rest)*);
    };

    ( $el:ident, on_key_up = $handler:ident $($rest:tt)* ) => {
        $crate::ui_apply_event_handler!($el, on_key_up, $handler, $($rest)*);
    };
    ( $el:ident, on_key_up = | $ev:ident , $window:ident , $cx:ident | $body:block $($rest:tt)* ) => {
        $crate::ui_apply_event_closure!($el, on_key_up, $ev, $window, $cx, $body, $($rest)*);
    };

    ( $el:ident, on_hover = $handler:ident $($rest:tt)* ) => {
        $crate::ui_apply_event_handler!($el, on_hover, $handler, $($rest)*);
    };
    ( $el:ident, on_hover = | $ev:ident , $window:ident , $cx:ident | $body:block $($rest:tt)* ) => {
        $crate::ui_apply_event_closure!($el, on_hover, $ev, $window, $cx, $body, $($rest)*);
    };

    ( $el:ident, on_modifiers_changed = $handler:ident $($rest:tt)* ) => {
        $crate::ui_apply_event_handler!($el, on_modifiers_changed, $handler, $($rest)*);
    };
    ( $el:ident, on_modifiers_changed = | $ev:ident , $window:ident , $cx:ident | $body:block $($rest:tt)* ) => {
        $crate::ui_apply_event_closure!($el, on_modifiers_changed, $ev, $window, $cx, $body, $($rest)*);
    };

    ( $el:ident, on_action = $handler:ident $($rest:tt)* ) => {
        $crate::ui_apply_action_handler!($el, $handler, $($rest)*);
    };
    ( $el:ident, on_action = < $action:ty > | $ev:ident , $window:ident , $cx:ident | $body:block $($rest:tt)* ) => {
        $crate::ui_apply_action_closure!($el, $action, $ev, $window, $cx, $body, $($rest)*);
    };

    ( $el:ident, on_drop = $handler:ident $($rest:tt)* ) => {
        $crate::ui_apply_drop_handler!($el, $handler, $($rest)*);
    };
    ( $el:ident, on_drop = < $value_ty:ty > | $value:ident , $window:ident , $cx:ident | $body:block $($rest:tt)* ) => {
        $crate::ui_apply_drop_closure!($el, $value_ty, $value, $window, $cx, $body, $($rest)*);
    };

    ( $el:ident, on_mouse_down = $handler:ident $($rest:tt)* ) => {
        $crate::ui_apply_button_event_handler!($el, on_mouse_down, $handler, $($rest)*);
    };
    ( $el:ident, on_mouse_down = | $ev:ident , $window:ident , $cx:ident | $body:block $($rest:tt)* ) => {
        $crate::ui_apply_button_event_closure!($el, on_mouse_down, $ev, $window, $cx, $body, $($rest)*);
    };

    ( $el:ident, on_mouse_up = $handler:ident $($rest:tt)* ) => {
        $crate::ui_apply_button_event_handler!($el, on_mouse_up, $handler, $($rest)*);
    };
    ( $el:ident, on_mouse_up = | $ev:ident , $window:ident , $cx:ident | $body:block $($rest:tt)* ) => {
        $crate::ui_apply_button_event_closure!($el, on_mouse_up, $ev, $window, $cx, $body, $($rest)*);
    };

    ( $el:ident, on_mouse_down_out = $handler:ident $($rest:tt)* ) => {
        $crate::ui_apply_button_event_handler!($el, on_mouse_down_out, $handler, $($rest)*);
    };
    ( $el:ident, on_mouse_down_out = | $ev:ident , $window:ident , $cx:ident | $body:block $($rest:tt)* ) => {
        $crate::ui_apply_button_event_closure!($el, on_mouse_down_out, $ev, $window, $cx, $body, $($rest)*);
    };

    ( $el:ident, on_mouse_up_out = $handler:ident $($rest:tt)* ) => {
        $crate::ui_apply_button_event_handler!($el, on_mouse_up_out, $handler, $($rest)*);
    };
    ( $el:ident, on_mouse_up_out = | $ev:ident , $window:ident , $cx:ident | $body:block $($rest:tt)* ) => {
        $crate::ui_apply_button_event_closure!($el, on_mouse_up_out, $ev, $window, $cx, $body, $($rest)*);
    };

    // on_mouse_down/up = (listener_expr) — wraps in MouseButton::Left automatically
    ( $el:ident, on_mouse_down = ( $handler:expr ) $($rest:tt)* ) => {
        $el = $el.on_mouse_down(gpui::MouseButton::Left, $handler);
        $crate::ui_apply_args!($el, $($rest)*);
    };
    ( $el:ident, on_mouse_up = ( $handler:expr ) $($rest:tt)* ) => {
        $el = $el.on_mouse_up(gpui::MouseButton::Left, $handler);
        $crate::ui_apply_args!($el, $($rest)*);
    };
    // on_mouse_move = (listener_expr)
    ( $el:ident, on_mouse_move = ( $handler:expr ) $($rest:tt)* ) => {
        $el = $el.on_mouse_move($handler);
        $crate::ui_apply_args!($el, $($rest)*);
    };
    // generic property assignment: prop = (expr)
    ( $el:ident, $prop:ident = ( $value:expr ) $($rest:tt)* ) => {
        $el = $el.$prop($value);
        $crate::ui_apply_args!($el, $($rest)*);
    };

    ( $el:ident, $prop:ident = $value:tt $($rest:tt)* ) => {
        $el = $el.$prop($value);
        $crate::ui_apply_args!($el, $($rest)*);
    };

    ( $el:ident, $style:ident $($rest:tt)* ) => {
        $el = $crate::apply_style_token($el, stringify!($style));
        $crate::ui_apply_args!($el, $($rest)*);
    };
}

/// Text node helper: text(content styles...)
#[macro_export]
macro_rules! ui_text {
    ( $content:ident $( $style:ident )* ) => {
        {
            let mut el = $crate::styled_div("");
            $(
                el = $crate::apply_style_token(el, stringify!($style));
            )*
            el.child($content)
        }
    };

    ( $content:literal $( $style:ident )* ) => {
        {
            let mut el = $crate::styled_div("");
            $(
                el = $crate::apply_style_token(el, stringify!($style));
            )*
            el.child($content)
        }
    };

    ( ( $content:expr ) $( $style:ident )* ) => {
        {
            let mut el = $crate::styled_div("");
            $(
                el = $crate::apply_style_token(el, stringify!($style));
            )*
            el.child($content)
        }
    };

    ( $content:expr, $( $style:ident )+ ) => {
        {
            let mut el = $crate::styled_div("");
            $(
                el = $crate::apply_style_token(el, stringify!($style));
            )*
            el.child($content)
        }
    };

    ( $content:expr ) => {
        $content
    };
}

/// Helper macro to collect children
#[macro_export]
macro_rules! ui_children {
    // Base case - no more children
    ($el:ident, ) => {};

    // if/else blocks
    ($el:ident, if $cond:ident { $($then:tt)* } else { $($else:tt)* } $($rest:tt)* ) => {
        if $cond {
            $crate::ui_children!($el, $($then)*);
        } else {
            $crate::ui_children!($el, $($else)*);
        }
        $crate::ui_children!($el, $($rest)*);
    };

    ($el:ident, if ( $cond:expr ) { $($then:tt)* } else { $($else:tt)* } $($rest:tt)* ) => {
        if $cond {
            $crate::ui_children!($el, $($then)*);
        } else {
            $crate::ui_children!($el, $($else)*);
        }
        $crate::ui_children!($el, $($rest)*);
    };

    // if without else
    ($el:ident, if $cond:ident { $($then:tt)* } $($rest:tt)* ) => {
        if $cond {
            $crate::ui_children!($el, $($then)*);
        }
        $crate::ui_children!($el, $($rest)*);
    };

    ($el:ident, if ( $cond:expr ) { $($then:tt)* } $($rest:tt)* ) => {
        if $cond {
            $crate::ui_children!($el, $($then)*);
        }
        $crate::ui_children!($el, $($rest)*);
    };

    // for loop - new syntax
    ($el:ident, for $item:ident in $iter:ident { $($body:tt)* } $($rest:tt)* ) => {
        for $item in $iter {
            $crate::ui_children!($el, $($body)*);
        }
        $crate::ui_children!($el, $($rest)*);
    };

    ($el:ident, for $item:ident in ( $iter:expr ) { $($body:tt)* } $($rest:tt)* ) => {
        for $item in $iter {
            $crate::ui_children!($el, $($body)*);
        }
        $crate::ui_children!($el, $($rest)*);
    };

    // text(content styles...)
    ($el:ident, text ( $($content:tt)+ ) $($rest:tt)* ) => {
        $el = $el.child($crate::ui_text!($($content)+));
        $crate::ui_children!($el, $($rest)*);
    };

    // text_raw(content) - raw text child with no wrapper
    ($el:ident, text_raw ( $content:expr ) $($rest:tt)* ) => {
        $el = $el.child($content);
        $crate::ui_children!($el, $($rest)*);
    };

    // label(content) - alias for text_raw
    ($el:ident, label ( $content:expr ) $($rest:tt)* ) => {
        $el = $el.child($content);
        $crate::ui_children!($el, $($rest)*);
    };

    // { component } - explicit component child
    ($el:ident, { $child:expr } $($rest:tt)* ) => {
        $el = $el.child($child);
        $crate::ui_children!($el, $($rest)*);
    };

    // list(...) child
    ($el:ident, list ( $($args:tt)* ) $($rest:tt)* ) => {
        $el = $el.child($crate::ui_list!($($args)*));
        $crate::ui_children!($el, $($rest)*);
    };

    // node with children (new DSL)
    ($el:ident, $node:ident ( $($args:tt)* ) { $($body:tt)* } $($rest:tt)* ) => {
        $el = $el.child($crate::ui! { $node( $($args)* ) { $($body)* } });
        $crate::ui_children!($el, $($rest)*);
    };

    // node without children (new DSL)
    ($el:ident, $node:ident ( $($args:tt)* ) $($rest:tt)* ) => {
        $el = $el.child($crate::ui! { $node( $($args)* ) });
        $crate::ui_children!($el, $($rest)*);
    };

}
