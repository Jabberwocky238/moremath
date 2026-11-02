use moremath::{bit, pack};

#[test]
fn a_field_is_the_bits_the_datasheet_names() {
    let word = 0xabcd_ef01u32;
    assert_eq!(bit![word, [12..=15]], 0xe);
    assert_eq!(bit![word, [0..=7]], 0x01);
    assert_eq!(bit![word, [31]], 1);
    assert_eq!(bit![word, [16..]], 0xabcd);
}

#[test]
fn what_pack_writes_bit_reads_back() {
    for (lo_value, mid_value, top_value) in [(0u32, 0u32, 0u32), (0x2b, 17, 0x7f), (0x7f, 31, 1)] {
        let word = pack![u32, [0..=6] = lo_value, [7..=11] = mid_value, [25..=31] = top_value];
        assert_eq!(bit![word, [0..=6]], lo_value);
        assert_eq!(bit![word, [7..=11]], mid_value);
        assert_eq!(bit![word, [25..=31]], top_value);
    }
}

#[test]
fn a_named_type_is_checked() {
    let word = 0x0000_0f80u32;
    assert_eq!(bit![word, [7..=11], u8], 31u8);
}

#[test]
#[should_panic(expected = "the field does not fit")]
fn a_field_that_does_not_fit_its_named_type_is_refused() {
    let word = 0xffff_ffffu32;
    let _ = bit![word, [0..=15], u8];
}
