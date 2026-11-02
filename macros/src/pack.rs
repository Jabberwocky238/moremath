use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Expr, Ident, Lit, Token, UnOp};

use crate::field::{check_literal, fit_check, hex, mask, word_bits, BitRange, Claims};

pub struct Pack {
    word: Ident,
    fields: Vec<(BitRange, Expr)>,
}

impl Parse for Pack {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let word = input.parse::<Ident>()?;
        input.parse::<Token![,]>()?;

        let mut fields = Vec::new();
        while !input.is_empty() {
            let range = BitRange::parse_bracketed(input)?;
            input.parse::<Token![=]>()?;
            let value = input.parse::<Expr>()?;
            fields.push((range, value));
            if input.parse::<Option<Token![,]>>()?.is_none() {
                break;
            }
        }
        if !input.is_empty() {
            return Err(input.error("expected `[low..=high] = value`, comma separated"));
        }
        if fields.is_empty() {
            return Err(input.error("a word with no fields is `0`, ⛔ not a `pack!`"));
        }
        Ok(Self { word, fields })
    }
}

fn literal(expr: &Expr) -> Option<i128> {
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Int(int) => int.base10_parse::<i128>().ok(),
            _ => None,
        },
        Expr::Unary(unary) if matches!(unary.op, UnOp::Neg(_)) => {
            literal(&unary.expr).map(|value| -value)
        }
        Expr::Group(group) => literal(&group.expr),
        Expr::Paren(paren) => literal(&paren.expr),
        _ => None,
    }
}

pub fn expand(pack: Pack) -> syn::Result<TokenStream> {
    let Pack { word, fields } = pack;
    let bits = word_bits(&word)?;

    let mut claims = Claims::new(bits);
    let mut pieces = Vec::new();
    for (range, value) in &fields {
        let top = range.top(bits)?;
        let width = range.width(bits)?;
        let spelled = range.spelled(bits);
        claims.claim(&spelled, range.lo, top, range.span)?;

        let what = format!("pack![{word}, {spelled} = {}]", quote!(#value));
        let placed = if let Some(constant) = literal(value) {
            check_literal(constant, width, value.span(), &spelled)?;
            #[allow(clippy::cast_sign_loss, reason = "补码的低 width 位就是硬件读到的")]
            let masked = (constant as u128) & mask(width);
            hex(masked << range.lo, &word)
        } else {
            let checked = fit_check(&quote!(#value), width, &word, &what);
            if range.lo == 0 {
                checked
            } else {
                let lo = range.lo;
                quote! { (#checked) << #lo }
            }
        };
        pieces.push(quote! { word |= #placed; });
    }

    Ok(quote! {
        {
            let mut word: #word = 0;
            #(#pieces)*
            word
        }
    })
}
