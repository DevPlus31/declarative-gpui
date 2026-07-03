//! Side-by-side showcase: the same UI built with hand-written GPUI builder
//! chains (left) and the `ui!` macro (right) — identical pixels, zero runtime
//! overhead, in a fraction of the code. Hover a pane's "view code" button to
//! see the source that renders it.
//!
//!     cargo run -p declarative-gpui-examples --release

mod data;
mod dsl;
mod hand_written;

use std::time::Instant;

use declarative_gpui::ui;
use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, Context, IntoElement, TitlebarOptions, Window, WindowBounds,
    WindowOptions, px, size,
};

/// Which implementation a pane shows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Hand,
    Dsl,
}

const HAND_SRC: &str = include_str!("hand_written.rs");
const DSL_SRC: &str = include_str!("dsl.rs");

struct Showcase {
    items: Vec<data::Item>,
    fps: f64,
    fps_frames: u32,
    fps_window_start: Instant,
    /// While the user hovers a pane's "view code" button, that pane shows its
    /// source instead of the rendered UI.
    code_view: Option<Side>,
}

impl Showcase {
    fn new() -> Self {
        Self {
            items: data::sample_items(24),
            fps: 0.0,
            fps_frames: 0,
            fps_window_start: Instant::now(),
            code_view: None,
        }
    }
}

fn stat(label: &str, value: String) -> impl IntoElement {
    ui! {
        col(gap_2, min_w_140) {
            text(label.to_string(), text_xs, text_8a857c)
            text(value, text_lg, semibold, text_f5f0e8)
        }
    }
}

/// The pane's "hover to view code" button. Hovering swaps the pane body for
/// the implementation's source; leaving swaps the live UI back.
fn code_button(side: Side, active: bool, cx: &mut Context<Showcase>) -> impl IntoElement {
    let listener = cx.listener(move |this: &mut Showcase, hovered: &bool, _window, cx| {
        if *hovered {
            this.code_view = Some(side);
        } else if this.code_view == Some(side) {
            this.code_view = None;
        }
        cx.notify();
    });
    let id: gpui::SharedString = match side {
        Side::Hand => "code-btn-hand".into(),
        Side::Dsl => "code-btn-dsl".into(),
    };
    ui! {
        row(
            id = id,
            on_hover = listener,
            cursor_pointer,
            px_8, py_2, rounded_md, border, border_2d2b27,
            bg = if active { gpui::rgb(0x2d2b27) } else { gpui::rgb(0x24211d) }
        ) {
            text(
                if active { "</> showing code" } else { "</> hover to view code" },
                text_xs,
                text_c8c4bc
            )
        }
    }
}

/// The implementation source as a monospace listing.
fn code_listing(source: &'static str) -> impl IntoElement {
    ui! {
        col(p_12, bg_141210, rounded_md, border, border_2d2b27, font_family = "Consolas") {
            for line in source.lines() {
                text(
                    if line.is_empty() { " ".to_string() } else { line.to_string() },
                    text_xs,
                    text_c8c4bc
                )
            }
        }
    }
}

fn pane(title: &str, loc: usize, body: AnyElement, code_btn: AnyElement) -> impl IntoElement {
    ui! {
        col(flex_1, gap_8, min_h_0) {
            row(items_center, justify_between, px_4) {
                text(title.to_string(), text_sm, semibold, text_f5f0e8)
                row(gap_8, items_center) {
                    text(format!("{loc} lines of code"), text_xs, text_8a857c)
                    { code_btn }
                }
            }
            scroll(flex_1) { { body } }
        }
    }
}

impl Render for Showcase {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.fps_frames += 1;
        let elapsed = self.fps_window_start.elapsed().as_secs_f64();
        if elapsed >= 1.0 {
            self.fps = self.fps_frames as f64 / elapsed;
            self.fps_frames = 0;
            self.fps_window_start = Instant::now();
        }

        // Keep rendering every frame so the fps readout is live.
        window.request_animation_frame();

        let hand_loc = HAND_SRC.lines().count();
        let dsl_loc = DSL_SRC.lines().count();
        let code_saved = (1.0 - dsl_loc as f64 / hand_loc as f64) * 100.0;

        // Hovering a pane's code button swaps that pane's body for its source.
        let hand_body = if self.code_view == Some(Side::Hand) {
            code_listing(HAND_SRC).into_any_element()
        } else {
            hand_written::view(&self.items).into_any_element()
        };
        let dsl_body = if self.code_view == Some(Side::Dsl) {
            code_listing(DSL_SRC).into_any_element()
        } else {
            dsl::view(&self.items).into_any_element()
        };
        let hand_btn =
            code_button(Side::Hand, self.code_view == Some(Side::Hand), cx).into_any_element();
        let dsl_btn =
            code_button(Side::Dsl, self.code_view == Some(Side::Dsl), cx).into_any_element();

        ui! {
            col(size_full, bg_141210, p_16, gap_12) {
                row(gap_24, items_center, px_16, py_12, bg_1c1a17, rounded_lg, border, border_2d2b27) {
                    { stat("source size", format!("{hand_loc} vs {dsl_loc} lines")) }
                    { stat("code saved by ui!", format!("−{code_saved:.0}%")) }
                    { stat("fps", format!("{:.0}", self.fps)) }
                }
                row(gap_12, flex_1, min_h_0) {
                    { pane("hand-written GPUI", hand_loc, hand_body, hand_btn) }
                    { pane("ui! macro", dsl_loc, dsl_body, dsl_btn) }
                }
            }
        }
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.), px(860.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("ui! showcase — hand-written vs macro".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Showcase::new()),
        )
        .expect("failed to open window");
        cx.activate(true);
    });
}
