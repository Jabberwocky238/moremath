// ⭐ quant 的转换表：每一对「从 A 换到 B」是一条 `FromWith<A, Quant> for B`，`transquant!` 把它们串成一条链。
//
//     整数 ⇔ 浮点     **位型**：u16 ⇔ bf16 / f16、u32 ⇔ f32、u64 ⇔ f64（⛔ 不是数值的整数 ⇒ 浮点，那是 IR 里的 uitofp）
//     加宽            精确：bf16 / f16 ⇒ f32 / f64、f32 ⇒ f64、tf32 ⇒ f32
//     变窄            就近舍入到偶数：f64 ⇒ f32、f32 ⇒ bf16
//     f64 ⇒ bf16      只有带上下文的那条（`with TowardZero`）；要就近就写 `f64 to f32 to bf16` —— 两次舍入写在明处，
//                     ⛔ 不藏一条「f64 直接就近到 bf16」：一般的 f64 先舍 f32 再舍 bf16 与一次舍到 bf16 结果可能不同。
//
// 判据：`tests/trans.rs`（每一条都穷举或按舍入情形全覆盖，期望值用独立的写法算）。
use crate::{bf16, f16, tf32};
use crate::FromWith;

/// 标记：数的那张转换表（[`transquant!`](crate::transquant) 每一步默认走它）。
#[derive(Clone, Copy, Debug)]
pub struct Quant;

/// 一行一对：`源 => 目的: |参数| 换法;`。
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

/// ⭐ 一串数的转换：`transquant!(值, u32 to f32 to bf16)` ⇒ 值按 u32 读，逐步换到 f32、再到 bf16。
///
/// 每一步是 [`IntoWith`](crate::IntoWith)：默认上下文是 [`Quant`]（这张转换表）；某一步要别的上下文写在它后面：
/// `transquant!(y, f64 to bf16 with TowardZero)`。`bf16` / `f16` / `tf32` 指本 crate 的类型，调用方不用 `use`。
///
/// ```
/// use moremath::transquant;
/// assert_eq!(transquant!(0x3f80_8000u32, u32 to f32 to bf16 to u16), 0x3f80);       // 平局到偶数
/// assert_eq!(transquant!(1.0f64 + 1e-9, f64 to bf16 with TowardZero to u16), 0x3f80);
/// ```
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
