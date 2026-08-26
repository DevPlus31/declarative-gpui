//! Parsing and expansion logic for the `ui!` GPUI macro. The proc-macro crate
//! (`declarative-gpui`) is a thin shim over [`ui_impl`]; everything here
//! works on `proc_macro2` streams so it can be tested and benchmarked as a
//! normal library.

use proc_macro2::TokenStream as TokenStream2;
use proc_macro2::{Delimiter, Group, Punct, Spacing, TokenTree};
use quote::{ToTokens, TokenStreamExt, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, Result, Token, braced, parenthesized};

/// Expand the contents of a `ui! { ... }` invocation. Parse errors are
/// returned as `compile_error!` invocations, never panics.
pub fn ui_impl(input: TokenStream2) -> TokenStream2 {
    let ui_ast = match syn::parse2::<Ui>(input) {
        Ok(ast) => ast,
        Err(err) => return err.to_compile_error(),
    };

    let mut out = TokenStream2::new();
    emit_nodes(&mut out, &ui_ast.nodes, Ctx::TopLevel);
    out
}

/// Expand the contents of a `ui_expand!( ... )` invocation: run the normal
/// `ui!` expansion, pretty-print the generated GPUI builder code, and return
/// it as a `&'static str` literal — a debugging/teaching aid to see exactly
/// what the macro emits (the moral equivalent of `cargo expand` scoped to
/// one invocation).
pub fn ui_expand_impl(input: TokenStream2) -> TokenStream2 {
    let expanded = ui_impl(input);
    // Wrap in a fn so prettyplease (which formats whole files) accepts it,
    // then strip the wrapper. Fall back to the raw token string for shapes
    // that don't parse as statements (e.g. multiple bare expressions).
    let pretty = match syn::parse2::<syn::File>(quote! { fn __ui_expand() { #expanded } }) {
        Ok(file) => {
            let s = prettyplease::unparse(&file);
            s.lines()
                .skip(1) // "fn __ui_expand() {"
                .take_while(|l| *l != "}")
                .map(|l| l.strip_prefix("    ").unwrap_or(l))
                .collect::<Vec<_>>()
                .join("\n")
        }
        Err(_) => expanded.to_string(),
    };
    let lit = proc_macro2::Literal::string(&pretty);
    quote! { #lit }
}

/// Expand the contents of a `color!( ... )` invocation into a `gpui::Hsla`
/// struct literal — the same compile-time RGB→HSL conversion the style
/// tokens use, exposed standalone so theme palettes can be `const`. Accepts
/// 3/4/6/8-digit hex (optional leading `#`, optionally quoted) or a named
/// color: `color!(1c1a17)`, `color!("#f5f0e8")`, `color!(red)`.
pub fn color_impl(input: TokenStream2) -> TokenStream2 {
    let raw: String = input.to_string();
    let s: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    let s = s.trim_matches('"');
    let s = s.strip_prefix('#').unwrap_or(s);
    match resolve_color(s) {
        Some(c) => c.hsla_expr(),
        None => syn::Error::new_spanned(
            input,
            format!(
                "`{s}` is not a color; expected 3/4/6/8-digit hex \
                 (optional leading `#`) or a named color"
            ),
        )
        .to_compile_error(),
    }
}

struct Ui {
    nodes: Vec<UiNode>,
}

impl Parse for Ui {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Ui {
            nodes: parse_nodes(input)?,
        })
    }
}

/// Parse `UiNode`s until the stream is exhausted — the shared body of every
/// node-list context (top level, control-flow branches, element children).
fn parse_nodes(input: ParseStream) -> Result<Vec<UiNode>> {
    let mut nodes = Vec::new();
    while !input.is_empty() {
        nodes.push(input.parse()?);
    }
    Ok(nodes)
}

struct IfNode {
    /// `Some(pat)` makes this an `if let #pat = #cond`; `None` is a plain
    /// boolean `if #cond`.
    pat: Option<syn::Pat>,
    cond: Expr,
    then_branch: Vec<UiNode>,
    else_branch: Option<Vec<UiNode>>,
}

struct ForNode {
    pat: syn::Pat,
    iter: Expr,
    body: Vec<UiNode>,
}

struct MatchArmNode {
    pat: syn::Pat,
    guard: Option<Expr>,
    body: Vec<UiNode>,
}

struct MatchNode {
    expr: Expr,
    arms: Vec<MatchArmNode>,
}

enum UiNode {
    Block(Expr),
    /// `{ ..expr }` — appended via `.children(expr)`, so any
    /// `IntoIterator<Item: IntoElement>` works: `Option` (renders nothing on
    /// `None`), `Vec<AnyElement>`, iterators.
    Spread(Expr),
    If(IfNode),
    For(ForNode),
    Match(MatchNode),
    Element(Element),
}

impl Parse for UiNode {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            // `..` must be claimed before Expr parsing, which would otherwise
            // swallow it as a RangeTo expression.
            if content.peek(Token![..]) {
                content.parse::<Token![..]>()?;
                let expr: Expr = content.parse()?;
                return Ok(UiNode::Spread(expr));
            }
            let expr: Expr = content.parse()?;
            return Ok(UiNode::Block(expr));
        }

        if input.peek(Token![if]) {
            input.parse::<Token![if]>()?;
            // `if let` needs manual parsing — syn's Expr does not accept a
            // `let` at this position. Branch bodies are ordinary node lists,
            // so multi-node `if let` bodies work like any other branch.
            let pat = if input.peek(Token![let]) {
                input.parse::<Token![let]>()?;
                let pat = syn::Pat::parse_multi_with_leading_vert(input)?;
                input.parse::<Token![=]>()?;
                Some(pat)
            } else {
                None
            };
            let cond = syn::Expr::parse_without_eager_brace(input)?;
            let then_content;
            braced!(then_content in input);
            let then_branch = parse_nodes(&then_content)?;

            let mut else_branch = None;
            if input.peek(Token![else]) {
                input.parse::<Token![else]>()?;
                if input.peek(Token![if]) {
                    else_branch = Some(vec![input.parse::<UiNode>()?]);
                } else {
                    let else_content;
                    braced!(else_content in input);
                    else_branch = Some(parse_nodes(&else_content)?);
                }
            }
            return Ok(UiNode::If(IfNode {
                pat,
                cond,
                then_branch,
                else_branch,
            }));
        }

        if input.peek(Token![for]) {
            input.parse::<Token![for]>()?;
            let pat = syn::Pat::parse_multi_with_leading_vert(input)?;
            input.parse::<Token![in]>()?;
            let iter = syn::Expr::parse_without_eager_brace(input)?;
            let body_content;
            braced!(body_content in input);
            let body = parse_nodes(&body_content)?;
            return Ok(UiNode::For(ForNode { pat, iter, body }));
        }

        if input.peek(Token![match]) {
            input.parse::<Token![match]>()?;
            let expr = syn::Expr::parse_without_eager_brace(input)?;
            let content;
            braced!(content in input);
            let mut arms = Vec::new();
            while !content.is_empty() {
                let pat = syn::Pat::parse_multi_with_leading_vert(&content)?;
                let guard = if content.peek(Token![if]) {
                    content.parse::<Token![if]>()?;
                    Some(content.parse::<Expr>()?)
                } else {
                    None
                };
                content.parse::<Token![=>]>()?;

                let body = if content.peek(syn::token::Brace) {
                    let arm_content;
                    braced!(arm_content in content);
                    parse_nodes(&arm_content)?
                } else {
                    vec![content.parse()?]
                };
                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                }
                arms.push(MatchArmNode { pat, guard, body });
            }
            return Ok(UiNode::Match(MatchNode { expr, arms }));
        }

        Ok(UiNode::Element(input.parse()?))
    }
}

/// Where a node sequence is being expanded. The control-flow nodes (If/For/
/// Match) expand identically in both contexts; only the *leaf* nodes (Element /
/// Block) differ — at top level they become standalone expressions, while as
/// children they become `__el = __el.child(..)` statements.
#[derive(Clone, Copy)]
enum Ctx {
    /// Top-level / branch body: each leaf is a bare expression.
    TopLevel,
    /// Inside a parent element: each leaf is appended via `el.child(..)`.
    Child,
}

/// Wrap an already-built stream in a brace group. `Group::new` takes the
/// stream by value, so — unlike interpolating a stream through `quote!` —
/// the subtree is moved, never re-cloned. This is what makes the emitter
/// single-pass: every token is materialized exactly once, no matter how
/// deeply the node that produced it is nested.
fn braced_group(inner: TokenStream2) -> TokenTree {
    TokenTree::Group(Group::new(Delimiter::Brace, inner))
}

/// Expand one node in the given context, appending to `out`. This is the
/// single source of truth for If/For/Match expansion in both contexts.
fn emit_node(out: &mut TokenStream2, node: &UiNode, ctx: Ctx) {
    match node {
        UiNode::Block(expr) => match ctx {
            Ctx::TopLevel => expr.to_tokens(out),
            Ctx::Child => out.extend(quote! { __el = __el.child( #expr ); }),
        },
        UiNode::Spread(expr) => match ctx {
            // No parent element to receive the children — reject rather than
            // emit something that half-works.
            Ctx::TopLevel => out.extend(
                syn::Error::new_spanned(
                    expr,
                    "spread `{ ..expr }` requires a parent element to receive the children",
                )
                .to_compile_error(),
            ),
            Ctx::Child => out.extend(quote! { __el = __el.children( #expr ); }),
        },
        UiNode::Element(el) => match ctx {
            Ctx::TopLevel => emit_element(out, el),
            Ctx::Child => {
                out.extend(quote! { __el = __el.child });
                let mut inner = TokenStream2::new();
                emit_element(&mut inner, el);
                out.append(TokenTree::Group(Group::new(Delimiter::Parenthesis, inner)));
                out.append(Punct::new(';', Spacing::Alone));
            }
        },
        UiNode::If(if_node) => {
            let cond = &if_node.cond;
            match &if_node.pat {
                Some(pat) => out.extend(quote! { if let #pat = #cond }),
                None => out.extend(quote! { if #cond }),
            }
            let mut then_body = TokenStream2::new();
            emit_nodes(&mut then_body, &if_node.then_branch, ctx);
            out.append(braced_group(then_body));
            if let Some(else_branch) = &if_node.else_branch {
                out.extend(quote! { else });
                let mut else_body = TokenStream2::new();
                emit_nodes(&mut else_body, else_branch, ctx);
                out.append(braced_group(else_body));
            }
        }
        UiNode::For(for_node) => {
            let pat = &for_node.pat;
            let iter = &for_node.iter;
            out.extend(quote! { for #pat in #iter });
            let mut body = TokenStream2::new();
            emit_nodes(&mut body, &for_node.body, ctx);
            out.append(braced_group(body));
        }
        UiNode::Match(match_node) => {
            let expr = &match_node.expr;
            out.extend(quote! { match #expr });
            let mut arms = TokenStream2::new();
            for arm in &match_node.arms {
                let pat = &arm.pat;
                arms.extend(quote! { #pat });
                if let Some(guard) = &arm.guard {
                    arms.extend(quote! { if #guard });
                }
                arms.extend(quote! { => });
                let mut body = TokenStream2::new();
                emit_nodes(&mut body, &arm.body, ctx);
                arms.append(braced_group(body));
            }
            out.append(braced_group(arms));
        }
    }
}

fn emit_nodes(out: &mut TokenStream2, nodes: &[UiNode], ctx: Ctx) {
    for node in nodes {
        emit_node(out, node, ctx);
    }
}

struct Element {
    /// A bare name (`row`, `badge`) or a path constructor (`Button::new`).
    /// Only single-ident heads participate in the built-in name table;
    /// multi-segment paths are always custom constructors.
    head: syn::Path,
    args: Vec<Arg>,
    children: Vec<UiNode>,
}

enum Arg {
    KeyValue(Ident, Expr),
    Expr(Expr),
}

impl Parse for Arg {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Ident) && input.peek2(Token![=]) {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let val: Expr = input.parse()?;
            Ok(Arg::KeyValue(key, val))
        } else {
            Ok(Arg::Expr(input.parse()?))
        }
    }
}

impl Parse for Element {
    fn parse(input: ParseStream) -> Result<Self> {
        let head: syn::Path = input.parse()?;

        let mut args = Vec::new();
        if input.peek(syn::token::Paren) {
            let content;
            parenthesized!(content in input);
            while !content.is_empty() {
                args.push(content.parse()?);
                // Commas between args are optional, but without them adjacent
                // expression args can merge (e.g. `size(20) .flag()` parses as
                // one method-call expression), so accepting them matters.
                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                }
            }
        }

        // Leaf elements (text-likes take content as an argument; `list`
        // renders through its closure) provably have no children, so a
        // `{ ... }` after one is unambiguous: it can only belong to the
        // enclosing node list. Leave it unconsumed and it parses as the
        // next sibling — no wrapper container needed.
        let is_leaf = head.get_ident().is_some_and(|name| {
            matches!(
                name.to_string().as_str(),
                "text" | "text_raw" | "label" | "list"
            )
        });

        let mut children = Vec::new();
        if !is_leaf && input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            while !content.is_empty() {
                children.push(content.parse()?);
            }
        }

        Ok(Element {
            head,
            args,
            children,
        })
    }
}

/// GPUI methods defined on `StatefulInteractiveElement` — they only exist
/// once the element has an ID (Div → Stateful<Div>). Seeing one on a known
/// div-backed container triggers automatic `.id()` injection.
fn requires_stateful(name: &str) -> bool {
    matches!(
        name,
        "on_click"
            | "on_hover"
            | "on_drag"
            | "tooltip"
            | "hoverable_tooltip"
            | "active"
            | "group_active"
            | "track_scroll"
            | "anchor_scroll"
            | "overflow_scroll"
            | "overflow_x_scroll"
            | "overflow_y_scroll"
    )
}

/// Would this argument be applied to the element as a builder call — a style
/// token, `key = value`, or call-style method? On custom constructors, the
/// leading run of args that are NOT builder args is passed to the constructor
/// itself: `Button::new("ok", label("Go"))` → `Button::new("ok").label("Go")`.
///
/// The boundary cases: a bare ident is a builder arg only when it maps to a
/// style token (an unknown ident in leading position is a constructor arg —
/// a variable, a unit variant); a call is a builder arg only with a
/// bare-ident callee (`Thing::default()` is a constructor arg); a method
/// call (`self.state.clone()`) is a constructor arg.
fn is_builder_arg(arg: &Arg) -> bool {
    match arg {
        Arg::KeyValue(..) => true,
        Arg::Expr(syn::Expr::Path(p)) => p
            .path
            .get_ident()
            .is_some_and(|i| token_to_direct_call(&i.to_string()).is_some()),
        Arg::Expr(syn::Expr::Call(call)) => {
            matches!(&*call.func, syn::Expr::Path(p) if p.path.get_ident().is_some())
        }
        _ => false,
    }
}

/// Emit the `let __el = __el...;` rebinding statement(s) for one element
/// argument. Rebinding (not reassignment) lets type-changing builder methods
/// like `.id()` (Div → Stateful<Div>) work mid-chain. Shared by every element
/// kind so arguments behave identically everywhere.
fn arg_stmt(arg: &Arg) -> TokenStream2 {
    match arg {
        Arg::KeyValue(key, val) => {
            if matches!(
                key.to_string().as_str(),
                "on_mouse_down" | "on_mouse_up" | "on_mouse_down_out" | "on_mouse_up_out"
            ) {
                quote! { let __el = __el.#key(gpui::MouseButton::Left, #val); }
            } else {
                quote! { let __el = __el.#key(#val); }
            }
        }
        Arg::Expr(syn::Expr::Path(path)) => {
            let path_str = path
                .path
                .get_ident()
                .map(|i| i.to_string())
                .unwrap_or_default();
            match token_to_direct_call(&path_str) {
                Some(call) => call,
                None => syn::Error::new_spanned(path, format!("Unknown style token `{path_str}`"))
                    .to_compile_error(),
            }
        }
        // `when(cond, <tokens>)` → `.when(cond, |__el| { ...; __el })` with
        // the trailing args expanded exactly like element args (style tokens,
        // `key = value`, call-style) — conditional styling without giving up
        // compile-time tokens. `when(cond, |el| ...)` still passes through
        // to GPUI's FluentBuilder below.
        Arg::Expr(syn::Expr::Call(call)) if when_token_form(call) => when_stmt(call),
        // `method(args)` → `__el.method(args)`
        Arg::Expr(syn::Expr::Call(call)) => {
            let func = &call.func;
            let call_args = &call.args;
            quote! { let __el = __el.#func(#call_args); }
        }
        // `recv.method(args)` → `__el.method(args)`; the receiver is dropped.
        // This mostly arises when two space-separated args merge into one
        // expression — separate args with commas to avoid it.
        Arg::Expr(syn::Expr::MethodCall(method)) => {
            let method_name = &method.method;
            let method_args = &method.args;
            quote! { let __el = __el.#method_name(#method_args); }
        }
        Arg::Expr(expr) => syn::Error::new_spanned(
            expr,
            "unsupported element argument; expected `key = value`, a style token, \
             or a method call",
        )
        .to_compile_error(),
    }
}

/// Is this `when(...)` call the conditional-token form (trailing args to be
/// applied to the element) rather than GPUI's raw `.when(cond, closure)`?
///
/// GPUI's `when` takes exactly (bool, closure), so three or more args are
/// always the token form. With exactly two, a closure literal — or a bare
/// path that is *not* a style token (a function-item callback) — passes
/// through; a style token, `key = value`, or method call is the token form.
fn when_token_form(call: &syn::ExprCall) -> bool {
    let is_when = matches!(&*call.func, syn::Expr::Path(p) if p.path.is_ident("when"));
    if !is_when || call.args.len() < 2 {
        return false;
    }
    if call.args.len() > 2 {
        return true;
    }
    match &call.args[1] {
        syn::Expr::Closure(_) => false,
        syn::Expr::Path(p) => p
            .path
            .get_ident()
            .is_some_and(|i| token_to_direct_call(&i.to_string()).is_some()),
        _ => true,
    }
}

/// Expand the conditional-token form of `when`. Trailing args reuse
/// `arg_stmt`, so everything valid in element-arg position is valid here —
/// including nested `when`s. Style tokens keep their compile-time expansion
/// (colors stay precomputed `Hsla` literals inside the branch).
fn when_stmt(call: &syn::ExprCall) -> TokenStream2 {
    let cond = &call.args[0];
    let mut body = TokenStream2::new();
    for expr in call.args.iter().skip(1) {
        // `bg = value` parses as an assignment expression in call position;
        // map it back to the `key = value` arg form.
        let arg = if let syn::Expr::Assign(assign) = expr {
            if let syn::Expr::Path(p) = &*assign.left
                && let Some(key) = p.path.get_ident()
            {
                Arg::KeyValue(key.clone(), (*assign.right).clone())
            } else {
                return syn::Error::new_spanned(
                    &assign.left,
                    "expected a method name on the left of `=` inside `when(...)`",
                )
                .to_compile_error();
            }
        } else {
            Arg::Expr(expr.clone())
        };
        body.extend(arg_stmt(&arg));
    }
    quote! { let __el = __el.when(#cond, |__el| { #body __el }); }
}

/// Emit an element as a block expression, appending to `out`. Args, styles,
/// and children are written straight into the block's own stream, which the
/// brace group then takes ownership of — single-pass, no re-cloning.
fn emit_element(out: &mut TokenStream2, el: &Element) {
    let head = &el.head;
    // Multi-segment paths (`Button::new`) never match the built-in name
    // table below — the empty string keeps them on the custom path.
    let name_str = head
        .get_ident()
        .map(|i| i.to_string())
        .unwrap_or_default();

    // ── text / text_raw / label ───────────────────────────────────────────
    if name_str == "text" || name_str == "text_raw" || name_str == "label" {
        // Text elements take their content as the first argument and never
        // have children — the parser leaves any following `{ ... }` to the
        // enclosing node list, so `el.children` is empty by construction.
        let mut style_stmts = TokenStream2::new();
        let mut content: Option<&Expr> = None;

        for (i, arg) in el.args.iter().enumerate() {
            if i == 0
                && let Arg::Expr(expr) = arg
            {
                content = Some(expr);
                continue;
            }
            style_stmts.extend(arg_stmt(arg));
        }

        // A wrapper div exists only to carry styles; unstyled text is
        // emitted bare (strings are IntoElement) — no extra layout node.
        if let (Some(expr), true) = (content, style_stmts.is_empty()) {
            expr.to_tokens(out);
            return;
        }
        let mut body = quote! { let __el = gpui::div(); };
        body.extend(style_stmts);
        match content {
            Some(expr) => body.extend(quote! { __el.child(#expr) }),
            None => body.extend(quote! { __el }),
        }
        out.append(braced_group(body));
        return;
    }

    // ── list — build uniform_list inline ─────────────────────────────────
    if name_str == "list" {
        let mut list_id: Option<TokenStream2> = None;
        let mut list_count: Option<TokenStream2> = None;
        let mut list_render: Option<TokenStream2> = None;
        let mut list_styles = quote! {};

        for arg in &el.args {
            if let Arg::KeyValue(key, val) = arg {
                let handled = match key.to_string().as_str() {
                    "id" => {
                        list_id = Some(quote! { #val });
                        true
                    }
                    "count" => {
                        list_count = Some(quote! { #val });
                        true
                    }
                    "render" | "renderer" => {
                        list_render = Some(quote! { #val });
                        true
                    }
                    _ => false,
                };
                if handled {
                    continue;
                }
            }
            list_styles.extend(arg_stmt(arg));
        }

        let id = list_id.unwrap_or_else(|| quote! { concat!(file!(), ":", line!()) });
        let count = list_count.unwrap_or_else(|| quote! { 0 });
        let render = list_render.unwrap_or_else(|| quote! { |_ix| gpui::div() });

        // The closure owns the render function; no Arc, no per-frame
        // allocation or refcount traffic.
        let mut body = quote! {
            let __render_fn = #render;
            let __el = gpui::uniform_list(
                #id,
                #count,
                move |range, _window, _cx| range.map(|ix| __render_fn(ix)).collect(),
            );
        };
        body.extend(list_styles);
        body.extend(quote! { __el });
        out.append(braced_group(body));
        return;
    }

    // ── Constructor ───────────────────────────────────────────────────────
    let explicit_id_val: Option<&syn::Expr> = el.args.iter().find_map(|a| {
        if let Arg::KeyValue(k, v) = a {
            if *k == "id" { Some(v) } else { None }
        } else {
            None
        }
    });

    // Base constructors for the built-in div-backed containers. Unknown
    // names fall through to calling `name()` as-is.
    let known_base: Option<TokenStream2> = match name_str.as_str() {
        "row" => Some(quote! { gpui::div().flex().flex_row() }),
        "col" | "card" => Some(quote! { gpui::div().flex().flex_col() }),
        "center" => Some(quote! { gpui::div().flex().flex_row().items_center().justify_center() }),
        "div" => Some(quote! { gpui::div() }),
        _ => None,
    };

    // Custom constructors (unknown names, and paths like `Button::new`) take
    // positional arguments: the leading run of args that aren't builder args
    // goes to the constructor, the rest apply as builder calls. Built-in
    // containers take no positional args, so their split is always 0.
    let is_custom = known_base.is_none() && name_str != "scroll";
    let ctor_split = if is_custom {
        el.args.iter().take_while(|a| !is_builder_arg(a)).count()
    } else {
        0
    };

    // Some API methods require a Stateful<Div> (i.e. the element must have
    // an element ID so GPUI can persist per-element state across frames).
    // When any of those appear on a known div-backed container — as a
    // `key = value` arg, a call-style arg, or a bare token — we inject
    // .id() into the constructor so the type is already Stateful<Div> when
    // we apply it. `id_consumed` tracks whether an explicit `id =` arg was
    // folded into the constructor (so the args loop must skip it).
    let auto_id = || quote! { concat!(file!(), ":", line!(), ":", column!()) };
    let (constructor, id_consumed) = if name_str == "scroll" {
        let id = explicit_id_val
            .map(|v| quote! { #v })
            .unwrap_or_else(auto_id);
        (
            quote! { gpui::div().id(#id).overflow_y_scroll() },
            explicit_id_val.is_some(),
        )
    } else if let Some(base) = known_base {
        let needs_stateful = el.args.iter().any(|a| match a {
            Arg::KeyValue(k, _) => requires_stateful(&k.to_string()),
            Arg::Expr(syn::Expr::Path(p)) => p
                .path
                .get_ident()
                .is_some_and(|i| requires_stateful(&i.to_string())),
            Arg::Expr(syn::Expr::Call(c)) => {
                if let syn::Expr::Path(p) = &*c.func {
                    p.path
                        .get_ident()
                        .is_some_and(|i| requires_stateful(&i.to_string()))
                } else {
                    false
                }
            }
            _ => false,
        });
        if needs_stateful {
            let id = explicit_id_val
                .map(|v| quote! { #v })
                .unwrap_or_else(auto_id);
            (quote! { #base.id(#id) }, explicit_id_val.is_some())
        } else {
            (base, false)
        }
    } else {
        let ctor_args: Vec<&Expr> = el.args[..ctor_split]
            .iter()
            .map(|a| match a {
                Arg::Expr(expr) => expr,
                Arg::KeyValue(..) => unreachable!("key-value args are never constructor args"),
            })
            .collect();
        (quote! { #head( #(#ctor_args),* ) }, false)
    };

    // ── Body: constructor, args, children, result — built in one stream ────
    // Args REBIND `__el` (`let __el = __el.x();`) so type-changing builder
    // methods work mid-chain — `.id()` turns Div into Stateful<Div>, and a
    // plain reassignment would be a type error. Children then switch to a
    // single mutable binding, because control flow (`if`/`for`/`match`) can't
    // rebind across scopes.
    // Every element (scroll included) yields its concrete type; callers
    // that need type erasure can .into_any_element() themselves. Boxing
    // here would cost an allocation per render for everyone.
    let mut body = quote! { let __el = #constructor; };
    for arg in &el.args[ctor_split..] {
        if id_consumed && matches!(arg, Arg::KeyValue(k, _) if *k == "id") {
            continue; // already folded into the constructor
        }
        // Past the split, a non-builder arg on a custom constructor is
        // misplaced — surface it instead of letting `arg_stmt`'s generic
        // fallback fire (its "Unknown style token" message would mislead).
        if is_custom && !is_builder_arg(arg) {
            let expr = match arg {
                Arg::Expr(expr) => expr,
                Arg::KeyValue(..) => unreachable!("key-value args are builder args"),
            };
            body.extend(
                syn::Error::new_spanned(
                    expr,
                    "not a style token, `key = value` pair, or builder call — \
                     constructor arguments must come before those",
                )
                .to_compile_error(),
            );
            continue;
        }
        body.extend(arg_stmt(arg));
    }
    if !el.children.is_empty() {
        body.extend(quote! { let mut __el = __el; });
    }
    emit_nodes(&mut body, &el.children, Ctx::Child);
    body.extend(quote! { __el });
    out.append(braced_group(body));
}

/// A parsed color token suffix. 3- and 6-digit hex (and named colors) are
/// opaque RGB; 4- and 8-digit hex carry an alpha channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResolvedColor {
    Rgb(u32),
    Rgba(u32),
}

impl ResolvedColor {
    /// `[r, g, b, a]` in 0.0..=1.0, replicating `gpui::rgb` / `gpui::rgba`.
    fn parts(&self) -> [f32; 4] {
        match *self {
            ResolvedColor::Rgb(v) => {
                let [_, r, g, b] = v.to_be_bytes().map(|b| (b as f32) / 255.0);
                [r, g, b, 1.0]
            }
            ResolvedColor::Rgba(v) => {
                let [r, g, b, a] = v.to_be_bytes().map(|b| (b as f32) / 255.0);
                [r, g, b, a]
            }
        }
    }

    /// A `gpui::Hsla` struct literal with the color-space conversion done at
    /// macro-expansion time. Replicates GPUI's `From<Rgba> for Hsla`
    /// (gpui/src/color.rs) operation-for-operation so the emitted value is
    /// bit-identical to what the runtime conversion would produce — every
    /// color-taking GPUI method funnels into `Hsla`, so this skips both the
    /// hex unpacking of `rgb()` and the RGB→HSL math on every render.
    fn hsla_expr(&self) -> TokenStream2 {
        let [r, g, b, a] = self.parts();

        let max = r.max(g.max(b));
        let min = r.min(g.min(b));
        let delta = max - min;

        let l = (max + min) / 2.0;
        let s = if l == 0.0 || l == 1.0 {
            0.0
        } else if l < 0.5 {
            delta / (2.0 * l)
        } else {
            delta / (2.0 - 2.0 * l)
        };

        let h = if delta == 0.0 {
            0.0
        } else if max == r {
            ((g - b) / delta).rem_euclid(6.0) / 6.0
        } else if max == g {
            ((b - r) / delta + 2.0) / 6.0
        } else {
            ((r - g) / delta + 4.0) / 6.0
        };

        quote! { gpui::Hsla { h: #h, s: #s, l: #l, a: #a } }
    }
}

/// Parse a hex color (no leading `#`): 6/3 digits → `0xRRGGBB`, 8/4 digits →
/// `0xRRGGBBAA`. The 3- and 4-digit short forms expand each nibble CSS-style
/// (`abc` → `aabbcc`).
fn hex_color(s: &str) -> Option<ResolvedColor> {
    if !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let nibble = |i: usize| u32::from_str_radix(&s[i..i + 1], 16).unwrap() * 17;
    match s.len() {
        6 => u32::from_str_radix(s, 16).ok().map(ResolvedColor::Rgb),
        8 => u32::from_str_radix(s, 16).ok().map(ResolvedColor::Rgba),
        3 => Some(ResolvedColor::Rgb(
            (nibble(0) << 16) | (nibble(1) << 8) | nibble(2),
        )),
        4 => Some(ResolvedColor::Rgba(
            (nibble(0) << 24) | (nibble(1) << 16) | (nibble(2) << 8) | nibble(3),
        )),
        _ => None,
    }
}

/// A small set of CSS-style named colors.
fn named_color(s: &str) -> Option<u32> {
    match s {
        "black" => Some(0x000000),
        "white" => Some(0xffffff),
        "gray" => Some(0x808080),
        "red" => Some(0xff0000),
        "green" => Some(0x00ff00),
        "blue" => Some(0x0000ff),
        "yellow" => Some(0xffff00),
        "cyan" => Some(0x00ffff),
        "magenta" => Some(0xff00ff),
        "orange" => Some(0xffa500),
        "purple" => Some(0x800080),
        _ => None,
    }
}

/// Resolve a color token tail: hex first, then a named color.
fn resolve_color(s: &str) -> Option<ResolvedColor> {
    hex_color(s).or_else(|| named_color(s).map(ResolvedColor::Rgb))
}

/// Style tokens that map 1:1 to a zero-argument GPUI method of the same name.
/// Sorted (byte order) and binary-searched directly — a test guards the
/// ordering. The `rounded_*` family and the box-model auto/full/px/fraction
/// suffixes are validated structurally in `token_to_direct_call` instead of
/// being enumerated here.
static ZERO_ARG_TOKENS: &[&str] = &[
    "absolute",
    "aspect_square",
    "block",
    "border_dashed",
    "col_end_auto",
    "col_span_full",
    "col_start_auto",
    "content_around",
    "content_between",
    "content_center",
    "content_end",
    "content_evenly",
    "content_normal",
    "content_start",
    "content_stretch",
    "cursor_alias",
    "cursor_col_resize",
    "cursor_context_menu",
    "cursor_copy",
    "cursor_crosshair",
    "cursor_default",
    "cursor_e_resize",
    "cursor_ew_resize",
    "cursor_grab",
    "cursor_grabbing",
    "cursor_move",
    "cursor_n_resize",
    "cursor_nesw_resize",
    "cursor_no_drop",
    "cursor_not_allowed",
    "cursor_ns_resize",
    "cursor_nwse_resize",
    "cursor_pointer",
    "cursor_row_resize",
    "cursor_s_resize",
    "cursor_text",
    "cursor_vertical_text",
    "cursor_w_resize",
    "debug",
    "debug_below",
    "flex",
    "flex_1",
    "flex_auto",
    "flex_col",
    "flex_col_reverse",
    "flex_grow",
    "flex_grow_0",
    "flex_grow_1",
    "flex_initial",
    "flex_none",
    "flex_nowrap",
    "flex_row",
    "flex_row_reverse",
    "flex_shrink",
    "flex_shrink_0",
    "flex_shrink_1",
    "flex_wrap",
    "flex_wrap_reverse",
    "grid",
    "hidden",
    "invisible",
    "italic",
    "items_baseline",
    "items_center",
    "items_end",
    "items_start",
    "items_stretch",
    "justify_around",
    "justify_between",
    "justify_center",
    "justify_end",
    "justify_evenly",
    "justify_start",
    "line_through",
    "not_italic",
    "overflow_hidden",
    "overflow_scroll",
    "overflow_x_hidden",
    "overflow_x_scroll",
    "overflow_y_hidden",
    "overflow_y_scroll",
    "relative",
    "row_end_auto",
    "row_span_full",
    "row_start_auto",
    "self_baseline",
    "self_center",
    "self_end",
    "self_flex_end",
    "self_flex_start",
    "self_start",
    "self_stretch",
    "shadow_2xl",
    "shadow_2xs",
    "shadow_lg",
    "shadow_md",
    "shadow_none",
    "shadow_sm",
    "shadow_xl",
    "shadow_xs",
    "text_2xl",
    "text_3xl",
    "text_base",
    "text_center",
    "text_decoration_0",
    "text_decoration_1",
    "text_decoration_2",
    "text_decoration_4",
    "text_decoration_8",
    "text_decoration_none",
    "text_decoration_solid",
    "text_decoration_wavy",
    "text_ellipsis",
    "text_ellipsis_middle",
    "text_ellipsis_start",
    "text_left",
    "text_lg",
    "text_right",
    "text_sm",
    "text_xl",
    "text_xs",
    "truncate",
    "underline",
    "visible",
    "whitespace_normal",
    "whitespace_nowrap",
];

fn is_zero_arg_token(token: &str) -> bool {
    ZERO_ARG_TOKENS.binary_search(&token).is_ok()
}

/// Tailwind-style fraction suffixes and their relative values (`full` = 100%).
/// Mirrors GPUI's generated method set exactly.
const FRACTIONS: &[(&str, f32)] = &[
    ("full", 1.0),
    ("1_2", 0.5),
    ("1_3", 1.0 / 3.0),
    ("2_3", 2.0 / 3.0),
    ("1_4", 0.25),
    ("2_4", 0.5),
    ("3_4", 0.75),
    ("1_5", 0.2),
    ("2_5", 0.4),
    ("3_5", 0.6),
    ("4_5", 0.8),
    ("1_6", 1.0 / 6.0),
    ("5_6", 5.0 / 6.0),
    ("1_12", 1.0 / 12.0),
];

/// Numeric px token prefixes and the GPUI method each maps to. The bool marks
/// prefixes where GPUI supports negative values (box-model lengths; not
/// borders, radii, or text sizes). A test asserts prefix == method + "_"
/// (with `text_` → `text_size` the single alias) so the columns can't drift.
const PX_METHODS: &[(&str, &str, bool)] = &[
    ("gap_", "gap", true),
    ("gap_x_", "gap_x", true),
    ("gap_y_", "gap_y", true),
    ("p_", "p", true),
    ("px_", "px", true),
    ("py_", "py", true),
    ("pt_", "pt", true),
    ("pb_", "pb", true),
    ("pl_", "pl", true),
    ("pr_", "pr", true),
    ("m_", "m", true),
    ("mx_", "mx", true),
    ("my_", "my", true),
    ("mt_", "mt", true),
    ("mb_", "mb", true),
    ("ml_", "ml", true),
    ("mr_", "mr", true),
    ("w_", "w", true),
    ("h_", "h", true),
    ("size_", "size", true),
    ("min_size_", "min_size", true),
    ("max_size_", "max_size", true),
    ("min_w_", "min_w", true),
    ("max_w_", "max_w", true),
    ("min_h_", "min_h", true),
    ("max_h_", "max_h", true),
    ("top_", "top", true),
    ("bottom_", "bottom", true),
    ("left_", "left", true),
    ("right_", "right", true),
    ("inset_", "inset", true),
    ("rounded_", "rounded", false),
    ("rounded_t_", "rounded_t", false),
    ("rounded_b_", "rounded_b", false),
    ("rounded_l_", "rounded_l", false),
    ("rounded_r_", "rounded_r", false),
    ("rounded_tl_", "rounded_tl", false),
    ("rounded_tr_", "rounded_tr", false),
    ("rounded_bl_", "rounded_bl", false),
    ("rounded_br_", "rounded_br", false),
    ("border_", "border", false),
    ("border_x_", "border_x", false),
    ("border_y_", "border_y", false),
    ("border_t_", "border_t", false),
    ("border_b_", "border_b", false),
    ("border_l_", "border_l", false),
    ("border_r_", "border_r", false),
    ("text_", "text_size", false),
    ("line_height_", "line_height", false),
    ("flex_basis_", "flex_basis", false),
    ("scrollbar_width_", "scrollbar_width", false),
];

/// Grid token prefixes → GPUI methods taking bare integers (u16 / i16, not
/// px). The bool marks signed methods, which also accept `neg_`
/// (`col_start_neg_1`). Same prefix == method + "_" test guard as PX_METHODS.
const GRID_METHODS: &[(&str, &str, bool)] = &[
    ("grid_cols_", "grid_cols", false),
    ("grid_cols_min_content_", "grid_cols_min_content", false),
    ("grid_cols_max_content_", "grid_cols_max_content", false),
    ("grid_rows_", "grid_rows", false),
    ("col_span_", "col_span", false),
    ("row_span_", "row_span", false),
    ("col_start_", "col_start", true),
    ("col_end_", "col_end", true),
    ("row_start_", "row_start", true),
    ("row_end_", "row_end", true),
];

fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// All-digit suffix after `prefix`: `int_suffix("px_8", "px_") == Some(8)`.
fn int_suffix(token: &str, prefix: &str) -> Option<u32> {
    let s = token.strip_prefix(prefix)?;
    if !all_digits(s) {
        return None;
    }
    s.parse().ok()
}

/// All-digit integer suffix, optionally negated via `neg_` when `signed`:
/// `signed_int_suffix("col_start_neg_1", "col_start_", true) == Some(-1)`.
fn signed_int_suffix(token: &str, prefix: &str, signed: bool) -> Option<i32> {
    let s = token.strip_prefix(prefix)?;
    let (negate, s) = match s.strip_prefix("neg_") {
        Some(rest) if signed => (true, rest),
        Some(_) => return None,
        None => (false, s),
    };
    if !all_digits(s) {
        return None;
    }
    let v: i32 = s.parse().ok()?;
    Some(if negate { -v } else { v })
}

/// Pixel-value suffix after `prefix`: integer (`8` → 8.0) or `p`-decimal
/// (`2p5` → 2.5), optionally negated via `neg_` where GPUI supports it.
fn px_suffix(token: &str, prefix: &str, neg_ok: bool) -> Option<f32> {
    let s = token.strip_prefix(prefix)?;
    let (negate, s) = match s.strip_prefix("neg_") {
        Some(rest) if neg_ok => (true, rest),
        Some(_) => return None,
        None => (false, s),
    };
    let v: f32 = match s.split_once('p') {
        Some((int, frac)) if all_digits(int) && all_digits(frac) => {
            format!("{int}.{frac}").parse().ok()?
        }
        Some(_) => return None,
        None if all_digits(s) => s.parse::<u32>().ok()? as f32,
        None => return None,
    };
    Some(if negate { -v } else { v })
}

/// Expand a style token (e.g. `px_8`, `bg_1c1a17`, `semibold`) into a direct
/// GPUI method call at compile time, bypassing all runtime string matching.
/// Returns `None` for any token this macro doesn't recognize — callers turn
/// that into an "Unknown style token" compile error, so this function is the
/// single source of truth for which tokens are valid.
///
/// Numeric suffixes are raw pixels (`w_240` → 240px), with `p` as a decimal
/// point (`p_2p5` → 2.5px) and `neg_` for negatives (`m_neg_8` → -8px) —
/// unlike GPUI's rem-based Tailwind scale. Rem-based values remain available
/// via key/value args (`w = rems(2.0)`).
///
/// Lookup order matters: exact tokens → structural suffixes → aliases /
/// special forms → colors → numeric forms. Colors win over generic numerics,
/// so `text_112` is the color `#112` while `text_16` (not a valid hex length)
/// is a 16px text size.
fn token_to_direct_call(token_str: &str) -> Option<TokenStream2> {
    fn method(name: &str) -> syn::Ident {
        syn::Ident::new(name, proc_macro2::Span::call_site())
    }

    // ── Zero-arg tokens: token name == GPUI method name ───────────────────────
    if is_zero_arg_token(token_str) {
        let m = method(token_str);
        return Some(quote! { let __el = __el.#m(); });
    }

    // rounded_{size} / rounded_{side}_{size} with a named size — the token is
    // again exactly the method name, so validate structurally instead of
    // enumerating all 81 combinations.
    if let Some(rest) = token_str.strip_prefix("rounded_") {
        const SIDES: &[&str] = &["t", "b", "l", "r", "tl", "tr", "bl", "br"];
        const SIZES: &[&str] = &["none", "xs", "sm", "md", "lg", "xl", "2xl", "3xl", "full"];
        let (side, size) = match rest.split_once('_') {
            Some((side, size)) => (Some(side), size),
            None => (None, rest),
        };
        if side.is_none_or(|s| SIDES.contains(&s)) && SIZES.contains(&size) {
            let m = method(token_str);
            return Some(quote! { let __el = __el.#m(); });
        }
    }

    // ── Box-model suffix tokens: auto / full / px / fractions ─────────────────
    // These map 1:1 onto GPUI's generated Tailwind-style zero-arg methods
    // (`w_full`, `mx_auto`, `w_1_2`, `top_px`, ...). `auto` only exists where
    // GPUI allows it (sizes, margins, positions — not padding or gap).
    {
        const AUTO_PREFIXES: &[&str] = &[
            "w", "h", "size", "min_size", "min_w", "min_h", "max_size", "max_w", "max_h", "m",
            "mt", "mb", "my", "mx", "ml", "mr", "inset", "top", "bottom", "left", "right",
        ];
        const NO_AUTO_PREFIXES: &[&str] = &[
            "gap", "gap_x", "gap_y", "p", "pt", "pb", "px", "py", "pl", "pr",
        ];
        let prefixes = AUTO_PREFIXES
            .iter()
            .map(|p| (*p, true))
            .chain(NO_AUTO_PREFIXES.iter().map(|p| (*p, false)));
        for (prefix, auto_ok) in prefixes {
            let Some(suffix) = token_str
                .strip_prefix(prefix)
                .and_then(|rest| rest.strip_prefix('_'))
            else {
                continue;
            };
            if suffix == "px"
                || (auto_ok && suffix == "auto")
                || FRACTIONS.iter().any(|(f, _)| *f == suffix)
            {
                let m = method(token_str);
                return Some(quote! { let __el = __el.#m(); });
            }
        }
    }

    // flex_basis has no generated suffix methods in GPUI, so expand fractions
    // and `auto` through the custom setter.
    if let Some(suffix) = token_str.strip_prefix("flex_basis_") {
        if suffix == "auto" {
            return Some(quote! { let __el = __el.flex_basis(gpui::auto()); });
        }
        if let Some((_, f)) = FRACTIONS.iter().find(|(name, _)| *name == suffix) {
            return Some(quote! { let __el = __el.flex_basis(gpui::relative(#f)); });
        }
        // Numeric values are handled by the px loop below.
    }

    // ── Aliases and tokens that don't map 1:1 to a method name ────────────────
    match token_str {
        "shadow" => return Some(quote! { let __el = __el.shadow_sm(); }),
        "rounded" => return Some(quote! { let __el = __el.rounded_sm(); }),
        "col" => return Some(quote! { let __el = __el.flex_col(); }),
        "row" => return Some(quote! { let __el = __el.flex_row(); }),
        "col_reverse" => return Some(quote! { let __el = __el.flex_col_reverse(); }),
        "row_reverse" => return Some(quote! { let __el = __el.flex_row_reverse(); }),
        "no_underline" => return Some(quote! { let __el = __el.text_decoration_none(); }),
        _ => {}
    }

    // Font weights, with or without the `font_` prefix (`semibold` / `font_semibold`).
    let weight = match token_str.strip_prefix("font_").unwrap_or(token_str) {
        "thin" => Some("THIN"),
        "extra_light" => Some("EXTRA_LIGHT"),
        "light" => Some("LIGHT"),
        "normal" => Some("NORMAL"),
        "medium" => Some("MEDIUM"),
        "semibold" => Some("SEMIBOLD"),
        "bold" => Some("BOLD"),
        "extra_bold" => Some("EXTRA_BOLD"),
        "black" => Some("BLACK"),
        _ => None,
    };
    if let Some(w) = weight {
        let w = method(w);
        return Some(quote! { let __el = __el.font_weight(gpui::FontWeight::#w); });
    }

    // Border shorthands: 1px width.
    if matches!(
        token_str,
        "border" | "border_x" | "border_y" | "border_t" | "border_b" | "border_l" | "border_r"
    ) {
        let m = method(token_str);
        return Some(quote! { let __el = __el.#m(gpui::px(1.0)); });
    }

    // ── Color tokens: bg_*, text_bg_*, text_decoration_*, text_*, border_* ────
    // Hex (3/4/6/8 digits) or a named color; 4/8-digit hex carries alpha. All
    // emit a precomputed `Hsla` literal — zero color math at runtime.
    if let Some(name) = token_str.strip_prefix("bg_")
        && let Some(c) = resolve_color(name)
    {
        let color = c.hsla_expr();
        return Some(quote! { let __el = __el.bg(#color); });
    }
    if let Some(name) = token_str.strip_prefix("text_bg_")
        && let Some(c) = resolve_color(name)
    {
        let color = c.hsla_expr();
        return Some(quote! { let __el = __el.text_bg(#color); });
    }
    if let Some(name) = token_str.strip_prefix("text_decoration_")
        && let Some(c) = resolve_color(name)
    {
        let color = c.hsla_expr();
        return Some(quote! { let __el = __el.text_decoration_color(#color); });
    }
    if let Some(name) = token_str.strip_prefix("text_")
        && let Some(c) = resolve_color(name)
    {
        let color = c.hsla_expr();
        return Some(quote! { let __el = __el.text_color(#color); });
    }
    if let Some(name) = token_str.strip_prefix("border_")
        && let Some(c) = resolve_color(name)
    {
        let color = c.hsla_expr();
        return Some(quote! { let __el = __el.border_color(#color); });
    }

    // ── Numeric px-based spacing / size / position / border / rounded tokens ──
    for (prefix, m, neg_ok) in PX_METHODS {
        if let Some(v) = px_suffix(token_str, prefix, *neg_ok) {
            let m = method(m);
            return Some(quote! { let __el = __el.#m(gpui::px(#v)); });
        }
    }

    // ── Grid integer tokens (u16 / i16, not px) ────────────────────────────────
    for (prefix, m, signed) in GRID_METHODS {
        let Some(v) = signed_int_suffix(token_str, prefix, *signed) else {
            continue;
        };
        let m = method(m);
        return if *signed {
            let v = v as i16;
            Some(quote! { let __el = __el.#m(#v); })
        } else {
            let v = v as u16;
            Some(quote! { let __el = __el.#m(#v); })
        };
    }

    // ── line_clamp_N ───────────────────────────────────────────────────────────
    if let Some(n) = int_suffix(token_str, "line_clamp_") {
        let n = n as usize;
        return Some(quote! { let __el = __el.line_clamp(#n); });
    }

    // ── opacity_N (0-100, or a `p`-decimal like `opacity_0p5`) ────────────────
    if let Some(v) = px_suffix(token_str, "opacity_", false) {
        let mut v = v;
        if v > 1.0 {
            v /= 100.0;
        }
        let v = v.clamp(0.0, 1.0);
        return Some(quote! { let __el = __el.opacity(#v); });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    use ResolvedColor::{Rgb, Rgba};

    #[test]
    fn hex_color_6_digit() {
        assert_eq!(hex_color("1c1a17"), Some(Rgb(0x1c1a17)));
        assert_eq!(hex_color("ffffff"), Some(Rgb(0xffffff)));
    }

    #[test]
    fn hex_color_3_digit_expands() {
        // #abc → #aabbcc
        assert_eq!(hex_color("abc"), Some(Rgb(0xaabbcc)));
        assert_eq!(hex_color("f00"), Some(Rgb(0xff0000)));
    }

    #[test]
    fn hex_color_alpha_forms() {
        assert_eq!(hex_color("1c1a17cc"), Some(Rgba(0x1c1a17cc)));
        // #abcd → #aabbccdd
        assert_eq!(hex_color("abcd"), Some(Rgba(0xaabbccdd)));
    }

    #[test]
    fn hex_color_rejects_non_hex_and_bad_len() {
        assert_eq!(hex_color("xyz"), None);
        assert_eq!(hex_color("12"), None); // length 2 unsupported
        assert_eq!(hex_color("12345"), None); // length 5 unsupported
    }

    #[test]
    fn named_colors_resolve() {
        assert_eq!(named_color("black"), Some(0x000000));
        assert_eq!(named_color("orange"), Some(0xffa500));
        assert_eq!(named_color("chartreuse"), None);
    }

    #[test]
    fn resolve_color_prefers_hex_then_named() {
        assert_eq!(resolve_color("ff0000"), Some(Rgb(0xff0000)));
        assert_eq!(resolve_color("red"), Some(Rgb(0xff0000)));
        assert_eq!(resolve_color("nope"), None);
    }

    // token_to_direct_call is the single source of truth for valid tokens.
    // We assert Some/None classification rather than exact token output.
    fn maps(tok: &str) -> bool {
        token_to_direct_call(tok).is_some()
    }

    #[test]
    fn known_fixed_tokens_map() {
        for t in [
            "semibold",
            "truncate",
            "rounded",
            "rounded_full",
            "shadow_lg",
            "flex_1",
            "items_center",
            "justify_between",
            "border_1",
            "overflow_hidden",
        ] {
            assert!(maps(t), "expected `{t}` to map");
        }
    }

    #[test]
    fn parameterized_tokens_map() {
        for t in [
            "px_8",
            "py_12",
            "gap_6",
            "w_240",
            "h_full",
            "mt_4",
            "min_w_0",
            "rounded_8",
        ] {
            assert!(maps(t), "expected `{t}` to map");
        }
    }

    #[test]
    fn color_tokens_map() {
        for t in [
            "bg_1c1a17",
            "text_f5f0e8",
            "border_2d2b27",
            "bg_red",
            "text_abc",
        ] {
            assert!(maps(t), "expected color token `{t}` to map");
        }
    }

    #[test]
    fn unknown_tokens_do_not_map() {
        // These should yield None → callers emit "Unknown style token".
        for t in [
            "definitely_not_a_token",
            "px_",
            "bg_zzzzzz",
            "wobble_9",
            // No CursorStyle::None in GPUI — must not silently generate it.
            "cursor_none",
            // `auto` is only valid where GPUI generates it.
            "p_auto",
            "gap_auto",
            // Negatives only where GPUI supports them.
            "border_neg_2",
            "rounded_neg_4",
            "text_neg_12",
            // Malformed numerics.
            "w_p5",
            "w_5p",
            "m_neg_",
        ] {
            assert!(!maps(t), "expected `{t}` to be rejected");
        }
    }

    // ── GPUI Styled parity (crates/gpui/src/styled.rs + gpui_macros/styles.rs) ──

    fn expansion(tok: &str) -> String {
        token_to_direct_call(tok)
            .unwrap_or_else(|| panic!("expected `{tok}` to map"))
            .to_string()
    }

    #[test]
    fn parity_zero_arg_tokens_map() {
        for t in [
            "items_stretch",
            "justify_evenly",
            "self_start",
            "self_end",
            "self_flex_start",
            "self_flex_end",
            "self_center",
            "self_baseline",
            "self_stretch",
            "flex_grow_0",
            "flex_grow_1",
            "flex_shrink_1",
            "aspect_square",
            "text_ellipsis_start",
            "text_ellipsis_middle",
            "overflow_y_scroll",
            "debug",
            "debug_below",
        ] {
            assert!(maps(t), "expected `{t}` to map");
        }
    }

    #[test]
    fn box_suffix_tokens_pass_through() {
        // auto / full / px / fractions map to GPUI's same-named methods.
        for t in [
            "w_auto",
            "h_auto",
            "size_auto",
            "m_auto",
            "mx_auto",
            "top_auto",
            "inset_auto",
            "min_w_full",
            "max_h_full",
            "gap_full",
            "p_full",
            "w_px",
            "gap_px",
            "m_px",
            "w_1_2",
            "h_2_3",
            "p_3_4",
            "min_w_1_12",
            "left_4_5",
        ] {
            let out = expansion(t);
            assert!(out.contains(&format!("{t} ()")), "`{t}` → {out}");
        }
    }

    #[test]
    fn negative_and_decimal_px_tokens() {
        assert!(expansion("m_neg_8").contains("m (gpui :: px (- 8f32))"));
        assert!(expansion("top_neg_4").contains("top (gpui :: px (- 4f32))"));
        assert!(expansion("p_2p5").contains("p (gpui :: px (2.5f32))"));
        assert!(expansion("m_neg_1p5").contains("m (gpui :: px (- 1.5f32))"));
        assert!(expansion("border_0p5").contains("border (gpui :: px (0.5f32))"));
    }

    #[test]
    fn flex_basis_fractions_and_auto() {
        assert!(expansion("flex_basis_1_2").contains("flex_basis (gpui :: relative (0.5f32))"));
        assert!(expansion("flex_basis_auto").contains("flex_basis (gpui :: auto ())"));
        assert!(expansion("flex_basis_20").contains("flex_basis (gpui :: px (20f32))"));
    }

    #[test]
    fn alpha_and_decoration_colors() {
        assert!(expansion("bg_1c1a17cc").contains("bg (gpui :: Hsla {"));
        assert!(expansion("text_abcd").contains("text_color (gpui :: Hsla {"));
        assert!(
            expansion("text_decoration_ff0000").contains("text_decoration_color (gpui :: Hsla {")
        );
        assert!(expansion("border_abc").contains("border_color (gpui :: Hsla {"));
    }

    #[test]
    fn colors_are_precomputed_hsla_literals() {
        // Pure red: h=0, s=1, l=0.5 — the RGB→HSL conversion happens at
        // expansion time, so the output must contain the final components.
        let out = expansion("bg_ff0000");
        assert!(out.contains("h : 0f32"), "{out}");
        assert!(out.contains("s : 1f32"), "{out}");
        assert!(out.contains("l : 0.5f32"), "{out}");
        assert!(out.contains("a : 1f32"), "{out}");
        // Alpha channel carried through from 8-digit hex: 0x80/255 ≈ 0.50196.
        let out = expansion("bg_ff000080");
        assert!(out.contains("a : 0.5019608f32"), "{out}");
        // No runtime color constructors or conversions anywhere.
        for t in ["bg_1c1a17", "text_f5f0e8", "bg_red", "text_bg_abc"] {
            let out = expansion(t);
            assert!(!out.contains("rgb"), "`{t}` must not call rgb(): {out}");
            assert!(
                !out.contains("into"),
                "`{t}` must not convert at runtime: {out}"
            );
        }
    }

    #[test]
    fn grid_content_sizing_and_negative_positions() {
        assert!(expansion("grid_cols_min_content_3").contains("grid_cols_min_content (3u16)"));
        assert!(expansion("grid_cols_max_content_2").contains("grid_cols_max_content (2u16)"));
        assert!(expansion("col_start_neg_1").contains("col_start (- 1i16)"));
        // Unsigned grid tokens reject neg_.
        assert!(!maps("col_span_neg_2"));
    }

    // ── Table invariants ───────────────────────────────────────────────────────

    #[test]
    fn zero_arg_token_table_is_sorted() {
        // is_zero_arg_token binary-searches the static directly.
        assert!(
            ZERO_ARG_TOKENS.is_sorted(),
            "ZERO_ARG_TOKENS must stay sorted (byte order)"
        );
    }

    #[test]
    fn method_table_prefixes_match_their_methods() {
        // A prefix/method typo would silently map a token to the wrong GPUI
        // method. `text_` → `text_size` is the single deliberate alias.
        for (prefix, method, _) in PX_METHODS {
            if *prefix == "text_" {
                assert_eq!(*method, "text_size");
                continue;
            }
            assert_eq!(
                format!("{method}_"),
                *prefix,
                "PX_METHODS prefix/method mismatch"
            );
        }
        for (prefix, method, _) in GRID_METHODS {
            assert_eq!(
                format!("{method}_"),
                *prefix,
                "GRID_METHODS prefix/method mismatch"
            );
        }
    }

    #[test]
    fn min_max_size_and_scrollbar_width() {
        assert!(expansion("min_size_24").contains("min_size (gpui :: px (24f32))"));
        assert!(expansion("max_size_128").contains("max_size (gpui :: px (128f32))"));
        assert!(expansion("scrollbar_width_8").contains("scrollbar_width (gpui :: px (8f32))"));
    }
}

/// End-to-end expansion tests: feed DSL source through the same path the
/// proc-macro entry point uses and assert on the generated code. These guard
/// refactors of the expansion logic.
#[cfg(test)]
mod expansion_tests {
    use super::*;

    fn expand(src: &str) -> String {
        let ts: TokenStream2 = src.parse().expect("bench input must tokenize");
        ui_impl(ts).to_string()
    }

    #[test]
    fn element_with_tokens_and_children() {
        let out = expand(r#"row(px_8 gap_4 bg_1c1a17) { text("hi" text_sm) }"#);
        assert!(out.contains("flex_row"), "row constructor: {out}");
        assert!(out.contains(". px (gpui :: px (8f32))"), "px_8: {out}");
        assert!(out.contains(". gap (gpui :: px (4f32))"), "gap_4: {out}");
        assert!(out.contains(". bg (gpui :: Hsla {"), "bg color: {out}");
        assert!(out.contains("child"), "child appended: {out}");
    }

    // ── Runtime-cost guarantees of the generated code ─────────────────────────

    #[test]
    fn unstyled_text_emits_no_wrapper_div() {
        // Strings are IntoElement; a wrapper div would add a layout node.
        assert_eq!(expand(r#"text("hi")"#), "\"hi\"");
        let out = expand(r#"col() { text(item.name) }"#);
        assert!(out.contains("child (item . name)"), "{out}");
        // Styled text still gets the (required) wrapper.
        assert!(expand(r#"text("hi" text_sm)"#).contains("gpui :: div ()"));
    }

    #[test]
    fn list_does_not_allocate_an_arc() {
        let out = expand(r#"list(count = n, render = |ix| item(ix))"#);
        assert!(
            !out.contains("Arc"),
            "render must be moved, not Arc'd: {out}"
        );
        assert!(out.contains("uniform_list"), "{out}");
    }

    #[test]
    fn args_rebind_so_id_can_change_the_element_type() {
        // Regression (found by the GPUI integration crate): `.id()` turns Div
        // into Stateful<Div>. With `__el = __el.id(..)` reassignment that was
        // a type error. Args must rebind; children then mutate a single
        // binding. (Stateful-requiring args like on_click now fold the id
        // into the constructor instead — covered by the stateful_id tests —
        // so the mid-chain rebind is exercised with a plain id here.)
        let out = expand(r#"div(id = "x" cursor_pointer) { text("y") }"#);
        assert!(out.contains(r#"let __el = __el . id ("x")"#), "{out}");
        assert!(out.contains("let __el = __el . cursor_pointer ()"), "{out}");
        assert!(out.contains("let mut __el = __el ;"), "{out}");
        assert!(out.contains("__el = __el . child"), "{out}");
    }

    #[test]
    fn explicit_id_folds_into_constructor_with_on_click() {
        let out = expand(r#"div(id = "x" on_click = h) { text("y") }"#);
        assert!(out.contains(r#". id ("x")"#), "{out}");
        // Folded once — not also rebound in the args.
        assert!(!out.contains(r#"let __el = __el . id"#), "{out}");
        assert!(out.contains("let __el = __el . on_click (h)"), "{out}");
    }

    #[test]
    fn scroll_is_not_boxed() {
        let out = expand(r#"scroll(h_full) { div() }"#);
        assert!(
            !out.contains("into_any_element"),
            "scroll must yield its concrete type: {out}"
        );
        assert!(out.contains("overflow_y_scroll"), "{out}");
    }

    #[test]
    fn control_flow_expands_in_child_context() {
        let out = expand(
            r#"col() {
                if cond { text("a") } else { text("b") }
                for i in items { text("c") }
                match state { Some(x) => text("d"), None => {} }
            }"#,
        );
        assert!(out.contains("if cond"), "{out}");
        assert!(out.contains("else"), "{out}");
        assert!(out.contains("for i in items"), "{out}");
        assert!(out.contains("match state"), "{out}");
        // Leaves inside a parent must append via .child()
        assert!(out.contains("__el = __el . child"), "{out}");
    }

    #[test]
    fn stateful_id_injected_for_hover() {
        let out = expand(r#"div(on_hover = handler) {}"#);
        assert!(out.contains(". id ("), "auto id for on_hover: {out}");
    }

    #[test]
    fn stateful_id_injected_for_click_tokens_and_call_style() {
        // on_click (key = value) — the old README footgun.
        let out = expand(r#"div(on_click = handler) {}"#);
        assert!(out.contains(". id ("), "auto id for on_click: {out}");
        // overflow scroll tokens need a stateful element.
        let out = expand(r#"div(overflow_x_scroll w_128) {}"#);
        assert!(
            out.contains(". id ("),
            "auto id for overflow_x_scroll: {out}"
        );
        // Call-style stateful style refinement.
        let out = expand(r#"div(active(|s| s)) {}"#);
        assert!(out.contains(". id ("), "auto id for active(): {out}");
        // Plain styling must NOT inject an id.
        let out = expand(r#"div(px_4 hover(|s| s)) {}"#);
        assert!(!out.contains(". id ("), "no id for plain styling: {out}");
    }

    #[test]
    fn when_with_tokens_expands_inside_closure() {
        let out = expand(r#"div(when(dark, bg_1c1a17, text_f5f0e8)) {}"#);
        assert!(out.contains("when (dark , | __el |"), "{out}");
        assert!(out.contains("bg (gpui :: Hsla {"), "{out}");
        assert!(out.contains("text_color (gpui :: Hsla {"), "{out}");
        // key = value and call-style work inside when too.
        let out = expand(r#"div(when(dark, bg = th.panel, hover(|s| s))) {}"#);
        assert!(out.contains("bg (th . panel)"), "{out}");
        assert!(out.contains("hover (| s | s)"), "{out}");
    }

    #[test]
    fn when_closure_form_passes_through() {
        let out = expand(r#"div(when(show, |el| el.opacity(0.5))) {}"#);
        assert!(
            out.contains("when (show , | el | el . opacity (0.5))"),
            "{out}"
        );
        // Bare non-token path stays a callback, not a token.
        let out = expand(r#"div(when(show, my_modifier)) {}"#);
        assert!(out.contains("when (show , my_modifier)"), "{out}");
    }

    #[test]
    fn when_token_form_rejects_unknown_tokens() {
        let out = expand(r#"div(when(dark, bg_red, wobble_9)) {}"#);
        assert!(out.contains("Unknown style token"), "{out}");
    }

    #[test]
    fn ui_expand_returns_pretty_source_literal() {
        let out = ui_expand_impl(r#"row(px_8) { text("hi") }"#.parse().unwrap()).to_string();
        // It's a string literal, not element-building code.
        assert!(out.starts_with('"'), "{out}");
        assert!(out.contains("let __el"), "{out}");
        assert!(out.contains("px"), "{out}");
        // Pretty-printed: multi-line, wrapper fn stripped.
        assert!(out.contains("\\n"), "{out}");
        assert!(!out.contains("__ui_expand"), "{out}");
    }

    #[test]
    fn unknown_style_token_is_compile_error() {
        let out = expand(r#"div(wobble_9)"#);
        assert!(out.contains("Unknown style token"), "{out}");
    }

    #[test]
    fn parse_error_becomes_compile_error() {
        let out = expand(r#"div( %% )"#);
        assert!(out.contains("compile_error"), "{out}");
    }

    #[test]
    fn commas_between_args_are_accepted() {
        let with = expand(r#"row(px_8, gap_4, id = "x",) {}"#);
        let without = expand(r#"row(px_8 gap_4 id = "x") {}"#);
        assert_eq!(with, without);
    }

    #[test]
    fn unknown_element_keeps_explicit_id_with_hover() {
        // Regression: `id` used to be marked "consumed by the constructor" and
        // then dropped entirely, because unknown constructors never emit it.
        let out = expand(r#"custom_widget(id = "x", on_hover = h)"#);
        assert!(out.contains(r#"id ("x")"#), "id must survive: {out}");
    }

    // ── Path constructors and positional constructor args ─────────────────

    #[test]
    fn path_constructor_with_positional_and_builder_args() {
        let out = expand(r#"Button::new("ok", label("Go"), primary(), on_click = h)"#);
        assert!(out.contains(r#"Button :: new ("ok")"#), "{out}");
        assert!(out.contains(r#". label ("Go")"#), "{out}");
        assert!(out.contains(". primary ()"), "{out}");
        assert!(out.contains(". on_click (h)"), "{out}");
    }

    #[test]
    fn path_constructor_takes_style_tokens() {
        // Anything implementing Styled accepts compile-time tokens.
        let out = expand(r#"Button::new("ok", w_full)"#);
        assert!(out.contains(r#"Button :: new ("ok")"#), "{out}");
        assert!(out.contains(". w_full ()"), "{out}");
    }

    #[test]
    fn path_constructor_with_children() {
        let out = expand(r#"Badge::new("b") { text("3") }"#);
        assert!(out.contains(r#"Badge :: new ("b")"#), "{out}");
        assert!(out.contains(r#"child ("3")"#), "{out}");
    }

    #[test]
    fn unknown_ident_constructor_takes_leading_args() {
        let out = expand(r#"badge("hi", px_4)"#);
        assert!(out.contains(r#"badge ("hi")"#), "{out}");
        assert!(out.contains(". px (gpui :: px (4f32))"), "{out}");
    }

    #[test]
    fn constructor_args_can_be_paths_calls_and_method_calls() {
        // Path-callee calls, references, and method calls in leading
        // position are constructor args, not builder calls.
        let out = expand(r#"Input::new(&self.state, Thing::default(), self.id.clone())"#);
        assert!(
            out.contains("Input :: new (& self . state , Thing :: default () , self . id . clone ())"),
            "{out}"
        );
    }

    #[test]
    fn constructor_arg_after_builder_arg_is_error() {
        let out = expand(r#"Button::new(px_4, "ok")"#);
        assert!(
            out.contains("constructor arguments must come before"),
            "{out}"
        );
    }

    #[test]
    fn builtin_containers_still_reject_positional_args() {
        // The positional-arg rule is for custom constructors only.
        let out = expand(r#"div("x")"#);
        assert!(out.contains("unsupported element argument"), "{out}");
    }

    #[test]
    fn text_without_content_omits_child_call() {
        // Regression: used to emit `.child()` with no argument.
        let out = expand(r#"text(text_sm)"#);
        assert!(!out.contains("child ()"), "{out}");
    }

    #[test]
    fn block_after_text_becomes_a_sibling() {
        // text provably has no children, so a `{ ... }` after it belongs to
        // the parent. This used to be a compile error forcing a wrapper div.
        let out = expand(r#"row() { text("label") { some_sibling } }"#);
        assert!(!out.contains("compile_error"), "{out}");
        assert!(out.contains(r#"child ("label")"#), "{out}");
        assert!(out.contains("child (some_sibling)"), "{out}");
    }

    #[test]
    fn block_after_list_becomes_a_sibling() {
        // Regression: `list` never emits children, so a `{ ... }` after it
        // used to be parsed as children and silently dropped.
        let out = expand(r#"col() { list(count = n, render = r) { some_sibling } }"#);
        assert!(out.contains("child (some_sibling)"), "{out}");
    }

    #[test]
    fn spread_appends_via_children() {
        let out = expand(r#"col() { { ..maybe_badge } }"#);
        assert!(out.contains("children (maybe_badge)"), "{out}");
        // Inside control flow in child position too.
        let out = expand(r#"col() { if cond { { ..items.iter().map(row) } } }"#);
        assert!(
            out.contains("children (items . iter () . map (row))"),
            "{out}"
        );
    }

    #[test]
    fn spread_at_top_level_is_a_compile_error() {
        let out = expand(r#"{ ..items }"#);
        assert!(out.contains("compile_error"), "{out}");
        assert!(out.contains("parent element"), "{out}");
    }

    #[test]
    fn if_let_expands_with_multi_node_body() {
        let out = expand(r#"col() { if let Some(m) = modal { { backdrop } { m } } }"#);
        assert!(out.contains("if let Some (m) = modal"), "{out}");
        assert!(out.contains("child (backdrop)"), "{out}");
        assert!(out.contains("child (m)"), "{out}");
    }

    #[test]
    fn else_if_let_chains() {
        let out = expand(
            r#"col() {
                if let Some(e) = error { text(e) }
                else if let Some(w) = warning { text(w) }
                else { text("ok") }
            }"#,
        );
        assert!(out.contains("if let Some (e) = error"), "{out}");
        assert!(out.contains("else"), "{out}");
        assert!(out.contains("if let Some (w) = warning"), "{out}");
    }

    #[test]
    fn list_rejects_unknown_style_token() {
        // Regression: unknown tokens were silently dropped inside `list`.
        let out = expand(r#"list(count = n, wobble_9)"#);
        assert!(out.contains("Unknown style token"), "{out}");
    }

    #[test]
    fn text_element_supports_call_args() {
        // Regression: call-style args were silently ignored inside `text`.
        let out = expand(r#"text("hi", opacity(0.5))"#);
        assert!(out.contains("opacity (0.5)"), "{out}");
    }

    #[test]
    fn color_wins_over_numeric_for_ambiguous_tokens() {
        // Documented precedence: a 3/6-digit hex suffix is a color, anything
        // else numeric is a size.
        let color = token_to_direct_call("text_112").unwrap().to_string();
        assert!(color.contains("text_color"), "{color}");
        let size = token_to_direct_call("text_16").unwrap().to_string();
        assert!(size.contains("text_size"), "{size}");
    }

    // ── color! ─────────────────────────────────────────────────────────────────

    fn expand_color(src: &str) -> String {
        color_impl(src.parse().expect("input must tokenize")).to_string()
    }

    #[test]
    fn color_macro_accepts_all_input_forms() {
        // Bare hex (lexes as int-with-suffix or ident), #-prefixed, quoted,
        // alpha forms, named colors — all produce an Hsla struct literal.
        for src in [
            "1c1a17",
            "#1c1a17",
            "\"#1c1a17\"",
            "\"f5f0e8\"",
            "abc",
            "abcd",
            "1c1a17cc",
            "red",
        ] {
            let out = expand_color(src);
            assert!(out.contains("gpui :: Hsla {"), "`{src}` → {out}");
            assert!(!out.contains("compile_error"), "`{src}` → {out}");
        }
    }

    #[test]
    fn color_macro_matches_style_token_expansion() {
        // color!(x) must be bit-identical to what bg_x embeds.
        let standalone = expand_color("ff0000");
        let via_token = token_to_direct_call("bg_ff0000").unwrap().to_string();
        assert!(
            via_token.contains(&standalone),
            "{standalone} vs {via_token}"
        );
    }

    #[test]
    fn color_macro_rejects_non_colors() {
        for src in ["wobble", "12", "12345", "\"\""] {
            let out = expand_color(src);
            assert!(out.contains("compile_error"), "`{src}` → {out}");
        }
    }

    /// Dump full expansions for a fixed corpus. Run before and after a refactor
    /// and diff the output:
    ///   cargo test dump_expansions -- --ignored --nocapture
    #[test]
    #[ignore]
    fn dump_expansions() {
        let corpus = [
            r#"row(px_8 gap_4 bg_1c1a17 rounded_lg shadow) { text("hi" text_sm semibold) }"#,
            r#"col(id = "main" on_hover = h on_mouse_down = m w_240) { div() }"#,
            r#"scroll(h_full) { for i in 0..10 { text("row") } }"#,
            r#"list(id = "l" count = n render = |ix| item(ix) px_4)"#,
            r#"center() { if a { text("a") } else if b { text("b") } else { text("c") } }"#,
            r#"card() { match x { Some(v) if v > 0 => text("pos"), _ => { text("other") } } }"#,
            r#"custom_widget(size(20) .with_flag() label = "x") { { any_expr() } }"#,
            r#"div() div()"#,
        ];
        for src in corpus {
            println!("=== {src}\n{}\n", expand(src));
        }
    }
}

/// Timing benches. In-crate because a `proc-macro` crate exports nothing but
/// its macros, so external `benches/` (criterion) cannot reach the internals.
/// Run with:
///   cargo test --release bench_ -- --ignored --nocapture
#[cfg(test)]
mod benches {
    use super::*;
    use std::time::Instant;

    fn time<F: FnMut()>(label: &str, iters: u32, mut f: F) {
        // Warmup
        for _ in 0..iters.min(100) {
            f();
        }
        let start = Instant::now();
        for _ in 0..iters {
            f();
        }
        let total = start.elapsed();
        println!(
            "{label:<40} {:>10.2} µs/iter   ({iters} iters, total {:?})",
            total.as_secs_f64() * 1e6 / iters as f64,
            total
        );
    }

    fn small_input() -> String {
        r#"row(px_8 gap_4 bg_1c1a17) { text("hello" text_sm) }"#.to_string()
    }

    fn large_input(n: usize) -> String {
        let mut s = String::from("col(gap_4 p_16 bg_1c1a17) {\n");
        for i in 0..n {
            s.push_str(&format!(
                r#"row(px_8 gap_4 rounded_lg shadow items_center on_hover = h{i}) {{
                    text("item" text_sm text_f5f0e8 semibold)
                    if cond{i} {{ text("on") }} else {{ text("off") }}
                    for x in 0..{i} {{ div(w_4 h_4 bg_red) }}
                    match state{i} {{ Some(v) => text("v"), None => {{}} }}
                }}
"#
            ));
        }
        s.push('}');
        s
    }

    #[test]
    #[ignore]
    fn bench_full_expansion() {
        let small: TokenStream2 = small_input().parse().unwrap();
        let large: TokenStream2 = large_input(200).parse().unwrap();

        time("parse+expand: small (1 element)", 10_000, || {
            std::hint::black_box(ui_impl(small.clone()));
        });
        time("parse+expand: large (200 rows)", 100, || {
            std::hint::black_box(ui_impl(large.clone()));
        });

        // Split parse vs emit for the large input to attribute the cost.
        time("parse only: large", 100, || {
            std::hint::black_box(syn::parse2::<Ui>(large.clone()).unwrap());
        });
        let ast = syn::parse2::<Ui>(large.clone()).unwrap();
        time("emit only: large", 100, || {
            let mut out = TokenStream2::new();
            emit_nodes(&mut out, &ast.nodes, Ctx::TopLevel);
            std::hint::black_box(out);
        });
    }

    #[test]
    #[ignore]
    fn bench_token_lookup() {
        let tokens = [
            "flex_1",
            "items_center",
            "rounded_br_2xl",
            "cursor_context_menu",
            "px_8",
            "gap_y_12",
            "grid_cols_3",
            "bg_1c1a17",
            "text_f5f0e8",
            "border_2d2b27",
            "opacity_50",
            "line_clamp_2",
            "definitely_not_a_token",
        ];
        time("token_to_direct_call (13 mixed)", 100_000, || {
            for t in tokens {
                std::hint::black_box(token_to_direct_call(t));
            }
        });
    }
}
