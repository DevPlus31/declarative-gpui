# declarative-ui

A small declarative DSL for GPUI that expands to the native builder API. It's built as a **procedural macro**, giving you precise compile-time errors, native Rust block expression support, and seamless IDE integration.

## Installation

Add `declarative-ui` to your `Cargo.toml`:

```toml
[dependencies]
declarative-ui = { git = "https://github.com/DevPlus31/declarative-gpui" }
```
*(Note: Replace with the actual path or repository link of your crate)*

## Quick Example

```rust
use declarative_ui::*;

ui! {
    col(size_full bg_0b0f14 p_12) {
        row(items_center justify_between) {
            text("Title" text_lg bold text_f2f0e9)
        }

        // The `scroll` node natively applies an ID and `overflow_y_scroll`
        scroll(flex_1 w(px(300.0)) mt_12) {
            for item in (items.iter()) {
                row(p_8 border_b_1 border_333333) {
                    text((item.name.clone()) text_sm text_aaaaaa)
                }
            }
        }
    }
}
```

## Advanced Features

### 1. Compile-Time Token Validation
If you make a typo with your style tokens (e.g. `text_grene`), the procedural macro will throw a compilation error directly in your IDE! No more silent runtime fallbacks.

### 2. Arbitrary Values & Native Method Calls
You can now call native GPUI builder methods directly inside the property list! This means you are never restricted by the predefined `declarative_ui` tokens.
```rust
ui! {
    div( w(px(300.)) h(rems(10.)) bg(rgb(0xff00ff)) ) {
        text("Direct GPUI Method Calls!")
    }
}
```

### 3. Native `match` Statements
You can write native `match` blocks directly inside the `ui!` tree, which is incredibly useful for rendering different states:
```rust
ui! {
    col {
        match app_state {
            State::Loading => { text("Loading...") }
            State::Loaded(data) => { text(data text_green) }
            State::Error => { text("Failed!" text_red) }
        }
    }
}
```

### 4. Dynamic Classes (`class=`)
You can pass a string of dynamic class names generated at runtime using the `class=(expr)` attribute:
```rust
let bg_color = if active { "bg-2e4a23" } else { "bg-1e1e1e" };

ui! {
    div(p_12 rounded_md class=bg_color) {
        text("Dynamic Styling" font_bold)
    }
}
```

### 5. Native Block Expressions
Thanks to the procedural macro, you can write native block expressions directly as arguments without any confusing parser issues:
```rust
ui! {
    div(
        bg=(if active { rgb(0x4a90e2) } else { rgb(0x333333) })
        rounded_lg p_12
    ) {
        text("Dynamic BG" text_white)
    }
}
```

### 6. Scroll Containers
A frequent GPUI pattern is needing a container to scroll, which requires assigning a stable `.id()` to a `Stateful<Div>`. The macro handles this seamlessly with the `scroll` keyword:
```rust
ui! {
    scroll(flex_1 p_8) {
        // Automatically injects a stable unique ID based on file/line and applies overflow_y_scroll()
        for i in 0..100 {
            text(format!("Item {}", i) text_base)
        }
    }
}
```

### 7. Control Flow (`if` and `for`)
Native Rust logic can be interwoven flawlessly:
```rust
ui! {
    col(gap_12) {
        // Native if statements
        if count < 0 {
            text("Warning: Negative!" text_red)
        } else if count > 10 {
            text("High Count!" text_green)
        } else {
            text("Normal" text_gray)
        }

        // Native for loops
        for item in 0..count {
            row(p_8) { text("Item" text_sm) }
        }
    }
}
```

## DSL Overview

Nodes:

```rust
row(gap_12 items_center) {
    text("Hello" text_lg bold)
    { child_component } // Mix in raw GPUI Elements using braces
    MyCustomComponent(label="Builder Props work too!") // Custom builder component
}
```

Text:

```rust
text("Literal" text_sm)
text(format!("{}", count) text_sm) // Dynamic text wrapping
```

## Colors

`bg_` / `text_` / `border_` accept:

- CSS-like named colors: `black`, `white`, `gray`, `red`, `green`, `blue`, `yellow`, `cyan`, `magenta`, `orange`, `purple`
- Hex tokens: `bg_0b0f14`, `text_f2f0e9`, `border_ff00aa`, `bg_0ff`

## Events

Sugar forms:

- `on_click=handler`
- `on_click=|_ev, window, cx| { ... }`
- `on_mouse_down=handler`
- `on_mouse_down=|_ev, window, cx| { ... }`
- `on_action=handler`
- `on_action=<ActionType>|action, window, cx| { ... }`
- `on_drop=handler`
- `on_drop=<ValueType>|value, window, cx| { ... }`
