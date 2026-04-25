use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{braced, parenthesized, Expr, Ident, Token, Result};

#[proc_macro]
pub fn ui(input: TokenStream) -> TokenStream {
    let ui_ast = match syn::parse::<Ui>(input) {
        Ok(ast) => ast,
        Err(err) => return err.to_compile_error().into(),
    };
    
    let expanded = if ui_ast.nodes.len() == 1 {
        let node = &ui_ast.nodes[0];
        expand_node(node)
    } else {
        let nodes = &ui_ast.nodes;
        expand_nodes(nodes)
    };
    
    TokenStream::from(expanded)
}

struct Ui {
    nodes: Vec<UiNode>,
}

impl Parse for Ui {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut nodes = Vec::new();
        while !input.is_empty() {
            nodes.push(input.parse()?);
        }
        Ok(Ui { nodes })
    }
}

struct IfNode {
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
    guard: Option<(Token![if], Box<Expr>)>,
    body: Vec<UiNode>,
}

struct MatchNode {
    expr: Expr,
    arms: Vec<MatchArmNode>,
}

enum UiNode {
    Block(Expr),
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
            let expr: Expr = content.parse()?;
            return Ok(UiNode::Block(expr));
        }

        if input.peek(Token![if]) {
            input.parse::<Token![if]>()?;
            let cond = syn::Expr::parse_without_eager_brace(input)?;
            let then_content;
            braced!(then_content in input);
            let mut then_branch = Vec::new();
            while !then_content.is_empty() {
                then_branch.push(then_content.parse()?);
            }
            
            let mut else_branch = None;
            if input.peek(Token![else]) {
                input.parse::<Token![else]>()?;
                if input.peek(Token![if]) {
                    else_branch = Some(vec![input.parse::<UiNode>()?]);
                } else {
                    let else_content;
                    braced!(else_content in input);
                    let mut else_nodes = Vec::new();
                    while !else_content.is_empty() {
                        else_nodes.push(else_content.parse()?);
                    }
                    else_branch = Some(else_nodes);
                }
            }
            return Ok(UiNode::If(IfNode { cond, then_branch, else_branch }));
        }

        if input.peek(Token![for]) {
            input.parse::<Token![for]>()?;
            let pat = syn::Pat::parse_multi_with_leading_vert(input)?;
            input.parse::<Token![in]>()?;
            let iter = syn::Expr::parse_without_eager_brace(input)?;
            let body_content;
            braced!(body_content in input);
            let mut body = Vec::new();
            while !body_content.is_empty() {
                body.push(body_content.parse()?);
            }
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
                    let if_token = content.parse::<Token![if]>()?;
                    let guard_expr = content.parse::<Expr>()?;
                    Some((if_token, Box::new(guard_expr)))
                } else {
                    None
                };
                content.parse::<Token![=>]>()?;
                
                // Usually body is wrapped in `{ ... }` but in our DSL we want native nodes!
                // Wait, if it's wrapped in `{ ... }`, we can just parse it as an expression? No, we parse it as a block of UiNodes!
                // Wait, rust allows `pat => expr,` or `pat => { ... }`.
                // In our DSL, let's force `{ ... }` or just parse multiple UiNodes until `,` or `}`?
                // Let's expect `{ ... }` for the arm body.
                let arm_content;
                let is_braced = content.peek(syn::token::Brace);
                if is_braced {
                    braced!(arm_content in content);
                    let mut body = Vec::new();
                    while !arm_content.is_empty() {
                        body.push(arm_content.parse()?);
                    }
                    if content.peek(Token![,]) {
                        content.parse::<Token![,]>()?;
                    }
                    arms.push(MatchArmNode { pat, guard, body });
                } else {
                    // Single node until comma
                    let mut body = Vec::new();
                    body.push(content.parse()?);
                    if content.peek(Token![,]) {
                        content.parse::<Token![,]>()?;
                    }
                    arms.push(MatchArmNode { pat, guard, body });
                }
            }
            return Ok(UiNode::Match(MatchNode { expr, arms }));
        }

        let el: Element = input.parse()?;
        Ok(UiNode::Element(el))
    }
}

fn expand_node(node: &UiNode) -> TokenStream2 {
    match node {
        UiNode::Block(expr) => quote! { #expr },
        UiNode::If(if_node) => {
            let cond = &if_node.cond;
            let then_tokens = expand_nodes(&if_node.then_branch);
            let else_tokens = if let Some(else_branch) = &if_node.else_branch {
                let else_inner = expand_nodes(else_branch);
                quote! { else { #else_inner } }
            } else {
                quote! {}
            };
            quote! {
                if #cond {
                    #then_tokens
                } #else_tokens
            }
        }
        UiNode::For(for_node) => {
            let pat = &for_node.pat;
            let iter = &for_node.iter;
            let body_tokens = expand_nodes(&for_node.body);
            quote! {
                for #pat in #iter {
                    #body_tokens
                }
            }
        }
        UiNode::Match(match_node) => {
            let expr = &match_node.expr;
            let mut arms_tokens = TokenStream2::new();
            for arm in &match_node.arms {
                let pat = &arm.pat;
                let guard = if let Some((_, g_expr)) = &arm.guard {
                    quote! { if #g_expr }
                } else {
                    quote! {}
                };
                let body_tokens = expand_nodes(&arm.body);
                arms_tokens.extend(quote! {
                    #pat #guard => {
                        #body_tokens
                    }
                });
            }
            quote! {
                match #expr {
                    #arms_tokens
                }
            }
        }
        UiNode::Element(el) => {
            let mut tokens = TokenStream2::new();
            el.to_tokens(&mut tokens);
            tokens
        }
    }
}

fn expand_nodes(nodes: &[UiNode]) -> TokenStream2 {
    let mut tokens = quote! {};
    for node in nodes {
        tokens.extend(expand_node(node));
    }
    tokens
}

struct Element {
    name: Ident,
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
            let expr: Expr = input.parse()?;
            Ok(Arg::Expr(expr))
        }
    }
}

impl Parse for Element {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        
        let mut args = Vec::new();
        if input.peek(syn::token::Paren) {
            let content;
            parenthesized!(content in input);
            while !content.is_empty() {
                args.push(content.parse()?);
            }
        }
        
        let mut children = Vec::new();
        if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            while !content.is_empty() {
                children.push(content.parse()?);
            }
        }
        
        Ok(Element { name, args, children })
    }
}

impl ToTokens for Element {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;
        let name_str = name.to_string();
        
        if name_str == "text" || name_str == "text_raw" || name_str == "label" {
            let mut el_tokens = quote! { let mut el = declarative_ui::styled_div(""); };
            let mut content = quote! {};
            
            for (i, arg) in self.args.iter().enumerate() {
                if i == 0 {
                    if let Arg::Expr(expr) = arg {
                        content = quote! { #expr };
                        continue;
                    }
                }
                match arg {
                    Arg::Expr(syn::Expr::Path(path)) => {
                        let path_str = path.path.get_ident().map(|i| i.to_string()).unwrap_or_default();
                        if !is_valid_style_token(&path_str) {
                            let err = syn::Error::new_spanned(path, format!("Unknown style token `{}`", path_str)).to_compile_error();
                            el_tokens.extend(err);
                        } else {
                            el_tokens.extend(quote! {
                                el = declarative_ui::apply_style_token(el, stringify!(#path));
                            });
                        }
                    }
                    _ => {}
                }
            }
            
            tokens.extend(quote! {
                {
                    #el_tokens
                    el.child(#content)
                }
            });
            return;
        }

        let constructor = match name_str.as_str() {
            "row" => quote! { declarative_ui::row() },
            "col" => quote! { declarative_ui::col() },
            "center" => quote! { declarative_ui::center() },
            "card" => quote! { declarative_ui::card() },
            "div" => quote! { declarative_ui::styled_div("") },
            "scroll" => {
                // Point 2 & 6: Stateful<Div> incompatibility
                quote! {
                    declarative_ui::styled_div("")
                        .id(concat!(file!(), ":", line!(), ":", column!()))
                        .overflow_y_scroll()
                }
            }
            "list" => {
                // Actually emit a list
                quote! { declarative_ui::styled_div("") } 
            }
            _ => quote! { #name() }, // Fallback to calling a function
        };

        let mut args_tokens = quote! {};
        for arg in &self.args {
            match arg {
                Arg::KeyValue(key, val) => {
                    let key_str = key.to_string();
                    if key_str == "class" {
                        // Point 5: Dynamic Style Strings
                        args_tokens.extend(quote! {
                            el = declarative_ui::apply_styles(el, #val);
                        });
                    } else if key_str == "id" {
                        args_tokens.extend(quote! { el = el.id(#val); });
                    } else if key_str.starts_with("on_") {
                        // For event handlers, we delegate back to `declarative_ui::ui_apply_args!` 
                        // so it can wrap them properly.
                        args_tokens.extend(quote! {
                            declarative_ui::ui_apply_args!(el, #key = #val);
                        });
                    } else {
                        // Generic property
                        args_tokens.extend(quote! {
                            el = el.#key(#val);
                        });
                    }
                }
                Arg::Expr(syn::Expr::Path(path)) => {
                    // Style token
                    let path_ident = path.path.get_ident().unwrap();
                    let path_str = path_ident.to_string();
                    if !is_valid_style_token(&path_str) {
                        let err = syn::Error::new_spanned(path, format!("Unknown style token `{}`", path_str)).to_compile_error();
                        args_tokens.extend(err);
                    } else {
                        args_tokens.extend(quote! {
                            el = declarative_ui::apply_style_token(el, stringify!(#path_ident));
                        });
                    }
                }
                Arg::Expr(expr) => {
                    // Allow arbitrary GPUI builder methods like `w(px(300.))`
                    if let syn::Expr::Call(call) = &expr {
                        let func = &call.func;
                        let args = &call.args;
                        args_tokens.extend(quote! {
                            el = el.#func(#args);
                        });
                    } else if let syn::Expr::MethodCall(method) = &expr {
                        let method_name = &method.method;
                        let args = &method.args;
                        args_tokens.extend(quote! {
                            el = el.#method_name(#args);
                        });
                    }
                }
            }
        }

        fn push_children(children: &[UiNode]) -> TokenStream2 {
            let mut tokens = quote! {};
            for child in children {
                match child {
                    UiNode::Element(el_child) => {
                        tokens.extend(quote! {
                            el = el.child( #el_child );
                        });
                    }
                    UiNode::Block(expr) => {
                        tokens.extend(quote! {
                            el = el.child( #expr );
                        });
                    }
                    UiNode::If(if_node) => {
                        let cond = &if_node.cond;
                        let then_tokens = push_children(&if_node.then_branch);
                        let else_tokens = if let Some(else_branch) = &if_node.else_branch {
                            let else_inner = push_children(else_branch);
                            quote! { else { #else_inner } }
                        } else {
                            quote! {}
                        };
                        tokens.extend(quote! {
                            if #cond {
                                #then_tokens
                            } #else_tokens
                        });
                    }
                    UiNode::For(for_node) => {
                        let pat = &for_node.pat;
                        let iter = &for_node.iter;
                        let body_tokens = push_children(&for_node.body);
                        tokens.extend(quote! {
                            for #pat in #iter {
                                #body_tokens
                            }
                        });
                    }
                    UiNode::Match(match_node) => {
                        let expr = &match_node.expr;
                        let mut arms_tokens = TokenStream2::new();
                        for arm in &match_node.arms {
                            let pat = &arm.pat;
                            let guard = if let Some((_, g_expr)) = &arm.guard {
                                quote! { if #g_expr }
                            } else {
                                quote! {}
                            };
                            let body_tokens = push_children(&arm.body);
                            arms_tokens.extend(quote! {
                                #pat #guard => {
                                    #body_tokens
                                }
                            });
                        }
                        tokens.extend(quote! {
                            match #expr {
                                #arms_tokens
                            }
                        });
                    }
                }
            }
            tokens
        }

        let children_tokens = push_children(&self.children);

        // For elements that are returning Stateful<Div>, we must call `.into_any_element()` if they are nested,
        // but if they are the root, they don't have to be.
        // Wait, to preserve GPUI type inference for `cx` in event handlers, we should NOT call `into_any_element()`
        // unless absolutely necessary. We will let standard nodes return `Div` or `Stateful<Div>`.
        let is_scroll = name_str == "scroll";
        let finalize = if is_scroll || name_str == "list" {
            quote! { el.into_any_element() }
        } else {
            quote! { el }
        };

        tokens.extend(quote! {
            {
                let mut el = #constructor;
                #args_tokens
                #children_tokens
                #finalize
            }
        });
    }
}

fn is_valid_style_token(token: &str) -> bool {
    let exact = [
        "shadow", "shadow_none", "shadow_2xs", "shadow_xs", "shadow_sm", "shadow_md", "shadow_lg", "shadow_xl", "shadow_2xl",
        "rounded", "rounded_none", "rounded_xs", "rounded_sm", "rounded_md", "rounded_lg", "rounded_xl", "rounded_2xl", "rounded_3xl", "rounded_full",
        "flex", "block", "grid", "hidden",
        "flex_col", "col", "flex_row", "row", "flex_wrap", "flex_wrap_reverse", "flex_nowrap",
        "flex_1", "flex_auto", "flex_initial", "flex_none", "flex_grow", "flex_shrink", "flex_shrink_0",
        "justify_start", "justify_end", "justify_center", "justify_between", "justify_around",
        "items_start", "items_end", "items_center", "items_baseline",
        "content_normal", "content_start", "content_end", "content_center", "content_between", "content_around", "content_evenly", "content_stretch",
        "font_bold", "bold", "font_semibold", "semibold", "font_medium", "medium",
        "italic", "not_italic", "underline", "line_through", "no_underline", "text_decoration_none",
        "cursor_pointer", "cursor_default", "cursor_text", "cursor_move", "cursor_not_allowed", "cursor_none",
        "visible", "invisible", "absolute", "relative",
        "overflow_hidden", "overflow_x_hidden", "overflow_y_hidden",
        "h_full", "w_full", "size_full",
        "text_left", "text_center", "text_right", "truncate", "text_ellipsis",
        "whitespace_nowrap", "whitespace_normal",
        "text_xs", "text_sm", "text_base", "text_lg", "text_xl", "text_2xl", "text_3xl",
        "border", "border_x", "border_y", "border_t", "border_b", "border_l", "border_r",
    ];
    if exact.contains(&token) {
        return true;
    }
    
    let prefixes = [
        "bg_", "text_", "border_", "gap_", "p_", "pt_", "pb_", "pl_", "pr_", "px_", "py_",
        "m_", "mt_", "mb_", "ml_", "mr_", "mx_", "my_", "w_", "h_", "size_",
        "min_w_", "max_w_", "min_h_", "max_h_", "top_", "bottom_", "left_", "right_", "inset_",
        "border_x_", "border_y_", "border_t_", "border_b_", "border_l_", "border_r_",
        "rounded_", "line_clamp_", "opacity_"
    ];
    for p in prefixes {
        if token.starts_with(p) {
            return true;
        }
    }
    
    false
}
