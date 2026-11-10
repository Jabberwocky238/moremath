# moremath

Numbers beyond the standard library, as plain math (`no_std`, no `alloc`):

| type | format |
| --- | --- |
| `bf16` | 1-8-7 brain float: round-half-to-even from `f32`, `TowardZero` from `f64`, correctly rounded `+ - * /` |
| `f16` | IEEE 754 binary16 → `f32` / `f64` (exact) |
| `tf32` | 1-8-10 in the low 19 bits of a `u32` → `f32` (exact) |
| `u1`, `u4`, `i4` | sub-byte integers; built only through `TryFrom` (no silent truncation) |

Conversions with a context (`FromWith` / `IntoWith` / `TryFromWith` / `TryIntoWith`) carry what
the value alone cannot say, such as the rounding mode. `transquant!` chains them:

```rust
use moremath::transquant;
assert_eq!(transquant!(0x3f80_8000u32, u32 to f32 to bf16 to u16), 0x3f80); // tie to even
assert_eq!(transquant!(1.0f64 + 1e-9, f64 to bf16 with TowardZero to u16), 0x3f80);
```

## Costs

`Cost::cost()` says what something costs as one plain quantity (`Quantity`: anything that adds up and
compares — `f64`, `u64`, or the implementer's own unit); `Candidates::cheapest_or(rule)` picks the cheapest of
several ways of doing the same thing, or falls back to the caller's `rule` when any of them cannot be costed.
What the quantity means is the implementer's business.

## Running tasks

An `Execute` is one task (any `FnOnce() -> T + Send` is one); an `Executor` runs a `Vec` of them
and returns their outputs in task order. Two executors implement the same traits side by side:

| executor | feature | runs tasks |
| --- | --- | --- |
| `ThreadSingle` | `alloc` (`no_std`) | on the calling thread, in order |
| `ThreadMulti` | `std` | lock-free, on a pool of OS threads started once (`default()`: one per CPU; `from(n)` for `n: NonZeroUsize`); each worker takes the next untaken task; a call waits for the whole pool, so tasks may borrow from the caller |

## Bits and small syntax

| macro | what |
| --- | --- |
| `bit![word, [12..=15]]` / `bit![word, [12..=15], u8]` | read a bit field by its datasheet indices (low first, `=` required; `[12..16]` is a compile error); typed form panics when the field does not fit |
| `pack![u32, [25..=31] = f7, [0..=6] = 0x2b]` | build a word from its fields; overlaps / out-of-width / literal overflow are compile errors, a variable that does not fit panics naming its field; works in `const fn` |
| `pick!(b == 0 ? u32::MAX : a / b)` | C's conditional operator, both arms lazy |
| `declare_error! { @name E  Group { V { x: u32 } => "{x}", ... } }` | an error table into a `thiserror` enum (the caller depends on `thiserror`) |

`bit!` / `pack!` / `pick!` live in the `moremath-macros` proc-macro crate and are re-exported here;
depend on `moremath` only.

