//! Integration surface for the `ui!` macro against the real GPUI API.
//!
//! Every function here is a compile-time test: it expands DSL features into
//! actual GPUI builder calls, so this crate compiling *is* the assertion.
//! GPUI is pinned to the revision the macro's token tables were verified
//! against; bumping that rev and rebuilding detects API drift (e.g. a token
//! that now targets a renamed or removed method).
//!
//! The `#[test]`s at the bottom additionally construct the element trees at
//! runtime (no window required), catching panics in constructors.

use declarative_gpui::{color, ui, ui_expand};
use gpui::IntoElement;
use gpui::prelude::*;

/// Built-in containers with the core token classes: spacing, colors (named,
/// 3/6-digit hex), rounding, borders, shadows, layout, cursor, text styles.
pub fn containers_and_tokens() -> impl IntoElement {
    ui! {
        col(gap_4 p_16 bg_1c1a17 rounded_lg shadow_md border border_2d2b27) {
            row(px_8 py_2 gap_2 items_center justify_between flex_wrap) {
                text("styled" text_sm semibold text_f5f0e8 underline text_decoration_1)
                text("named colors" bg_red text_white rounded_full px_4)
                text("short hex" text_abc bg_222)
            }
            center(size_full min_h_240 cursor_pointer overflow_hidden) {
                card(w_240 h_128 rounded_xl shadow_lg items_start opacity_50) {
                    text("card")
                }
            }
            div(flex_1 truncate line_clamp_2 whitespace_nowrap text_ellipsis) {
                text("clamped")
            }
        }
    }
}

/// GPUI-parity tokens added in the styled-API sweep: self-alignment,
/// justify_evenly, aspect, ellipsis variants, flex grow/shrink forms.
pub fn parity_tokens() -> impl IntoElement {
    ui! {
        row(justify_evenly items_stretch content_between) {
            div(self_start flex_grow_1) { text("a") }
            div(self_center flex_shrink_0 aspect_square w_64) { text("b") }
            div(self_stretch flex_grow_0 flex_shrink_1) {
                text("ellipsis" text_ellipsis_middle)
            }
        }
    }
}

/// Box-model suffixes: auto / full / px / fractions, negatives, `p`-decimals,
/// min/max sizes, scrollbar width.
pub fn box_model_suffixes() -> impl IntoElement {
    ui! {
        row(w_full gap_px) {
            div(w_1_2 mx_auto p_2p5 mt_neg_8) {}
            div(w_1_3 h_auto min_w_full max_h_128 border_0p5) {}
            div(w_px min_size_24 max_size_128 flex_basis_1_2 top_neg_4 relative) {}
            div(flex_basis_auto scrollbar_width_8 inset_auto) {}
        }
    }
}

/// Grid layout: template, spans, placement (incl. negative and content-sized).
pub fn grid_layout() -> impl IntoElement {
    ui! {
        div(grid grid_cols_3 grid_rows_2 gap_x_8 gap_y_4) {
            div(col_span_2) { text("wide") }
            div(col_start_1 row_start_2) { text("placed") }
            div(col_start_neg_1) { text("from-end") }
            div(col_span_full grid_cols_min_content_2) { text("nested") }
        }
    }
}

/// Alpha colors (8/4-digit hex) and text-decoration color — all emitted as
/// precomputed `Hsla` literals.
pub fn alpha_and_decoration_colors() -> impl IntoElement {
    ui! {
        div(bg_1c1a17cc border border_ff000080) {
            text("ghost" text_abcd)
            text("wavy" underline text_decoration_wavy text_decoration_ff0000)
            text("highlight" text_bg_ffff00)
        }
    }
}

/// text / text_raw / label; unstyled text expands bare (no wrapper div).
pub fn text_variants() -> impl IntoElement {
    ui! {
        col() {
            text("unstyled — no wrapper div")
            text_raw("raw" text_lg)
            label("label" font_bold italic)
            text(format!("dynamic {}", 42))
        }
    }
}

/// scroll: auto-generated stable element ID + overflow_y_scroll, returned as
/// concrete Stateful<Div> (not boxed). Explicit-id overflow tokens too.
pub fn scrolling() -> impl IntoElement {
    ui! {
        scroll(h_full w_240) {
            for i in 0..32 {
                text(format!("row {i}"))
            }
        }
    }
}

pub fn overflow_on_stateful() -> impl IntoElement {
    ui! {
        div(id = "ov" overflow_x_scroll w_128) { text("wide content") }
    }
}

/// list → gpui::uniform_list, closure owns the render function (no Arc).
pub fn uniform_list_element() -> impl IntoElement {
    ui! {
        list(
            id = "items",
            count = 100,
            render = |ix: usize| gpui::div().child(format!("item {ix}")).into_any_element(),
            flex_1 px_4
        )
    }
}

/// Control flow: if / else-if / else, for, match with guards — in child
/// position.
pub fn control_flow(state: Option<i32>, items: Vec<String>) -> impl IntoElement {
    ui! {
        col(gap_2) {
            if state.is_some() {
                text("some")
            } else if items.is_empty() {
                text("empty")
            } else {
                text("other")
            }
            for item in items {
                row(gap_1) { text(item) }
            }
            match state {
                Some(v) if v > 0 => text("positive"),
                Some(_) => { text("non-positive") },
                None => {}
            }
        }
    }
}

/// `if let` with multi-node bodies, else-if-let chains, and a `{ ... }`
/// block directly after a leaf element (text/list) parsing as a sibling.
pub fn if_let_and_leaf_siblings(
    modal: Option<String>,
    warning: Option<String>,
) -> impl IntoElement {
    ui! {
        col(gap_2) {
            if let Some(m) = modal {
                { gpui::div().child("backdrop") }
                text(m text_sm)
            } else if let Some(w) = warning {
                text(w)
            }
            text("leaf") { gpui::div().child("sibling of the text, not a child") }
            list(count = 3, render = |ix: usize| gpui::div().child(format!("{ix}")).into_any_element())
            { gpui::div().child("sibling of the list") }
        }
    }
}

/// `color!`: compile-time `Hsla` literals are const-constructible, so theme
/// palettes carry no per-frame conversion — a themed `bg = th.panel` is a
/// field copy.
pub struct Theme {
    pub panel: gpui::Hsla,
    pub text: gpui::Hsla,
    pub accent: gpui::Hsla,
}

pub const LIGHT: Theme = Theme {
    panel: color!(f5f0e8),
    text: color!("#1c1a17"),
    accent: color!(ff6b3580),
};

pub const DARK: Theme = Theme {
    panel: color!(1c1a17),
    text: color!(f5f0e8),
    accent: color!(orange),
};

pub fn themed(dark: bool) -> impl IntoElement {
    let th = if dark { &DARK } else { &LIGHT };
    ui! {
        col(bg = th.panel, text_color = th.text, border, border_color = th.accent) {
            text("themed with zero runtime color math")
        }
    }
}

/// Spread `{ ..expr }`: anything `IntoIterator<Item: IntoElement>` — an
/// `Option` (renders nothing on `None`), a heterogeneous `Vec<AnyElement>`,
/// a mapped iterator.
pub fn spread_children(badge: Option<String>, extras: Vec<gpui::AnyElement>) -> impl IntoElement {
    ui! {
        col(gap_2) {
            { ..badge }
            { ..extras }
            { ..(0..3).map(|i| gpui::div().child(format!("row {i}"))) }
        }
    }
}

/// Escape hatches: `{ expr }` blocks, custom element constructors,
/// key = value for arbitrary builder methods, call-style args.
pub fn badge() -> gpui::Div {
    gpui::div()
}

pub fn escape_hatches(show: bool) -> impl IntoElement {
    ui! {
        col() {
            { gpui::div().child("arbitrary expression") }
            badge(px_4 bg_gray rounded_sm)
            div(w(gpui::rems(2.0)) when(show, |el| el.opacity(0.5))) {
                text("rem-scale via call args")
            }
        }
    }
}

/// Event handlers: mouse-button defaulting, stateful auto-ID injection for
/// on_hover / on_click / active / overflow tokens, explicit id folding,
/// hover style refinement.
pub fn interactivity() -> impl IntoElement {
    ui! {
        col() {
            div(on_mouse_down = |_event, _window, _cx| {}) { text("left click") }
            row(on_hover = |_hovered, _window, _cx| {}) { text("auto stateful id") }
            div(id = "btn" on_click = |_event, _window, _cx| {} cursor_pointer) {
                text("explicit id folded into constructor")
            }
            div(on_click = |_event, _window, _cx| {}) { text("auto stateful id for click") }
            div(active(|s| s.opacity(0.9)) w_64) { text("active refinement") }
            div(overflow_x_scroll w_128) { text("overflow token auto id") }
            div(hover(|s| s.opacity(0.8)) group("g")) { text("hover style") }
        }
    }
}

/// Conditional style tokens: `when(cond, <tokens>)` expands the trailing
/// args inside a `.when()` closure — compile-time colors survive
/// conditional (e.g. theme-dependent) styling.
pub fn conditional_tokens(dark: bool) -> impl IntoElement {
    let th = if dark { &DARK } else { &LIGHT };
    ui! {
        col(px_4, when(dark, bg_1c1a17, text_f5f0e8, rounded_lg)) {
            text("token form")
            div(when(dark, bg = th.panel, hover(|s| s.opacity(0.9)))) {
                text("key = value and call-style inside when")
            }
            div(when(dark, |el| el.opacity(0.5))) { text("closure passthrough") }
        }
    }
}

/// Top-level control flow (no parent element) and multi-node bodies.
pub fn top_level_branching(dark: bool) -> gpui::Div {
    ui! {
        if dark {
            div(bg_black text_white) { text("dark") }
        } else {
            div(bg_white text_black) { text("light") }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Constructing the trees requires no Window/App — this catches runtime
    /// panics in constructors (bad ElementIds, etc.), not just type errors.
    #[test]
    fn all_trees_construct() {
        let _ = containers_and_tokens();
        let _ = parity_tokens();
        let _ = box_model_suffixes();
        let _ = grid_layout();
        let _ = alpha_and_decoration_colors();
        let _ = text_variants();
        let _ = scrolling();
        let _ = overflow_on_stateful();
        let _ = uniform_list_element();
        let _ = control_flow(Some(3), vec!["a".into(), "b".into()]);
        let _ = control_flow(None, vec![]);
        let _ = if_let_and_leaf_siblings(Some("m".into()), None);
        let _ = if_let_and_leaf_siblings(None, Some("w".into()));
        let _ = spread_children(Some("new".into()), vec![badge().into_any_element()]);
        let _ = spread_children(None, vec![]);
        let _ = themed(true);
        let _ = themed(false);
        let _ = conditional_tokens(true);
        let _ = conditional_tokens(false);
    }

    /// `ui_expand!` yields the generated builder code as a string constant.
    #[test]
    fn ui_expand_previews_generated_code() {
        const PREVIEW: &str = ui_expand! { row(px_8 bg_1c1a17) { text("hi") } };
        assert!(PREVIEW.contains("gpui::div()"), "{PREVIEW}");
        assert!(PREVIEW.contains("Hsla"), "{PREVIEW}");
        assert!(
            PREVIEW.contains('\n'),
            "should be pretty-printed: {PREVIEW}"
        );
        let _ = escape_hatches(true);
        let _ = interactivity();
        let _ = top_level_branching(false);
    }
}
