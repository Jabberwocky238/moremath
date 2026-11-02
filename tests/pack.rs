use moremath::pack;

#[test]
fn a_packed_word_is_the_word_the_hand_written_shifts_made() {
    const fn by_hand(f3: u8, slot: u8, rs1: u8, f7: u8) -> u32 {
        ((f7 as u32 & 0x7f) << 25)
            | ((rs1 as u32 & 0x1f) << 15)
            | ((f3 as u32 & 7) << 12)
            | ((slot as u32 & 0x1f) << 7)
            | 0x2b
    }

    const fn custom(f3: u8, slot: u8, rs1: u8, f7: u8) -> u32 {
        pack![u32,
            [25..=31] = f7,
            [15..=19] = rs1,
            [12..=14] = f3,
            [7..=11]  = slot,
            [0..=6]   = 0x2b,
        ]
    }

    const CUSTOM_17: u32 = custom(0, 17, 0, 0);

    for (f3, slot, rs1, f7) in [(0, 0, 0, 0), (0, 17, 0, 0), (7, 16, 3, 0), (1, 5, 31, 0x7f), (2, 31, 1, 10)] {
        assert_eq!(custom(f3, slot, rs1, f7), by_hand(f3, slot, rs1, f7), "f3={f3} slot={slot} rs1={rs1} f7={f7}");
    }

    assert_eq!(custom(0, 17, 0, 0), 0x0000_08ab);
    assert_eq!(CUSTOM_17, 0x0000_08ab);
}

#[test]
fn a_whole_word_of_fields() {
    const fn lock_word(hi8: u8, op8: u8, value: u16) -> u32 {
        pack![u32, [24..=31] = hi8, [16..=23] = op8, [0..=15] = value]
    }
    assert_eq!(lock_word(0, 0, 0), 0);
    assert_eq!(lock_word(0xab, 0xcd, 0xef01), 0xabcd_ef01);
    assert_eq!(lock_word(0, 0, 0xffff), 0x0000_ffff);
    assert_eq!(lock_word(0xff, 0, 0), 0xff00_0000);
}

#[test]
fn undeclared_bits_stay_clear() {
    let word = pack![u32, [7..=11] = 31u8];
    assert_eq!(word, 0x0000_0f80);
}

#[test]
fn a_negative_immediate_is_its_twos_complement() {
    const fn addi(rd: u8, rs1: u8, imm: i32) -> u32 {
        pack![u32, [20..=31] = imm, [15..=19] = rs1, [7..=11] = rd, [0..=6] = 0x13]
    }
    assert_eq!(addi(12, 12, -1), 0xfff6_0613);
    assert_eq!(addi(12, 12, 1), 0x0016_0613);
    assert_eq!(addi(0, 0, -2048) >> 20, 0x800);
    assert_eq!(addi(0, 0, 4095) >> 20, 0xfff);
}

#[test]
#[should_panic(expected = "值放不进这 5 位")]
fn a_value_too_wide_for_its_field_is_refused() {
    let register = 32u8;
    let _ = pack![u32, [7..=11] = register, [0..=6] = 0x13];
}

#[test]
#[should_panic(expected = "值放不进这 12 位")]
fn a_value_too_negative_for_its_field_is_refused() {
    let immediate = -2049i32;
    let _ = pack![u32, [20..=31] = immediate];
}

#[test]
fn an_open_top_runs_to_the_top_of_the_word() {
    let upper = 0xa_2f1u32;
    assert_eq!(pack![u32, [12..] = upper], 0x0a2f_1000);
    assert_eq!(pack![u16, [8..] = 0xffu8], 0xff00);
}

#[test]
fn one_bit_and_a_wide_word() {
    assert_eq!(pack![u32, [31] = 1], 0x8000_0000);
    assert_eq!(pack![u64, [32..=63] = 0xdead_beefu32], 0xdead_beef_0000_0000);
}
