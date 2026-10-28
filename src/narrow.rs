// 不足一个字节的整数：u1、u4、i4。
//
// ⭐ 这里只定**值**：一个 u4 是 0..=15 里的一个数，⛔ 不管它在字节里怎么摆（两个四位谁在低半字节是打包的事，归用它的那一层）。⇒ 造一个窄整数只有 `TryFrom`（装不进就拒），⛔ 没有静默截断。
use crate::OutOfRange;

/// ⭐ 1 位无符号整数（0 或 1）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
#[allow(non_camel_case_types, reason = "与原生整数（u8 / i8）同一种写法：它就是一种数")]
pub struct u1(bool);

impl From<bool> for u1 {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<u1> for bool {
    fn from(value: u1) -> Self {
        value.0
    }
}

impl From<u1> for u8 {
    fn from(value: u1) -> Self {
        Self::from(value.0)
    }
}

impl TryFrom<u8> for u1 {
    type Error = OutOfRange;

    /// 0 / 1 ⇒ u1；其余拒。
    fn try_from(value: u8) -> Result<Self, OutOfRange> {
        match value {
            0 => Ok(Self(false)),
            1 => Ok(Self(true)),
            _ => Err(OutOfRange),
        }
    }
}

/// ⭐ 4 位无符号整数（0..=15）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
#[allow(non_camel_case_types, reason = "与原生整数（u8 / i8）同一种写法：它就是一种数")]
pub struct u4(u8);

impl u4 {
    pub const MIN: Self = Self(0);
    pub const MAX: Self = Self(15);
}

impl From<u4> for u8 {
    fn from(value: u4) -> Self {
        value.0
    }
}

impl TryFrom<u8> for u4 {
    type Error = OutOfRange;

    /// 0..=15 ⇒ u4；其余拒。
    fn try_from(value: u8) -> Result<Self, OutOfRange> {
        if value <= Self::MAX.0 { Ok(Self(value)) } else { Err(OutOfRange) }
    }
}

/// ⭐ 4 位有符号整数（−8..=7，补码）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
#[allow(non_camel_case_types, reason = "与原生整数（u8 / i8）同一种写法：它就是一种数")]
pub struct i4(i8);

impl i4 {
    pub const MIN: Self = Self(-8);
    pub const MAX: Self = Self(7);
}

impl From<i4> for i8 {
    fn from(value: i4) -> Self {
        value.0
    }
}

impl TryFrom<i8> for i4 {
    type Error = OutOfRange;

    /// −8..=7 ⇒ i4；其余拒。
    fn try_from(value: i8) -> Result<Self, OutOfRange> {
        if (Self::MIN.0..=Self::MAX.0).contains(&value) { Ok(Self(value)) } else { Err(OutOfRange) }
    }
}
