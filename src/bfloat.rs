// bf16：字的位型、它和 f32 / f64 之间的舍入，以及它上面正确舍入的 IEEE 四则。
//
// ⭐ 四则（`Add` / `Sub` / `Mul` / `Div`）：f64 里算，再 f64 ⇒ f32 ⇒ bf16 两次就近偶数舍入。结果等于对精确值舍一次：
// 对 + − × ÷ 有定理 —— 中间精度 p' ≥ 2p + 2 时 RNₚ(RNₚ'(x)) = RNₚ(x)。f64 ⇒ f32：53 ≥ 2·24 + 2；f32 ⇒ bf16：24 ≥ 2·8 + 2。
// 取负只翻符号位。
use crate::FromWith;

/// ⭐⭐ A bf16 word: the top 16 bits of an fp32, rounded.
///
/// bf16 has 8 mantissa bits, so an fp32 whose low 16 bits are exactly `0x8000` is an
/// exact tie; [`From<f32>`] breaks it to even.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
#[allow(non_camel_case_types, reason = "与原生浮点类型（f32 / f64）同一种写法：它就是一种数")]
pub struct bf16(pub u16);

impl From<f32> for bf16 {
    /// ⭐⭐⭐ Rounds an fp32 to bf16, **round-half-to-even**.
    ///
    /// ⚠ The tie is decided on the **discarded** bits: exactly `0x8000` and nothing
    /// below, else it is not a tie and rounds by magnitude.
    ///
    /// ⚠ `From` 而⛔ 不是一个叫 `of` 的构造器：一个 f32 **就是**一个待舍入的值，
    /// ⇒ `value.into()` 在调用点说得清，而 `of` 要先去看签名。
    ///
    /// 🚨 NaN ⇒ NaN（高 16 位置上安静位，符号与高位载荷照留）：⛔ 不按大小舍 —— 低位载荷小的 NaN（`0x7f80_0001`）
    /// 照舍入公式会落成 `0x7f80`，也就是 **inf**。
    fn from(value: f32) -> Self {
        let bits = value.to_bits();
        let low = bits & 0xffff;
        #[allow(clippy::cast_possible_truncation, reason = "the top 16 bits, by design")]
        let high = (bits >> 16) as u16;
        if value.is_nan() {
            return Self(high | 0x0040);
        }
        // ⭐ Below the midpoint: truncate. Above: round up. Exactly at it: to even.
        let round_up = match low.cmp(&0x8000) {
            core::cmp::Ordering::Less => false,
            core::cmp::Ordering::Greater => true,
            // 🚨 The tie. `high & 1` is the low mantissa bit, so rounding up only when it
            // is odd lands on even.
            core::cmp::Ordering::Equal => high & 1 == 1,
        };
        Self(high.wrapping_add(u16::from(round_up)))
    }
}

impl From<bf16> for f32 {
    /// The fp32 this word stands for. ⭐ Exact -- bf16 is a truncation of fp32.
    ///
    /// ⚠ 一条标准转换，⛔ 不是一个叫 `to_f32` 的手写方法。
    fn from(value: bf16) -> Self {
        Self::from_bits(u32::from(value.0) << 16)
    }
}


impl From<bf16> for f64 {
    /// 精确：bf16 是 f32 的截断，f32 进 f64 无损。
    fn from(value: bf16) -> Self {
        Self::from(f32::from(value))
    }
}

/// 标记：f64 ⇒ bf16 **向零截断**。
#[derive(Clone, Copy, Debug)]
pub struct TowardZero;

impl FromWith<f64, TowardZero> for bf16 {
    /// 先向零截断到 f32、再丢掉低 16 位：bf16 的网格是 f32 网格的子集（非规格数也是），两次截断复合仍是一次截断。
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
    /// 规范的安静 NaN。
    pub const NAN: Self = Self(0x7fc0);
    pub const INFINITY: Self = Self(0x7f80);
    pub const ONE: Self = Self(0x3f80);
}

/// 取负：只翻符号位（+0 ⇒ −0，非规格数、NaN 的载荷照留）。
impl core::ops::Neg for bf16 {
    type Output = Self;

    fn neg(self) -> Self {
        Self(self.0 ^ 0x8000)
    }
}

/// 两个 bf16 在 f64 里算，再经 f32 就近偶数舍到 bf16（见文件头：不会双重舍入）。
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

    // ⭐⭐ bf16 rounding, on the three cases that matter.
    #[test]
    fn bf16_rounds_half_to_even() {
        // ⭐ Below the midpoint: truncate.
        assert_eq!(bf16::from(f32::from_bits(0x3f80_7fff)).0, 0x3f80);
        // ⭐ Above it: up.
        assert_eq!(bf16::from(f32::from_bits(0x3f80_8001)).0, 0x3f81);
        // 🚨 Exactly at it, from an **odd** word: up, landing on even.
        assert_eq!(bf16::from(f32::from_bits(0x3f81_8000)).0, 0x3f82);
        // 🚨 Exactly at it, from an **even** word: stay.
        assert_eq!(bf16::from(f32::from_bits(0x3f80_8000)).0, 0x3f80);
    }

    // ⭐ bf16 is a truncation of fp32, so a bf16 round-trips exactly.
    #[test]
    fn a_bf16_round_trips_through_f32() {
        for word in [0x3f80u16, 0x0000, 0xbf80, 0x3586, 0x4049] {
            assert_eq!(bf16::from(f32::from(bf16(word))).0, word);
        }
    }
}
