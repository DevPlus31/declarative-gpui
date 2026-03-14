# declarative-ui

A small declarative DSL for GPUI that expands to the native builder API.

## Quick Example

```rust
ui! {
    col(size_full bg_0b0f14) {
        row(items_center justify_between p_12) {
            text("Title" text_lg bold)
            { right_panel }
        }

        list(
            id="items"
            count=(items.len())
            render=|ix| {
                let item = &items[ix];
                text((item.name.clone()) text_sm)
            }
            gap_8
            p_12
        )
    }
}
```

## DSL Overview

Nodes:

```
row(gap_12 items_center) {
    text("Hello" text_lg bold)
    { child_component }
}
```

Text:

```
text(title text_sm)
text("Literal" text_sm)
text((format!("{}", count)) text_sm)
text_raw("No wrapper")
label("Alias for text_raw")
```

Props:

```
input(
    placeholder=("Search...")
    value=(query.clone())
)
```

Conditions:

```
if loading {
    text("Loading..." text_sm)
} else {
    { content_panel }
}
```

Loops:

```
for item in items {
    row { text((item.name.clone()) text_sm) }
}

for item in (items.iter()) {
    row { text((item.name.clone()) text_sm) }
}
```

## Colors

`bg_` / `text_` / `border_` accept:

- CSS-like named colors: `black`, `white`, `gray`, `red`, `green`, `blue`, `yellow`, `cyan`, `magenta`, `orange`, `purple`
- Hex tokens: `bg_0b0f14`, `text_f2f0e9`, `border_ff00aa`, `bg_0ff`

## Events

Sugar forms:

- `on_click=handler`
- `on_click=|ev, window, cx| { ... }`
- `on_mouse_down=handler`
- `on_mouse_down=|ev, window, cx| { ... }`
- `on_action=handler`
- `on_action=<ActionType>|action, window, cx| { ... }`
- `on_drop=handler`
- `on_drop=<ValueType>|value, window, cx| { ... }`

More events are supported directly as props, for example:

```
row(on_mouse_move=handle_move on_key_down=handle_key) { ... }
```

## List DSL

```
list(
    id="items"
    count=(items.len())
    render=|ix| { ... }
    gap_8
    p_12
)
```

## Style Tokens

Common layout:

- `flex`, `block`, `grid`, `hidden`
- `row`, `col`, `flex_wrap`, `flex_nowrap`
- `justify_start`, `justify_between`, `items_center`
- `gap_8`, `p_12`, `px_6`, `py_4`, `m_8`, `mx_4`
- `w_320`, `h_64`, `size_48`

Text:

- `text_xs`, `text_sm`, `text_base`, `text_lg`, `text_xl`, `text_2xl`, `text_3xl`
- `italic`, `underline`, `line_through`, `truncate`
- `text_left`, `text_center`, `text_right`

Borders and radius:

- `border`, `border_2`, `border_t_1`, `border_x_1`
- `rounded`, `rounded_md`, `rounded_full`, `rounded_4`

Shadows:

- `shadow`, `shadow_sm`, `shadow_md`, `shadow_lg`, `shadow_xl`

Overflow:

- `overflow_hidden`, `overflow_x_hidden`, `overflow_y_hidden`

## Notes

- Use parentheses for complex expressions in `text(...)` and in prop assignments.
- `if` supports `if flag { ... }` or `if (a && b) { ... }`.
- `for` supports `for item in items { ... }` or `for item in (items.iter()) { ... }`.
