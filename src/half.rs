#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
#[allow(non_camel_case_types, reason = "与原生浮点类型（f32 / f64）同一种写法：它就是一种数")]
pub struct f16(pub u16);

impl From<f16> for f32 {
    #[allow(clippy::cast_precision_loss, reason = "尾数 < 2¹⁰，f32 精确")]
    fn from(f16(h): f16) -> Self {
        let (sign, exponent, fraction) = (u32::from(h >> 15) << 31, u32::from((h >> 10) & 0x1f), u32::from(h & 0x3ff));
        let bits = match (exponent, fraction) {
            (0, 0) => sign,
            (0, _) => return Self::from_bits(sign | 0x3f80_0000) * fraction as Self * Self::from_bits(0x3380_0000),
            (0x1f, _) => sign | 0x7f80_0000 | fraction << 13,
            _ => sign | (exponent + 112) << 23 | fraction << 13,
        };
        Self::from_bits(bits)
    }
}

impl From<f16> for f64 {
    fn from(value: f16) -> Self {
        Self::from(f32::from(value))
    }
}
