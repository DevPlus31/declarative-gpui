use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{braced, parenthesized, Expr, Ident, Token, Pat, Result};

pub struct Ui {
    pub nodes: Vec<UiNode>,
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

pub enum UiNode {
    Block(Expr),
    If(syn::ExprIf),
    For(syn::ExprForLoop),
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
            let if_expr: syn::ExprIf = input.parse()?;
            return Ok(UiNode::If(if_expr));
        }

        if input.peek(Token![for]) {
            let for_expr: syn::ExprForLoop = input.parse()?;
            return Ok(UiNode::For(for_expr));
        }

        let el: Element = input.parse()?;
        Ok(UiNode::Element(el))
    }
}

pub struct Element {
    pub name: Ident,
    pub args: Vec<Arg>,
    pub children: Vec<UiNode>,
}

pub enum Arg {
    KeyValue(Ident, Expr),
    Style(Ident),
}

impl Parse for Element {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        
        let mut args = Vec::new();
        if input.peek(syn::token::Paren) {
            let content;
            parenthesized!(content in input);
            while !content.is_empty() {
                if content.peek(Ident) && content.peek2(Token![=]) {
                    let key: Ident = content.parse()?;
                    content.parse::<Token![=]>()?;
                    let val: Expr = content.parse()?;
                    args.push(Arg::KeyValue(key, val));
                } else if content.peek(Ident) {
                    let key: Ident = content.parse()?;
                    args.push(Arg::Style(key));
                } else {
                    return Err(content.error("expected identifier or key=value"));
                }
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
