use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::ParseStream;
use syn::spanned::Spanned;
use syn::{bracketed, Ident, LitInt, Token};

pub struct BitRange {
    pub lo: u32,
    pub hi: Option<u32>,
    pub span: Span,
}

impl BitRange {
    pub fn parse_bracketed(input: ParseStream) -> syn::Result<Self> {
        let range;
        let bracket = bracketed!(range in input);
        let span = bracket.span.join();

        let low_literal = range.parse::<LitInt>()?;
        let lo = low_literal.base10_parse::<u32>()?;

        let hi = if range.peek(Token![..=]) {
            let dots = range.parse::<Token![..=]>()?;
            let high_literal = range.parse::<LitInt>()?;
            let high = high_literal.base10_parse::<u32>()?;
            if high < lo {
                return Err(syn::Error::new(
                    dots.span(),
                    format!(
                        "{lo}..={high} runs backwards: a datasheet's bits[{lo}:{high}] \
                         is written low bound first here, as [{high}..={lo}]",
                    ),
                ));
            }
            Some(high)
        } else if range.peek(Token![..]) {
            let dots = range.parse::<Token![..]>()?;
            if range.is_empty() {
                None
            } else {
                let high = range.parse::<LitInt>()?;
                let high = high.base10_parse::<u32>()?;
                return Err(syn::Error::new(
                    dots.span(),
                    format!(
                        "write the top bit, not one past it: `[{lo}..={}]` \
                         if bit {} is in the field, `[{lo}..={high}]` if it is not",
                        high.saturating_sub(1),
                        high.saturating_sub(1),
                    ),
                ));
            }
        } else {
            Some(lo)
        };

        if !range.is_empty() {
            return Err(range.error("expected `[low..=high]`, `[low..]` or `[bit]`"));
        }
        Ok(Self { lo, hi, span })
    }

    pub fn top(&self, word_bits: u32) -> syn::Result<u32> {
        let top = self.hi.unwrap_or(word_bits - 1);
        if top >= word_bits {
            return Err(syn::Error::new(
                self.span,
                format!("bits {}..={top} run past the {word_bits}-bit word", self.lo),
            ));
        }
        Ok(top)
    }

    pub fn width(&self, word_bits: u32) -> syn::Result<u32> {
        Ok(self.top(word_bits)? - self.lo + 1)
    }

    pub fn spelled(&self, word_bits: u32) -> String {
        match self.hi {
            Some(hi) => format!("[{}..={hi}]", self.lo),
            None => format!("[{}..={}]", self.lo, word_bits - 1),
        }
    }
}

pub fn word_bits(ty: &Ident) -> syn::Result<u32> {
    match ty.to_string().as_str() {
        "u8" => Ok(8),
        "u16" => Ok(16),
        "u32" => Ok(32),
        "u64" => Ok(64),
        other => Err(syn::Error::new(
            ty.span(),
            format!("the word is `u8`, `u16`, `u32` or `u64`, ⛔ not `{other}`"),
        )),
    }
}

pub struct Claims {
    owner: Vec<Option<String>>,
}

impl Claims {
    pub fn new(word_bits: u32) -> Self {
        Self { owner: vec![None; word_bits as usize] }
    }

    pub fn claim(&mut self, name: &str, lo: u32, top: u32, span: Span) -> syn::Result<()> {
        for bit in lo..=top {
            if let Some(other) = &self.owner[bit as usize] {
                return Err(syn::Error::new(
                    span,
                    format!("bit {bit} is already claimed by `{other}`"),
                ));
            }
            self.owner[bit as usize] = Some(name.to_owned());
        }
        Ok(())
    }
}

pub fn mask(width: u32) -> u128 {
    if width >= 128 { u128::MAX } else { (1u128 << width) - 1 }
}

pub fn hex(value: u128, word: &Ident) -> TokenStream {
    let digits = format!("{value:x}");
    let grouped: Vec<String> = digits
        .as_bytes()
        .rchunks(4)
        .rev()
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect();
    let spelled = format!("0x{}_{word}", grouped.join("_"));
    spelled.parse().expect("十六进制字面量")
}

pub fn admits(width: u32) -> (i128, i128) {
    let unsigned_top = (1i128 << width) - 1;
    let signed_floor = -(1i128 << (width - 1));
    (signed_floor, unsigned_top)
}

pub fn check_literal(value: i128, width: u32, span: Span, what: &str) -> syn::Result<()> {
    let (floor, ceiling) = admits(width);
    if value < floor || value > ceiling {
        return Err(syn::Error::new(
            span,
            format!(
                "{value} 放不进 {what} 这 {width} 位（收 {floor}..={ceiling}）",
            ),
        ));
    }
    Ok(())
}

pub fn fit_check(value: &TokenStream, width: u32, word: &Ident, what: &str) -> TokenStream {
    let bound = hex(mask(width), word);
    let floor_shift = width - 1;
    let message = format!("{what}: 值放不进这 {width} 位")
        .replace('{', "{{")
        .replace('}', "}}");
    quote! {
        {
            #[allow(
                clippy::cast_lossless,
                clippy::cast_possible_truncation,
                clippy::cast_possible_wrap,
                clippy::cast_sign_loss,
                reason = "这一步就是那道闸：先无损进 i128，判过了再落到字宽"
            )]
            let wide = (#value) as i128;
            ::core::assert!(wide >= -(1i128 << #floor_shift), #message);
            ::core::assert!(wide < 1i128 << #width, #message);
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "上面那条 assert 就是这次窄化的凭证；负值取补码低位，正是硬件读到的"
            )]
            let narrowed = (wide as #word) & #bound;
            narrowed
        }
    }
}
