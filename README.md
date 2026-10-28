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
