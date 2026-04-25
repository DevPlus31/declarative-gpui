use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{braced, parenthesized, Expr, Ident, Token, Result};

pub enum Arg {
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
