// TF32：19 位浮点，位宽不是 2 的幂。

/// ⭐ TF32：1 位符号、8 位指数（偏置 127）、10 位尾数 —— 就是 f32 去掉低 13 位尾数。
/// 位型放在 u32 的低 19 位。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
#[allow(non_camel_case_types, reason = "与原生浮点类型（f32 / f64）同一种写法：它就是一种数")]
pub struct tf32(u32);

impl TryFrom<u32> for tf32 {
    type Error = crate::OutOfRange;

    /// 低 19 位的位型 ⇒ tf32；高位非零拒（⛔ 不静默截掉）。
    fn try_from(bits: u32) -> Result<Self, Self::Error> {
        if bits >> 19 == 0 { Ok(Self(bits)) } else { Err(crate::OutOfRange) }
    }
}

impl From<tf32> for u32 {
    fn from(value: tf32) -> Self {
        value.0
    }
}

impl From<tf32> for f32 {
    /// 精确：补回低 13 位的 0。
    fn from(value: tf32) -> Self {
        Self::from_bits(value.0 << 13)
    }
}
