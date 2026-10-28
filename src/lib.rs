//! moremath：标准库之外的数 —— bf16、f16、tf32、u1 / u4 / i4，它们的位型、舍入与转换。
//!
//! 纯数学：每个类型按它的格式定义，舍入按 IEEE 754 就近偶数（或显式的 [`TowardZero`]），
//! ⛔ 不模仿任何一块硬件的近似。`no_std`，不需要 `alloc`。
//!
//! 子模块私有，对外的名字都在根上。
#![no_std]

mod bfloat;
mod half;
mod narrow;
mod tensorfloat;
mod trans;
mod transform;

pub use bfloat::{bf16, TowardZero};
pub use half::f16;
pub use narrow::{i4, u1, u4};
pub use tensorfloat::tf32;
pub use trans::Quant;
pub use transform::{FromWith, IntoWith, TryFromWith, TryIntoWith};

/// 值装不进这个类型（u1 0..=1、u4 0..=15、i4 −8..=7；tf32 的位型只有低 19 位）。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutOfRange;

impl core::fmt::Display for OutOfRange {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("值装不进这个类型（u1 0..=1、u4 0..=15、i4 −8..=7；tf32 的位型只有低 19 位）")
    }
}

impl core::error::Error for OutOfRange {}
