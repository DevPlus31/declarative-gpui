use declarative_ui::*;
use gpui::*;

struct AppState {
    count: i32,
}

impl Global for AppState {}

struct DemoApp;

impl Render for DemoApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        cx.observe_global::<AppState>(|_, cx| cx.notify()).detach();
        let count = cx.global::<AppState>().count;

        let dynamic_bg = if count > 5 { "bg-2e4a23 border-4a7a33" } else { "bg-1e1e1e border-333333" };

        ui! {
            col(size_full bg_0b0f14 p_12 items_center) {
                row(items_center gap_8 mt_12) {
                    text("Declarative GPUI Demo" text_2xl font_bold text_f2f0e9)
                }

                // Showcasing dynamic string styles using `class=(...)`
                // Showcasing block expressions using `bg=(...)`
                col(
                    mt_12 p_16 rounded_xl border border_x_1 border_y_1 shadow_md items_center gap_8
                    class=dynamic_bg
                ) {
                    text("Counter" text_lg text_cccccc)

                    row(items_center gap_12 mt_4) {
                        div(
                            px_12 py_6 rounded_md bg_333333 text_f2f0e9 cursor_pointer
                            on_mouse_down=|_ev, _window, cx| {
                                cx.update_global::<AppState, _>(|state, _app_cx| {
                                    state.count -= 1;
                                });
                            }
                        ) {
                            text("- 1" font_bold)
                        }

                        text(format!("{}", count) text_3xl font_bold text_4a90e2 w_64 text_center)

                        div(
                            px_12 py_6 rounded_md bg_4a90e2 text_f2f0e9 cursor_pointer
                            on_mouse_down=|_ev, _window, cx| {
                                cx.update_global::<AppState, _>(|state, _app_cx| {
                                    state.count += 1;
                                });
                            }
                        ) {
                            text("+ 1" font_bold)
                        }
                    }

                    // Showcasing native match statement
                    match count.cmp(&0) {
                        std::cmp::Ordering::Less => {
                            text("Warning: Negative Count!" text_sm text_red mt_4)
                        }
                        std::cmp::Ordering::Greater if count > 10 => {
                            text("High Count Achieved!" text_sm text_green mt_4)
                        }
                        std::cmp::Ordering::Greater => {
                            text("Keep going..." text_sm text_gray mt_4)
                        }
                        std::cmp::Ordering::Equal => {
                            text("Started!" text_sm text_gray mt_4)
                        }
                    }
                }

                // Showcasing `scroll` keyword, native `for` loop, and arbitrary builder values like `w(px(300.))`
                scroll(flex_1 mt_12 bg_161616 rounded_lg p_8 border border_333333 w(px(300.))) {
                    for i in 0..count.abs() {
                        row(p_8 border_b_1 border_222222 justify_between items_center) {
                            text(format!("Generated Item #{}", i + 1) text_sm text_aaaaaa)
                            if i % 2 == 0 {
                                div(size_8 rounded_full bg_4a90e2)
                            } else {
                                div(size_8 rounded_full bg_e24a4a)
                            }
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.set_global(AppState { count: 0 });

        let bounds = Bounds::centered(None, size(px(400.0), px(400.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_cx| DemoApp),
        )
        .unwrap();
    });
}
