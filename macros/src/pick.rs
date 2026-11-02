use proc_macro2::{Spacing, Span, TokenStream, TokenTree};
use quote::quote;

pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    if tokens.is_empty() {
        return Err(syn::Error::new(
            Span::call_site(),
            "pick!(cond ? then : otherwise) —— 它是空的",
        ));
    }

    let Some(question) = tokens.iter().position(is_question) else {
        return Err(syn::Error::new(
            span_of(&tokens),
            "pick!(cond ? then : otherwise): 没找到顶层的 `?`",
        ));
    };
    let Some(colon) = separator(&tokens[question + 1..]).map(|at| question + 1 + at) else {
        return Err(syn::Error::new(
            span_of(&tokens),
            "pick!(cond ? then : otherwise): `?` 之后没找到顶层的 `:`\n\
             \x20 ⚠ `::` 不算 —— 它是一个路径分隔符，⛔ 不是这里的那个冒号",
        ));
    };

    let cond = part(&tokens[..question], "条件", &tokens)?;
    let then = part(&tokens[question + 1..colon], "`?` 之后那条臂", &tokens)?;
    let otherwise = part(&tokens[colon + 1..], "`:` 之后那条臂", &tokens)?;

    Ok(quote! { if #cond { #then } else { #otherwise } })
}

fn is_question(token: &TokenTree) -> bool {
    matches!(token, TokenTree::Punct(punct) if punct.as_char() == '?')
}

fn separator(tokens: &[TokenTree]) -> Option<usize> {
    let mut at = 0;
    while at < tokens.len() {
        if let TokenTree::Punct(punct) = &tokens[at]
            && punct.as_char() == ':'
        {
            if punct.spacing() == Spacing::Joint {
                at += 2;
                continue;
            }
            return Some(at);
        }
        at += 1;
    }
    None
}

fn part(tokens: &[TokenTree], what: &str, all: &[TokenTree]) -> syn::Result<TokenStream> {
    if tokens.is_empty() {
        return Err(syn::Error::new(
            span_of(all),
            format!("pick!(cond ? then : otherwise): {what}是空的"),
        ));
    }
    Ok(tokens.iter().cloned().collect())
}

fn span_of(tokens: &[TokenTree]) -> Span {
    tokens
        .first()
        .map_or_else(Span::call_site, TokenTree::span)
}
