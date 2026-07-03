//! The identical "Team Inbox" card via the `ui!` macro. Same structure, same
//! values, same rendered pixels as `src/hand_written.rs` — but colors are
//! precomputed to `Hsla` at compile time (the hand-written version pays
//! `rgb()` unpacking + RGB→HSL conversion on every build).

use crate::data::Item;
use declarative_gpui::ui;
use gpui::IntoElement;
use gpui::prelude::*;

pub fn view(items: &[Item]) -> impl IntoElement {
    ui! {
        col(gap_12, p_16, bg_1c1a17, rounded_xl, border, border_2d2b27, w_full) {
            row(items_center, justify_between) {
                col(gap_2) {
                    text("Team Inbox", text_lg, semibold, text_f5f0e8)
                    text("realtime presence", text_xs, text_8a857c)
                }
                text("LIVE", text_xs, bold, text_9ece6a, bg_1e2a1e, px_8, py_2, rounded_full)
            }
            div(h_px, w_full, bg_2d2b27) {}
            col(gap_8) {
                for item in items {
                    row(gap_12, px_12, py_8, rounded_md, bg_24211d, border, border_2d2b27, items_center) {
                        div(size_32, rounded_md, bg = gpui::rgb(item.accent)) {}
                        col(flex_1, gap_2) {
                            text(item.name.clone(), text_sm, semibold, text_f5f0e8)
                            text(item.role.clone(), text_xs, text_8a857c)
                        }
                        if item.active {
                            text("ACTIVE", text_xs, semibold, text_9ece6a, bg_1e2a1e, px_8, py_2, rounded_full)
                        } else {
                            text("IDLE", text_xs, text_8a857c, bg_262320, px_8, py_2, rounded_full)
                        }
                    }
                }
            }
            row(items_center, justify_between, pt_8) {
                text(format!("{} members", items.len()), text_xs, text_8a857c)
                row(gap_8) {
                    text("Invite", text_xs, semibold, text_1c1a17, bg_e0b184, px_12, py_4, rounded_md, cursor_pointer)
                    text("Settings", text_xs, text_f5f0e8, border, border_2d2b27, px_12, py_4, rounded_md, cursor_pointer)
                }
            }
        }
    }
}
