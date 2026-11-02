use crate::FromWith;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
#[allow(non_camel_case_types, reason = "与原生浮点类型（f32 / f64）同一种写法：它就是一种数")]
pub struct bf16(pub u16);

impl From<f32> for bf16 {
    fn from(value: f32) -> Self {
        let bits = value.to_bits();
        let low = bits & 0xffff;
        #[allow(clippy::cast_possible_truncation, reason = "the top 16 bits, by design")]
        let high = (bits >> 16) as u16;
        if value.is_nan() {
            return Self(high | 0x0040);
        }
        let round_up = match low.cmp(&0x8000) {
            core::cmp::Ordering::Less => false,
            core::cmp::Ordering::Greater => true,
            core::cmp::Ordering::Equal => high & 1 == 1,
        };
        Self(high.wrapping_add(u16::from(round_up)))
    }
}

impl From<bf16> for f32 {
    fn from(value: bf16) -> Self {
        Self::from_bits(u32::from(value.0) << 16)
    }
}

impl From<bf16> for f64 {
    fn from(value: bf16) -> Self {
        Self::from(f32::from(value))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TowardZero;

impl FromWith<f64, TowardZero> for bf16 {
    #[allow(clippy::cast_possible_truncation, reason = "先舍到 f32、按大小回退一格，就是向零截断")]
    fn from_with(value: f64, _: TowardZero) -> Self {
        if value.is_nan() {
            return Self::NAN;
        }
        let mut single = value as f32;
        if f64::from(single).abs() > value.abs() {
            single = f32::from_bits(single.to_bits() - 1);
        }
        Self((single.to_bits() >> 16) as u16)
    }
}

impl bf16 {
    pub const NAN: Self = Self(0x7fc0);
    pub const INFINITY: Self = Self(0x7f80);
    pub const ONE: Self = Self(0x3f80);
}

impl core::ops::Neg for bf16 {
    type Output = Self;

    fn neg(self) -> Self {
        Self(self.0 ^ 0x8000)
    }
}

macro_rules! arithmetic {
    ($($trait:ident $method:ident $op:tt;)+) => {$(
        impl core::ops::$trait for bf16 {
            type Output = Self;

            fn $method(self, rhs: Self) -> Self {
                crate::transquant!(f64::from(self) $op f64::from(rhs), f64 to f32 to bf16)
            }
        }
    )+};
}

arithmetic! {
    Add add +;
    Sub sub -;
    Mul mul *;
    Div div /;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bf16_rounds_half_to_even() {
        assert_eq!(bf16::from(f32::from_bits(0x3f80_7fff)).0, 0x3f80);
        assert_eq!(bf16::from(f32::from_bits(0x3f80_8001)).0, 0x3f81);
        assert_eq!(bf16::from(f32::from_bits(0x3f81_8000)).0, 0x3f82);
        assert_eq!(bf16::from(f32::from_bits(0x3f80_8000)).0, 0x3f80);
    }

    #[test]
    fn a_bf16_round_trips_through_f32() {
        for word in [0x3f80u16, 0x0000, 0xbf80, 0x3586, 0x4049] {
            assert_eq!(bf16::from(f32::from(bf16(word))).0, word);
        }
    }
}
