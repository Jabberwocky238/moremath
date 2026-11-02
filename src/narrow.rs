use crate::OutOfRange;

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

    fn try_from(value: u8) -> Result<Self, OutOfRange> {
        match value {
            0 => Ok(Self(false)),
            1 => Ok(Self(true)),
            _ => Err(OutOfRange),
        }
    }
}

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

    fn try_from(value: u8) -> Result<Self, OutOfRange> {
        if value <= Self::MAX.0 { Ok(Self(value)) } else { Err(OutOfRange) }
    }
}

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

    fn try_from(value: i8) -> Result<Self, OutOfRange> {
        if (Self::MIN.0..=Self::MAX.0).contains(&value) { Ok(Self(value)) } else { Err(OutOfRange) }
    }
}
