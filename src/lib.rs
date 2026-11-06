#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod bfloat;
mod error;
mod half;
mod narrow;
#[cfg(feature = "alloc")]
mod parallel;
mod tensorfloat;
#[cfg(feature = "std")]
mod thread_multi;
#[cfg(feature = "alloc")]
mod thread_single;
mod trans;
mod transform;

pub use bfloat::{bf16, TowardZero};
pub use half::f16;
pub use moremath_macros::{bit, pack, pick};
pub use narrow::{i4, u1, u4};
#[cfg(feature = "alloc")]
pub use parallel::{Execute, Executor};
pub use tensorfloat::tf32;
#[cfg(feature = "std")]
pub use thread_multi::ThreadMulti;
#[cfg(feature = "alloc")]
pub use thread_single::ThreadSingle;
pub use trans::Quant;
pub use transform::{FromWith, IntoWith, TryFromWith, TryIntoWith};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutOfRange;

impl core::fmt::Display for OutOfRange {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("值装不进这个类型（u1 0..=1、u4 0..=15、i4 −8..=7；tf32 的位型只有低 19 位）")
    }
}

impl core::error::Error for OutOfRange {}
