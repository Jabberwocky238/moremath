//! ⭐ quant 转换表的每一条，穷举或按舍入情形全覆盖；期望值一律用**独立的写法**算（按 IEEE 的定义在 f64 里现算、
//! 按与两个邻居的距离定舍入方向），⛔ 不调被测的那几行位运算。
//!
//! ```text
//! 位型    u16 ⇔ bf16 / f16、u32 ⇔ f32、u64 ⇔ f64    往返恒等（NaN 的载荷也不丢）
//! 加宽    bf16 / f16 ⇒ f32 / f64、f32 ⇒ f64           全部 65,536 个码对定义逐位
//! 变窄    f32 ⇒ bf16（就近偶数）                       每个 bf16 码 × 舍入的每种情形（低 16 位：0、1、半差一、恰半、半多一、全 1 …）
//!         f64 ⇒ bf16（向零截断）                       每个 bf16 码 × 码上 / 码后一点 / 两码中点 / 下一码前一点
//!         f64 ⇒ f32                                    就近偶数（`as`）在邻居与中点上
//! 宏      transquant! 的链 == 逐步手写；`with` 换上下文
//! ```
use moremath::{bf16, f16, IntoWith, Quant, transquant};

/// bf16 码按定义的值：(−1)ˢ · 2^(e−127) · (1 + m/128)，e = 0 时 2⁻¹²⁶ · m/128；inf / NaN 另说。
fn bf16_value(code: u16) -> f64 {
    let (sign, e, m) = (if code >> 15 == 1 { -1.0 } else { 1.0 }, i32::from((code >> 7) & 0xff), f64::from(code & 0x7f));
    match e {
        0xff if m == 0.0 => sign * f64::INFINITY,
        0xff => f64::NAN,
        0 => sign * 2f64.powi(-126) * m / 128.0,
        _ => sign * 2f64.powi(e - 127) * (1.0 + m / 128.0),
    }
}

/// 舍入时「下一个码」的值：最大有限码之后是 2¹²⁸（IEEE 的就近舍入就拿它当 inf 的位置）。
fn successor_value(code: u16) -> f64 {
    let next = code + 1;
    if next & 0x7fff == 0x7f80 { f64::copysign(2f64.powi(128), bf16_value(code)) } else { bf16_value(next) }
}

fn is_nan_code(code: u16) -> bool {
    code & 0x7f80 == 0x7f80 && code & 0x7f != 0
}

/// 每个有限、非 NaN 的 bf16 码（两种符号，含 ±0 与非规格数）。
fn finite_codes() -> impl Iterator<Item = u16> {
    (0..=u16::MAX).filter(|&c| c & 0x7f80 != 0x7f80)
}

#[test]
fn bit_patterns_round_trip() {
    for code in 0..=u16::MAX {
        assert_eq!(transquant!(code, u16 to bf16 to u16), code);
        assert_eq!(transquant!(code, u16 to f16 to u16), code);
    }
    // u32 ⇔ f32、u64 ⇔ f64：每个高 16 位 × 低位的几种形态（NaN 的各种载荷都在里面）
    for high in 0..=u32::from(u16::MAX) {
        for low in [0, 1, 0x7fff, 0x8000, 0xffff] {
            let bits = high << 16 | low;
            assert_eq!(transquant!(bits, u32 to f32 to u32), bits);
            let wide = u64::from(bits) << 32 | u64::from(low);
            assert_eq!(transquant!(wide, u64 to f64 to u64), wide);
        }
    }
}

#[test]
fn widening_is_exact_for_every_code() {
    for code in 0..=u16::MAX {
        let want = bf16_value(code);
        let (single, double): (f32, f64) = (transquant!(code, u16 to bf16 to f32), transquant!(code, u16 to bf16 to f64));
        if want.is_nan() {
            assert!(single.is_nan() && double.is_nan(), "{code:#06x}");
            assert_eq!(single.to_bits(), u32::from(code) << 16, "{code:#06x}：NaN 的载荷要原样留着");
        } else {
            assert_eq!(f64::from(single).to_bits(), want.to_bits(), "{code:#06x} ⇒ f32");
            assert_eq!(double.to_bits(), want.to_bits(), "{code:#06x} ⇒ f64");
        }
        // f16：(−1)ˢ · 2^(e−15) · (1 + m/1024)
        let (sign, e, m) = (if code >> 15 == 1 { -1.0 } else { 1.0 }, i32::from((code >> 10) & 0x1f), f64::from(code & 0x3ff));
        let half: f64 = transquant!(code, u16 to f16 to f64);
        match e {
            0x1f if m == 0.0 => assert_eq!(half.to_bits(), (sign * f64::INFINITY).to_bits(), "f16 {code:#06x}"),
            0x1f => assert!(half.is_nan() && transquant!(code, u16 to f16 to f32 to u32) & 0x7f_ffff == u32::from(code & 0x3ff) << 13, "f16 {code:#06x}"),
            0 => assert_eq!(half.to_bits(), (sign * 2f64.powi(-14) * m / 1024.0).to_bits(), "f16 {code:#06x}"),
            _ => assert_eq!(half.to_bits(), (sign * 2f64.powi(e - 15) * (1.0 + m / 1024.0)).to_bits(), "f16 {code:#06x}"),
        }
        assert_eq!(transquant!(code, u16 to f16 to f32 to f64).to_bits(), half.to_bits(), "f16 {code:#06x}：经 f32 与直接到 f64 一致");
    }
}

#[test]
fn f32_to_bf16_rounds_to_nearest_even_in_every_case() {
    // 低 16 位覆盖舍入的每种情形：0（恰在码上）、1、半差一、恰半（平局）、半多一、全 1，外加中间几处
    const LOWS: [u32; 10] = [0, 1, 0x3fff, 0x4000, 0x7ffe, 0x7fff, 0x8000, 0x8001, 0xc000, 0xffff];
    for high in 0..=u16::MAX {
        for low in LOWS {
            let x = f32::from_bits(u32::from(high) << 16 | low);
            let got: u16 = transquant!(x, f32 to bf16 to u16);
            if x.is_nan() {
                assert!(is_nan_code(got) && got >> 15 == high >> 15, "{x:?}（{:#010x}）⇒ {got:#06x}：NaN 要留 NaN、留符号", x.to_bits());
                continue;
            }
            if x.is_infinite() {
                assert_eq!(got, high, "{x}");
                continue;
            }
            // 两个候选：向零截断的码、它的下一个码；取近的，一样近取尾数为偶的
            let (x, below) = (f64::from(x), high);
            let (d_below, d_above) = ((x - bf16_value(below)).abs(), (successor_value(below) - x).abs());
            let want = match d_below.total_cmp(&d_above) {
                core::cmp::Ordering::Less => below,
                core::cmp::Ordering::Greater => below + 1,
                core::cmp::Ordering::Equal => if below & 1 == 0 { below } else { below + 1 },
            };
            assert_eq!(got, want, "{x:e}（{:#010x}）", u32::from(high) << 16 | low);
        }
    }
}

#[test]
fn f64_to_bf16_toward_zero_at_every_boundary() {
    let toward = |y: f64| -> u16 { transquant!(y, f64 to bf16 with TowardZero to u16) };
    for code in finite_codes() {
        let (value, next) = (bf16_value(code), successor_value(code));
        // 码上、码后一个 f64 ulp、两码中点、下一码前一个 f64 ulp ⇒ 都截到这个码（下一码是 2¹²⁸ 时也一样：⛔ 不进 inf）
        let away = |v: f64| f64::from_bits(v.to_bits() + 1);
        let toward_value = |v: f64| f64::from_bits(v.to_bits() - 1);
        let points = [value, if value == 0.0 { f64::copysign(f64::from_bits(1), value) } else { away(value) }, f64::midpoint(value, next), toward_value(next)];
        for y in points {
            assert_eq!(toward(y), code, "{y:e} 应截到 {code:#06x}");
        }
        if next.abs() < 2f64.powi(128) {
            assert_eq!(toward(next), code + 1, "{next:e} 恰在下一码上");
        }
    }
    assert_eq!(toward(f64::INFINITY), 0x7f80);
    assert_eq!(toward(f64::NEG_INFINITY), 0xff80);
    assert_eq!(toward(1e300), 0x7f7f, "有限的大数截到最大有限码，⛔ 不进 inf");
    assert_eq!(toward(-1e300), 0xff7f);
    assert_eq!(toward(1e-300), 0x0000, "比最小非规格数还小 ⇒ +0");
    assert_eq!(toward(-1e-300), 0x8000, "⇒ −0（符号留着）");
    assert!(is_nan_code(toward(f64::NAN)));
}

#[test]
fn f64_to_f32_rounds_to_nearest_even() {
    // 每个 f32 码与它下一个码的中点（f64 里精确）：平局落到尾数为偶的那个；中点两侧一个 f64 ulp 各落向近的
    for high in 0..=u32::from(u16::MAX) {
        for low in [0u32, 1, 0x7ffe, 0xfffe] {
            let a = f32::from_bits(high << 16 | low);
            let b = f32::from_bits((high << 16 | low) + 1);
            if !a.is_finite() || !b.is_finite() {
                continue;
            }
            let (lo, hi) = (f64::from(a), f64::from(b));
            let mid = f64::midpoint(lo, hi);
            let even = if a.to_bits() & 1 == 0 { a } else { b };
            let single: f32 = transquant!(mid, f64 to f32);
            assert_eq!(single.to_bits(), even.to_bits(), "中点 {mid:e}");
            // 位型减一 = 往零挪（两种符号都是）⇒ 落到小的那个 a；加一 ⇒ 落到 b
            assert_eq!(transquant!(f64::from_bits(mid.to_bits() - 1), f64 to f32).to_bits(), a.to_bits(), "中点内侧 {mid:e}");
            assert_eq!(transquant!(f64::from_bits(mid.to_bits() + 1), f64 to f32).to_bits(), b.to_bits(), "中点外侧 {mid:e}");
            assert_eq!(transquant!(lo, f64 to f32).to_bits(), a.to_bits(), "码上");
        }
    }
}

#[test]
fn the_macro_is_the_chain_written_out() {
    for code in finite_codes().step_by(7) {
        let x = bf16(code);
        // 逐步手写 == 宏
        let step: f64 = IntoWith::<f32, _>::into_with(x, Quant).into_with(Quant);
        assert_eq!(transquant!(code, u16 to bf16 to f32 to f64).to_bits(), step.to_bits());
        // f64 ⇒ f32 ⇒ bf16 两次就近 与 f64 向零截断 是两回事：码上相同，码后一点不同方向时才分开
        let y = bf16_value(code);
        assert_eq!(transquant!(y, f64 to f32 to bf16 to u16), transquant!(y, f64 to bf16 with TowardZero to u16));
    }
    // 平局：0x3f80_8000 是 1.0 与下一码的正中，就近偶数留在 1.0
    assert_eq!(transquant!(0x3f80_8000u32, u32 to f32 to bf16 to u16), 0x3f80);
    assert_eq!(transquant!(0x3f81_8000u32, u32 to f32 to bf16 to u16), 0x3f82);
    let _: f16 = transquant!(0x3c00u16, u16 to f16);
}
