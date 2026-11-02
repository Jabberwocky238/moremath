use crate::{bf16, f16, tf32};
use crate::FromWith;

#[derive(Clone, Copy, Debug)]
pub struct Quant;

macro_rules! table {
    ($($from:ty => $to:ty : |$v:ident| $how:expr;)+) => {$(
        impl FromWith<$from, Quant> for $to {
            fn from_with($v: $from, _: Quant) -> Self {
                $how
            }
        }
    )+};
}

table! {
    u16 => bf16: |v| bf16(v);
    bf16 => u16: |v| v.0;
    u16 => f16: |v| f16(v);
    f16 => u16: |v| v.0;
    u32 => f32: |v| f32::from_bits(v);
    f32 => u32: |v| v.to_bits();
    u64 => f64: |v| f64::from_bits(v);
    f64 => u64: |v| v.to_bits();
    bf16 => f32: |v| v.into();
    bf16 => f64: |v| v.into();
    f16 => f32: |v| v.into();
    f16 => f64: |v| v.into();
    f32 => f64: |v| v.into();
    f64 => f32: |v| {
        #[allow(clippy::cast_possible_truncation, reason = "`as` 就是 IEEE 的就近舍入到偶数")]
        let single = v as f32;
        single
    };
    f32 => bf16: |v| v.into();
    tf32 => f32: |v| v.into();
}

#[macro_export]
macro_rules! transquant {
    ($value:expr, $from:ident $(to $to:ident $(with $context:ident)?)+) => {{
        let value: $crate::transquant!(@type $from) = $value;
        $(let value: $crate::transquant!(@type $to) = $crate::IntoWith::into_with(value, $crate::transquant!(@context $($context)?));)+
        value
    }};
    (@type bf16) => { $crate::bf16 };
    (@type f16) => { $crate::f16 };
    (@type tf32) => { $crate::tf32 };
    (@type $other:ident) => { $other };
    (@context) => { $crate::Quant };
    (@context $context:ident) => { $crate::$context };
}
