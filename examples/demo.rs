use declarative_ui::*;
use gpui::*;

struct AppState {
    count: i32,
}

impl Global for AppState {}

struct DemoApp;

impl Render for DemoApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let count = cx.global::<AppState>().count;

        ui! {
            col(size_full bg_0b0f14 p_12 items_center justify_center) {
                row(items_center gap_8) {
                    text("Declarative GPUI Demo" text_2xl font_bold text_f2f0e9)
                }

                col(mt_12 p_16 rounded_xl bg_1e1e1e border border_x_1 border_y_1 shadow_md items_center gap_8) {
                    text("Counter" text_lg text_cccccc)

                    row(items_center gap_12 mt_4) {
                        div(
                            px_12 py_6 rounded_md bg_333333 text_f2f0e9 cursor_pointer
                            on_mouse_down=|_ev, window, cx| {
                                cx.update_global::<AppState, _>(|state, _app_cx| {
                                    state.count -= 1;
                                });
                                window.defer(cx, |_window, cx| cx.refresh());
                            }
                        ) {
                            text("- 1" font_bold)
                        }

                        text((format!("{}", count)) text_3xl font_bold text_4a90e2 w_64 text_center)

                        div(
                            px_12 py_6 rounded_md bg_4a90e2 text_f2f0e9 cursor_pointer
                            on_mouse_down=|_ev, window, cx| {
                                cx.update_global::<AppState, _>(|state, _app_cx| {
                                    state.count += 1;
                                });
                                window.defer(cx, |_window, cx| cx.refresh());
                            }
                        ) {
                            text("+ 1" font_bold)
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
