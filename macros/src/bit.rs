use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Token, Type};

use crate::field::BitRange;

pub struct Bits {
    value: Expr,
    range: BitRange,
    into: Option<Type>,
}

impl Parse for Bits {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let value = input.parse::<Expr>()?;
        input.parse::<Token![,]>()?;
        let range = BitRange::parse_bracketed(input)?;

        let into = if input.parse::<Option<Token![,]>>()?.is_some() && !input.is_empty() {
            let into = input.parse::<Type>()?;
            let _ = input.parse::<Token![,]>();
            Some(into)
        } else {
            None
        };
        if !input.is_empty() {
            return Err(input.error("expected `bit![value, [range]]` or `bit![value, [range], type]`"));
        }

        Ok(Self { value, range, into })
    }
}

pub fn expand(bits: Bits) -> TokenStream {
    let Bits { value, range, into } = bits;
    let low = range.lo;
    let high = range.hi;

    let width = high.map_or_else(
        || quote! { bits - #low },
        |high| {
            let width = high - low + 1;
            quote! { #width }
        },
    );
    let checked = high.map(|high| {
        quote! {
            assert!(
                #high < bits,
                "bit![_, [{}..={}]]: past the top of a {}-bit value",
                #low, #high, bits,
            );
        }
    });

    let field = quote! {
        {
            let value = #value;
            let bits = (::core::mem::size_of_val(&value) * 8) as u32;
            assert!(
                #low < bits,
                "bit![_, [{}..]]: past the top of a {}-bit value", #low, bits,
            );
            #checked
            let width = #width;
            let mask = if width >= bits { !0 } else { !(!0 << width) };
            (value >> #low) & mask
        }
    };

    match into {
        Some(into) => quote! {
            match ::core::convert::TryInto::<#into>::try_into(#field) {
                Ok(field) => field,
                Err(_) => panic!(concat!(
                    "bit![_, [..], ", stringify!(#into), "]: the field does not fit",
                )),
            }
        },
        None => field,
    }
}
