//! The "Team Inbox" card, hand-written the way GPUI code is typically
//! written today: chained builders, `rgb()` color constructors (converted to
//! Hsla at runtime, every build), explicit flex setup.
//!
//! `src/dsl.rs` renders the identical UI with the `ui!` macro — diff the two
//! files to see the code difference the macro buys.

use crate::data::Item;
use gpui::prelude::*;
use gpui::{FontWeight, IntoElement, div, px, rgb};

pub fn view(items: &[Item]) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .p(px(16.))
        .bg(rgb(0x1c1a17))
        .rounded_xl()
        .border(px(1.))
        .border_color(rgb(0x2d2b27))
        .w_full()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(0xf5f0e8))
                                .child("Team Inbox"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x8a857c))
                                .child("realtime presence"),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(0x9ece6a))
                        .bg(rgb(0x1e2a1e))
                        .px(px(8.))
                        .py(px(2.))
                        .rounded_full()
                        .child("LIVE"),
                ),
        )
        .child(div().h(px(1.)).w_full().bg(rgb(0x2d2b27)))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .children(items.iter().map(|item| {
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(12.))
                        .px(px(12.))
                        .py(px(8.))
                        .rounded_md()
                        .bg(rgb(0x24211d))
                        .border(px(1.))
                        .border_color(rgb(0x2d2b27))
                        .items_center()
                        .child(div().size(px(32.)).rounded_md().bg(rgb(item.accent)))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(0xf5f0e8))
                                        .child(item.name.clone()),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0x8a857c))
                                        .child(item.role.clone()),
                                ),
                        )
                        .child(if item.active {
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(0x9ece6a))
                                .bg(rgb(0x1e2a1e))
                                .px(px(8.))
                                .py(px(2.))
                                .rounded_full()
                                .child("ACTIVE")
                        } else {
                            div()
                                .text_xs()
                                .text_color(rgb(0x8a857c))
                                .bg(rgb(0x262320))
                                .px(px(8.))
                                .py(px(2.))
                                .rounded_full()
                                .child("IDLE")
                        })
                })),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .pt(px(8.))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x8a857c))
                        .child(format!("{} members", items.len())),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(8.))
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(0x1c1a17))
                                .bg(rgb(0xe0b184))
                                .px(px(12.))
                                .py(px(4.))
                                .rounded_md()
                                .cursor_pointer()
                                .child("Invite"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0xf5f0e8))
                                .border(px(1.))
                                .border_color(rgb(0x2d2b27))
                                .px(px(12.))
                                .py(px(4.))
                                .rounded_md()
                                .cursor_pointer()
                                .child("Settings"),
                        ),
                ),
        )
}
