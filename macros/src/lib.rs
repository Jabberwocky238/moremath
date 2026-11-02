#![allow(clippy::too_many_lines)]

use proc_macro::TokenStream;

mod bit;
mod field;
mod pack;
mod pick;

#[proc_macro]
pub fn bit(input: TokenStream) -> TokenStream {
    match syn::parse::<bit::Bits>(input) {
        Ok(bits) => bit::expand(bits).into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro]
pub fn pack(input: TokenStream) -> TokenStream {
    match syn::parse::<pack::Pack>(input) {
        Ok(parsed) => match pack::expand(parsed) {
            Ok(tokens) => tokens.into(),
            Err(error) => error.to_compile_error().into(),
        },
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro]
pub fn pick(input: TokenStream) -> TokenStream {
    match pick::expand(input.into()) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
