# declarative-gpui

A declarative `ui!` macro for building [GPUI](https://www.gpui.rs/) element
trees with Tailwind-style tokens, real Rust control flow, and **zero runtime
overhead** — every token compiles directly to GPUI builder calls, and colors are
converted to their final `Hsla` values at compile time (faster than typical
hand-written GPUI).

```rust
use declarative_gpui::ui;
use gpui::prelude::*;

fn render(items: &[Item], dark: bool) -> impl IntoElement {
    ui! {
        col(gap_4 p_16 bg_1c1a17 rounded_lg shadow_md) {
            row(items_center justify_between) {
                text("Inbox" text_lg semibold text_f5f0e8)
                if dark { text("dark") } else { text("light") }
            }
            for item in items {
                row(gap_2 px_8 py_2 on_hover = |_, _, _| {}) {
                    text(&item.title text_sm)
                }
            }
        }
    }
}
```

## Code → pixels

The 47-line [`examples/src/dsl.rs`](examples/src/dsl.rs) renders the card
below — pixel-identical to the 165-line hand-written
[`examples/src/hand_written.rs`](examples/src/hand_written.rs), **72% less
code**:

<img src="https://raw.githubusercontent.com/DevPlus31/declarative-gpui/main/examples/screenshots/card.png" width="480" alt="Team Inbox card rendered by the 47-line dsl.rs">

Both implementations run side by side in the showcase app
(`cargo run -p declarative-gpui-examples --release`); hover a pane's
`</> view code` button to see the source that renders it:

![side-by-side showcase: hand-written GPUI (left) vs ui! (right)](https://raw.githubusercontent.com/DevPlus31/declarative-gpui/main/examples/screenshots/showcase.png)

**Zero runtime overhead** — the macro expands to the same builder calls
you'd write by hand, so there is nothing extra at runtime. Details in
[examples/README.md](examples/README.md).

## Elements

| Element | Expands to |
|---|---|
| `div(...)` | `gpui::div()` |
| `row(...)` | `gpui::div().flex().flex_row()` |
| `col(...)`, `card(...)` | `gpui::div().flex().flex_col()` |
| `center(...)` | `div().flex().flex_row().items_center().justify_center()` |
| `scroll(...)` | stateful div with a stable auto ID + `.overflow_y_scroll()` |
| `list(id, count, render, ...)` | `gpui::uniform_list(...)` |
| `text(content, ...)`, `text_raw`, `label` | the content, wrapped in a styled div only when style args are present |
| any other name | called as a constructor: `badge(...)` → `badge()` |

Children go in braces; `if`/`else`, `for`, and `match` (with guards) work
anywhere a child can appear and expand to real Rust control flow. `{ expr }`
escapes to an arbitrary Rust expression.

## Element arguments

Three forms, in any mix (commas between args are optional but **recommended**
— without them adjacent expression args can merge and misparse):

- **Style tokens** — `px_8`, `bg_1c1a17`, `rounded_lg` (reference below).
- **`key = value`** — any builder method: `id = "main"`,
  `w = gpui::rems(2.0)`, `font_family = "Zed Mono"`, `on_click = |e, w, cx| …`.
  Mouse handlers (`on_mouse_down` etc.) default to `MouseButton::Left`.
- **Call-style** — `hover(|s| s.opacity(0.8))`, `when(cond, |el| …)` become
  `.hover(…)`, `.when(…)`.

Elements using `on_hover`, `on_drag`, `tooltip`, or `hoverable_tooltip`
automatically get a stable `.id()` (from `file!:line!:column!`) so they are
`Stateful<Div>`. An explicit `id = …` is folded into the constructor instead.
Argument order matters for type-changing methods: put `id = …` before
`on_click` / `overflow_x_scroll` on plain `div`s.

## Style token reference

Numeric conventions (deliberately different from GPUI's rem scale — rem
values remain available via `key = value`):

| Form | Meaning | Example |
|---|---|---|
| `_N` | N **pixels** | `w_240` → `.w(px(240.))` |
| `_XpY` | decimal pixels | `p_2p5` → 2.5px |
| `_neg_N` | negative (where GPUI allows) | `m_neg_8` → −8px |
| `_auto`, `_full`, `_px` | GPUI's auto / 100% / 1px methods | `mx_auto`, `min_w_full`, `w_px` |
| fractions | relative size | `w_1_2` → 50%, `h_2_3`, `flex_basis_1_4` |

- **Layout** — `flex` `grid` `block` `hidden`, `flex_row`/`flex_col`
  (+`_reverse`), wrap variants, `flex_1` `flex_auto` `flex_initial`
  `flex_none`, `flex_grow(_0/_1)`, `flex_shrink(_0/_1)`, `justify_*`
  (incl. `justify_evenly`), `items_*` (incl. `items_stretch`), `self_*`,
  `content_*`, `aspect_square`.
- **Spacing** — `p`/`px`/`py`/`pt`/`pb`/`pl`/`pr`, `m*` (margins allow
  `_auto`/`_neg_`), `gap`/`gap_x`/`gap_y`.
- **Sizing** — `w` `h` `size` `min_w` `min_h` `min_size` `max_w` `max_h`
  `max_size` with all numeric forms.
- **Position** — `relative` `absolute`, `top`/`bottom`/`left`/`right`/`inset`
  (+`_auto`/`_neg_`).
- **Colors** — `bg_`, `text_`, `border_`, `text_bg_`, `text_decoration_` +
  hex (`1c1a17`, `abc`, `1c1a17cc` with alpha, `abcd`) or a named color
  (`black white gray red green blue yellow cyan magenta orange purple`).
  Emitted as **precomputed `Hsla` literals** — zero color math at runtime.
  Precedence note: a 3/6-digit all-numeric suffix is a *color* (`text_112` =
  `#112`); anything else numeric is a *size* (`text_16` = 16px).
- **Typography** — `text_xs…text_3xl`, `text_N` (px size), weights
  (`thin…black`, with or without `font_` prefix), `italic`, `underline`,
  `line_through`, `text_decoration_{0,1,2,4,8,solid,wavy,none}` and
  `text_decoration_<color>`, `truncate`, `text_ellipsis(_start/_middle)`,
  `line_clamp_N`, `whitespace_nowrap/normal`, `text_left/center/right`.
- **Borders & radius** — `border(_x/_y/_t/_b/_l/_r)` (bare = 1px, `_N` = Npx,
  `_0p5` = 0.5px), `border_dashed`, `border_<color>`; full `rounded` family
  (`rounded_lg`, `rounded_tl_2xl`, `rounded_8`, …).
- **Effects & misc** — `shadow(_2xs…_2xl,_none)`, `opacity_N` (0–100),
  `cursor_*`, `visible`/`invisible`, `overflow_hidden(_x/_y)`,
  `overflow_scroll(_x/_y)` (needs a stateful element), `scrollbar_width_N`,
  `debug`, `debug_below`.
- **Grid** — `grid_cols_N`/`grid_rows_N`, `grid_cols_min_content_N`/
  `_max_content_N`, `col_span_N`/`row_span_N` (+`_full`),
  `col_start`/`col_end`/`row_start`/`row_end` (+`_auto`, `_neg_N`).

Unknown tokens are **compile errors** ("Unknown style token"), never silently
ignored.

## Zero runtime overhead

The generated code is what a careful GPUI author would write, or better:

- style tokens are direct method calls — no runtime string matching;
- colors are compile-time `Hsla` literals — no `rgb()` unpacking or RGB→HSL
  conversion per frame;
- unstyled `text(...)` emits the bare content — no wrapper div layout node;
- `list` moves its render closure — no `Arc`, no per-frame allocation;
- every element yields its concrete type — no `AnyElement` boxing.

These guarantees are pinned by tests in `core`.

## GPUI compatibility

Token tables are verified against GPUI at zed rev `bb48a42` (2026-07-02).
The `integration` crate compiles every DSL feature against that exact rev —
if GPUI renames or removes an API the macro targets, that crate stops
compiling. Bump the rev in `integration/Cargo.toml` to check newer GPUI.

## Workspace layout & development

```
.             proc-macro shim (`ui!` entry point)
core/         declarative-gpui-core — parser, emitter, token tables, tests
integration/  compile-surface tests against real GPUI (excluded from default builds)
examples/     side-by-side showcase: hand-written GPUI vs ui! (see examples/README.md)
```

```sh
cargo test                                        # unit + expansion tests (fast)
cargo test -p declarative-gpui-core dump_expansions -- --ignored --nocapture    # golden expansion dump
cargo test -p declarative-gpui-integration      # real-GPUI compile check (slow first build)
cargo run -p declarative-gpui-examples --release   # side-by-side showcase
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
