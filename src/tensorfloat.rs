#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
#[allow(non_camel_case_types, reason = "与原生浮点类型（f32 / f64）同一种写法：它就是一种数")]
pub struct tf32(u32);

impl TryFrom<u32> for tf32 {
    type Error = crate::OutOfRange;

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
    fn from(value: tf32) -> Self {
        Self::from_bits(value.0 << 13)
    }
}
