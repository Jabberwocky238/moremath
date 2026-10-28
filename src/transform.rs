//! 带上下文的转换：`From` / `Into` / `TryFrom` / `TryInto` 的四个对应物，
//! 各多收一个**转换期才知道**的参数。
//!
//! # ⭐⭐⭐ 为什么标准库那四个不够
//!
//! `From` 的签名说的是「这个值**自己**就足以决定结果」。数的转换里有一整类不满足这句话 ——
//! 最常见的是**舍入方式**：同一个 f64 变成 bf16，就近偶数和向零截断给出不同的字，而舍入方式不在 f64 里。
//!
//! ⇒ 把它变成签名：`bf16: FromWith<f64, TowardZero>` 里的 `TowardZero` 是**必填参数**，
//! 调用方写不出「忘了说怎么舍」的代码。
//!
//! # ⭐⭐ 只实现 `FromWith` / `TryFromWith` 这两个
//!
//! 另外两个由**全覆盖实现**免费得到，和标准库同一条纪律。
//! ⛔ 不要直接为自己的类型实现 `IntoWith` / `TryIntoWith`：那会和这里的全覆盖实现冲突，
//! 而冲突的报错（`conflicting implementations`）指向的是这个文件，不是写错的那一行。
//!
//! # ⭐⭐⭐ 一个源类型，**多个**转换变体 —— 选择器就是 `C`
//!
//! 「加法和减法都是 `From<i32>`」这件事标准库做不到：`impl From<i32> for Offset`
//! 只能有一份。⇒ 需要有个东西把两个变体分开，而**这四个 trait 的 `C` 就是那个位置**，
//! ⛔ 不必再给 trait 加一个 const 参数。
//!
//! 两种写法，都零运行时开销（标记类型是 ZST）：
//!
//! ```
//! use moremath::{FromWith, IntoWith};
//!
//! #[derive(Debug, PartialEq)]
//! struct Offset(i64);
//!
//! // ① 变体是一个**选择** ⇒ 具名的 ZST 标记
//! struct Add;
//! struct Sub;
//! impl FromWith<i32, Add> for Offset {
//!     fn from_with(v: i32, _: Add) -> Self { Offset(i64::from(v)) }
//! }
//! impl FromWith<i32, Sub> for Offset {
//!     fn from_with(v: i32, _: Sub) -> Self { Offset(-i64::from(v)) }
//! }
//!
//! // ② 变体带一个**编译期的数** ⇒ const 泛型的标记
//! struct Scaled<const N: u32>;
//! impl<const N: u32> FromWith<i32, Scaled<N>> for Offset {
//!     fn from_with(v: i32, _: Scaled<N>) -> Self { Offset(i64::from(v) * i64::from(N)) }
//! }
//!
//! let a: Offset = 5i32.into_with(Add);
//! let b: Offset = 5i32.into_with(Sub);
//! let c: Offset = 5i32.into_with(Scaled::<8>);
//! assert_eq!((a, b, c), (Offset(5), Offset(-5), Offset(40)));
//! ```
//!
//! ## 🚨 ⛔ 为什么 const 参数**不**挂在 trait 上
//!
//! `trait FromWith<T, C, const TAG: usize>` 是编得过的（const 参数甚至能有默认值），
//! ⛔ 但它**永远推不出来**：`TAG` 在实参里没有对应物，编译器无从确定它。
//!
//! 实测两种写法在同一个调用点的差别：
//!
//! ```text
//!   C 是标记类型   let x: Offset = 5i32.into_with(Sub);              ✅ 零标注
//!   TAG 挂 trait   let x: Offset = IntoTagged::<Offset, 1>::into_tagged(5i32);
//!                  ⛔ 不写就是 E0283「multiple `impl`s satisfying …」
//! ```
//!
//! ⚠ 而且 `1` 是个魔数：读代码的人要去翻表才知道它是减法。`Sub` 不用翻。
//! ⇒ 需要编译期的**数**时，把 const 泛型放进标记类型（上面的 `Scaled<N>`），
//! ⛔ 而不是放进 trait 的参数表。
//!
//! # 🚨 ⛔ 这里**没有**「`FromWith` 的就自动是 `TryFromWith`」
//!
//! 标准库有 `impl<T, U> TryFrom<U> for T where U: Into<T>`，代价是：**一对类型之间，
//! 不可能既有一个无误的转换、又有一个校验的转换**。
//!
//! ⚠ 而这正是数的转换里常要的一对 —— 「我已经知道它在范围里，直接换」和
//! 「替我检查它在不在范围里」。⇒ 这条全覆盖实现故意不加。
//! 需要「无误版当成可失败版用」的泛型代码，自己写 `Infallible` 的那一层。

/// 从 `T` 变出 `Self`，转换过程要用 `C`。
///
/// ⭐ `C` **按值**收，⛔ 不是 `&C`：这样调用方自己决定 —— 上下文很小就传值
/// （`TowardZero`、`u32`），很大就把 `C` 取成 `&'a Layout`。写死 `&C` 会逼着
/// 每个传 `Copy` 上下文的调用点多写一个 `&`。
pub trait FromWith<T, C>: Sized {
    fn from_with(value: T, context: C) -> Self;
}

/// [`FromWith`] 的反向读法。
///
/// ⛔ **不要实现它** —— 见模块说明。实现 [`FromWith`]，这个自动就有。
pub trait IntoWith<T, C>: Sized {
    fn into_with(self, context: C) -> T;
}

impl<T, U, C> IntoWith<U, C> for T
where
    U: FromWith<T, C>,
{
    fn into_with(self, context: C) -> U {
        U::from_with(self, context)
    }
}

/// 从 `T` 变出 `Self`，可能失败，转换过程要用 `C`。
///
/// ⭐ `Error` 是**关联类型**而不是第三个泛型参数：一对 (源, 上下文) 只该有一种失败
/// 方式。⛔ 做成泛型参数，调用方就得在每个 `?` 处标注它是哪一个。
pub trait TryFromWith<T, C>: Sized {
    type Error;

    /// # Errors
    ///
    /// 由实现者定义 —— 典型的是「值在这个上下文里没有意义」：
    /// 值超出这个上下文给的范围。
    fn try_from_with(value: T, context: C) -> Result<Self, Self::Error>;
}

/// [`TryFromWith`] 的反向读法。
///
/// ⛔ **不要实现它** —— 见模块说明。
pub trait TryIntoWith<T, C>: Sized {
    type Error;

    /// # Errors
    ///
    /// 转发 [`TryFromWith::try_from_with`] 的错误，⛔ 不另包一层。
    fn try_into_with(self, context: C) -> Result<T, Self::Error>;
}

impl<T, U, C> TryIntoWith<U, C> for T
where
    U: TryFromWith<T, C>,
{
    type Error = U::Error;

    fn try_into_with(self, context: C) -> Result<U, Self::Error> {
        U::try_from_with(self, context)
    }
}

/// ⛔ **没有标记就写不出第二个变体** —— 这条是 `C` 存在的反面证据。
///
/// 两个 `impl FromWith<i32, ()>` 冲突，编译期就停：
///
/// ```compile_fail
/// use moremath::FromWith;
///
/// struct Offset(i64);
/// impl FromWith<i32, ()> for Offset {
///     fn from_with(v: i32, (): ()) -> Self { Offset(i64::from(v)) }
/// }
/// // error[E0119]: conflicting implementations of trait `FromWith<i32, ()>`
/// impl FromWith<i32, ()> for Offset {
///     fn from_with(v: i32, (): ()) -> Self { Offset(-i64::from(v)) }
/// }
/// ```
///
/// ⭐ 换成两个不同的标记类型就成立 —— 见模块说明里那段可运行的例子。
#[allow(dead_code)]
#[repr(C)]
struct ConflictWitness;
