# Experimental declarative UI macros for GPUI

This project demonstrates a declarative UI macro system for the [GPUI](https://gpui.rs/) framework. It transforms verbose, imperative UI builder code into a clean, readable, and highly maintainable syntax.

## Features

- **`ui!` Macro**: A nested block syntax for building UI hierarchies.
- **`jsx!` Macro**: A familiar JSX-inspired syntax for React developers.
- **Tailwind-like Styling**: Support for string-based utility classes (e.g., `"flex col gap-4 bg-dark"`).
- **Type-Safe**: Built on top of Rust's powerful macro and type system.
- **Interactive**: Supports event handlers and dynamic state updates.

## Example

### Declarative Syntax (`ui!`)

```rust
let root = ui! {
    div["flex col bg-dark size-full text-gray"] {
        div["flex items-center justify-between p-4 bg-light-gray"] {
            div["text-xl bold"] { text["GPUI Showcase"] }
        }
        div["flex-1 p-8"] {
            div["flex col gap-6"] {
                div["bg-blue text-white p-4", "plus", {
                    move |cx| {
                        let cx = cx.downcast_mut::<gpui::App>().unwrap();
                        cx.dispatch_action(&Increment);
                    }
                }] { text["+"] }
            }
        }
    }
};
```

### JSX-like Syntax (`jsx!`)

```rust
let welcome = jsx! {
    <div class={"flex col gap-4 bg-gray p-6"}> {
        <text>{ format!("Welcome, {}!", name) }</text>
        <row gap={10.0}> {
            <box size={20.0} color={DColor::Hex(0x0000ff)} />
        } </row>
    } </div>
};
```

## Running the Project

Ensure you have Rust and the necessary GPUI dependencies installed.

```bash
cargo run
```

## Implementation Details

- `src/declarative_ui.rs`: Contains the core macro definitions, style parser, and element structures.
- `src/main.rs`: Demonstrates how to bridge the declarative elements into native GPUI elements and manage application state.
