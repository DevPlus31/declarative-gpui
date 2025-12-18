mod declarative_ui;

use gpui::{
    App, Application, Context, FontWeight, Global, Menu, MenuItem, SystemMenuType, Window,
    WindowOptions, actions, div, prelude::*, rgb, AnyElement, InteractiveElement, ParentElement, Styled,
};
use declarative_ui::{Color as DColor, Element as DElement, Style as DStyle};

struct AppView;

impl AppView {
    fn render_element(&self, el: DElement, cx: &mut Context<Self>) -> AnyElement {
        let mut gpui_el = div();

        // Apply styles
        for style in el.styles {
            match style {
                DStyle::Flex => gpui_el = gpui_el.flex(),
                DStyle::FlexCol => gpui_el = gpui_el.flex_col(),
                DStyle::FlexRow => gpui_el = gpui_el.flex_row(),
                DStyle::JustifyCenter => gpui_el = gpui_el.justify_center(),
                DStyle::JustifyBetween => gpui_el = gpui_el.justify_between(),
                DStyle::ItemsCenter => gpui_el = gpui_el.items_center(),
                DStyle::Gap(p) => gpui_el = gpui_el.gap(gpui::px(p)),
                DStyle::Padding(p) => gpui_el = gpui_el.p(gpui::px(p)),
                DStyle::Width(w) => gpui_el = gpui_el.w(gpui::px(w)),
                DStyle::Height(h) => gpui_el = gpui_el.h(gpui::px(h)),
                DStyle::Size(s) => gpui_el = gpui_el.size(gpui::px(s)),
                DStyle::SizeFull => gpui_el = gpui_el.size_full(),
                DStyle::Background(color) => {
                    gpui_el = gpui_el.bg(self.convert_color(color));
                }
                DStyle::TextColor(color) => {
                    gpui_el = gpui_el.text_color(self.convert_color(color));
                }
                DStyle::TextSize(s) => {
                    gpui_el = gpui_el.text_size(gpui::px(s));
                }
                DStyle::FontWeightBold => {
                    gpui_el = gpui_el.font_weight(FontWeight::BOLD);
                }
            }
        }

        if let Some(id) = el.id {
            // Use static strings for known IDs to avoid leaking memory, 
            // otherwise leak the string for this demo.
            let leaked_id: &'static str = match id.as_str() {
                "plus" => "plus",
                "minus" => "minus",
                _ => Box::leak(id.clone().into_boxed_str()),
            };
            let mut stateful_el = gpui_el.id(leaked_id);
            
            if let Some(on_click) = el.on_click {
                stateful_el = stateful_el.on_click(move |_, _, cx| {
                    on_click(cx);
                });
            }

            if id == "plus" || id == "minus" {
                stateful_el = stateful_el.hover(|s| s.bg(rgb(0x357abd))).cursor_pointer();
            }

            // Add children
            for child in el.children {
                stateful_el = stateful_el.child(self.render_element(child, cx));
            }

            // Add content if any
            if let Some(content) = el.content {
                stateful_el = stateful_el.child(content);
            }

            stateful_el.into_any_element()
        } else {
            // Add children
            for child in el.children {
                gpui_el = gpui_el.child(self.render_element(child, cx));
            }

            // Add content if any
            if let Some(content) = el.content {
                gpui_el = gpui_el.child(content);
            }

            gpui_el.into_any_element()
        }
    }

    fn convert_color(&self, color: DColor) -> gpui::Hsla {
        match color {
            DColor::Hex(h) => rgb(h).into(),
            DColor::Name("red") => gpui::red().into(),
            DColor::Name("green") => gpui::green().into(),
            DColor::Name("blue") => rgb(0x4a90e2).into(),
            DColor::Rgb(r, g, b) => gpui::rgb((r as u32) << 16 | (g as u32) << 8 | (b as u32)).into(),
            _ => rgb(0x000000).into(),
        }
    }
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_state = cx.global::<AppState>();
        let name = "Developer";

        // Example of declarative UI using the ui! macro
        let _ui_root = ui! {
            div["flex col gap-3 bg-gray p-4"] {
                text[format!("Hello, {}!", name)]
                row[2.0] {
                    box[8.0, DColor::Name("red")]
                }
            }
        };

        // Example of declarative UI using the jsx! macro
        let _jsx_root = jsx! {
            <div class={"flex col gap-4 bg-gray p-6"}> {
                <text>{ format!("Welcome, {}!", name) }</text>
                <row gap={10.0}> {
                    <box size={20.0} color={DColor::Hex(0x0000ff)} />
                    <box size={20.0} color={DColor::Hex(0xffff00)} />
                } </row>
            } </div>
        };

        // New syntax using ui! macro
        let root = ui! {
            div["flex col bg-dark size-full text-gray"] {
                // Header
                div["flex items-center justify-between p-4 bg-light-gray"] {
                    div["text-xl bold"] { text["GPUI Showcase"] }
                    div["flex gap-4"] { text[format!("View: {:?}", app_state.view_mode)] }
                }
                // Main Content
                div["flex-1 p-8"] {
                    div["flex col gap-6"] {
                        // Counter Example
                        div["flex col gap-2"] {
                            div["text-lg"] { text["Counter Example"] }
                            div["flex items-center gap-4"] {
                                div["bg-blue text-white p-4", "minus", {
                                    move |cx| {
                                        let cx = cx.downcast_mut::<gpui::App>().unwrap();
                                        cx.dispatch_action(&Decrement);
                                    }
                                }] { text["-"] }
                                div["text-2xl"] { text[format!("{}", app_state.count)] }
                                div["bg-blue text-white p-4", "plus", {
                                    move |cx| {
                                        let cx = cx.downcast_mut::<gpui::App>().unwrap();
                                        cx.dispatch_action(&Increment);
                                    }
                                }] { text["+"] }
                            }
                        }
                        // Instructions
                        div["flex col gap-2"] {
                            div["text-lg"] { text["Instructions"] }
                            div["text-sm text-dim"] { text["1. Use the buttons above to change the counter."] }
                            div["text-sm text-dim"] { text["2. Check the system menu to toggle View Mode (List/Grid)."] }
                            div["text-sm text-dim"] { text["3. Notice how the UI updates based on global state."] }
                        }
                    }
                }
                // Footer
                div["p-2 bg-footer text-xs text-dim"] {
                    text["Built with GPUI"]
                }
            }
        };

        self.render_element(root, cx)

        /*
        // ORIGINAL IMPERATIVE CODE KEPT FOR REFERENCE
        div()
            .flex()
            .flex_col()
            .bg(rgb(0x1e1e1e))
            .size_full()
            .text_color(rgb(0xcccccc))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p_4()
                    .bg(rgb(0x333333))
                    .child(div().text_xl().font_weight(FontWeight::BOLD).child("GPUI Showcase"))
                    .child(
                        div()
                            .flex()
                            .gap_4()
                            .child(format!("View: {:?}", app_state.view_mode))
                    )
            )
            .child(
                div()
                    .flex_1()
                    .p_8()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_6()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(div().text_lg().child("Counter Example"))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_4()
                                            .child(
                                                div()
                                                    .px_4()
                                                    .py_2()
                                                    .bg(rgb(0x4a90e2))
                                                    .text_color(rgb(0xffffff))
                                                    .hover(|s| s.bg(rgb(0x357abd)))
                                                    .cursor_pointer()
                                                    .id("minus")
                                                    .on_click(|_, _, cx| {
                                                        cx.update_global::<AppState, _>(|state, _| {
                                                            state.count -= 1;
                                                        });
                                                    })
                                                    .child("-")
                                            )
                                            .child(div().text_2xl().child(format!("{}", app_state.count)))
                                            .child(
                                                div()
                                                    .px_4()
                                                    .py_2()
                                                    .bg(rgb(0x4a90e2))
                                                    .text_color(rgb(0xffffff))
                                                    .hover(|s| s.bg(rgb(0x357abd)))
                                                    .cursor_pointer()
                                                    .id("plus")
                                                    .on_click(|_, _, cx| {
                                                        cx.update_global::<AppState, _>(|state, _| {
                                                            state.count += 1;
                                                        });
                                                    })
                                                    .child("+")
                                            )
                                    )
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(div().text_lg().child("Instructions"))
                                    .child(div().text_sm().text_color(rgb(0x888888)).child("1. Use the buttons above to change the counter."))
                                    .child(div().text_sm().text_color(rgb(0x888888)).child("2. Check the system menu to toggle View Mode (List/Grid)."))
                                    .child(div().text_sm().text_color(rgb(0x888888)).child("3. Notice how the UI updates based on global state."))
                            )
                    )
            )
            .child(
                div()
                    .p_2()
                    .bg(rgb(0x252525))
                    .text_xs()
                    .text_color(rgb(0x666666))
                    .child("Built with GPUI")
            )
        */
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.set_global(AppState::new());

        // Bring the menu bar to the foreground (so you can see the menu bar)
        cx.activate(true);
        // Register actions
        cx.on_action(quit);
        cx.on_action(toggle_list_mode);
        cx.on_action(toggle_grid_mode);
        cx.on_action(increment);
        cx.on_action(decrement);
        
        // Add menu items
        set_app_menus(cx);
        cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| AppView))
            .unwrap();
    });
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum ViewMode {
    List,
    Grid,
}

struct AppState {
    view_mode: ViewMode,
    count: i32,
}

impl AppState {
    fn new() -> Self {
        Self {
            view_mode: ViewMode::List,
            count: 0,
        }
    }
}

impl Global for AppState {}

fn set_app_menus(cx: &mut App) {
    let app_state = cx.global::<AppState>();
    let list_label = if app_state.view_mode == ViewMode::List { "✓ List Mode" } else { "List Mode" };
    let grid_label = if app_state.view_mode == ViewMode::Grid { "✓ Grid Mode" } else { "Grid Mode" };

    cx.set_menus(vec![
        Menu {
            name: "GPUI Example".into(),
            items: vec![
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Quit", Quit),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action(list_label, ToggleListMode),
                MenuItem::action(grid_label, ToggleGridMode),
            ],
        },
    ]);
}

// Associate actions using the `actions!` macro
actions!(gpuiex, [Quit, ToggleListMode, ToggleGridMode, Increment, Decrement]);

// Define the quit function that is registered with the App
fn quit(_: &Quit, cx: &mut App) {
    println!("Gracefully quitting the application . . .");
    cx.quit();
}

fn increment(_: &Increment, cx: &mut App) {
    cx.update_global::<AppState, _>(|state, _| {
        state.count += 1;
    });
}

fn decrement(_: &Decrement, cx: &mut App) {
    cx.update_global::<AppState, _>(|state, _| {
        state.count -= 1;
    });
}

fn toggle_list_mode(_: &ToggleListMode, cx: &mut App) {
    cx.update_global::<AppState, _>(|state, _| {
        state.view_mode = ViewMode::List;
    });
    set_app_menus(cx);
}

fn toggle_grid_mode(_: &ToggleGridMode, cx: &mut App) {
    cx.update_global::<AppState, _>(|state, _| {
        state.view_mode = ViewMode::Grid;
    });
    set_app_menus(cx);
}