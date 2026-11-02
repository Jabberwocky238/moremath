use moremath::pick;

#[test]
fn a_path_in_the_true_arm_survives_the_split() {
    let (a, b) = (7u32, 0u32);
    assert_eq!(pick!(b == 0 ? u32::MAX : a / b), u32::MAX);
    assert_eq!(pick!(b != 0 ? u32::MAX : a + 1), 8);
}

#[test]
fn the_arm_not_taken_does_not_run() {
    let (a, b) = (7u32, 0u32);
    assert_eq!(pick!(b == 0 ? 0 : a / b), 0);
}

#[test]
fn a_nested_group_keeps_its_own_punctuation() {
    let word = 0x1234u32;
    let funct3 = 5u32;
    assert_eq!(
        pick!(matches!(funct3, 1 | 5) ? (word >> 8) & 0x1f : word),
        (word >> 8) & 0x1f
    );
}

#[test]
fn the_arms_may_be_any_expression() {
    let reg = 0u8;
    let values = [9u32; 32];
    assert_eq!(pick!(reg == 0 ? 0 : values[reg as usize & 31]), 0);
    assert_eq!(pick!(reg == 7 ? "srai" : "srli"), "srli");
}

#[test]
fn it_is_an_expression_like_any_other() {
    let n = 3u32;
    assert_eq!(1 + pick!(n > 2 ? 10 : 20), 11);
    assert_eq!(pick!(n == 3 ? pick!(n > 0 ? "a" : "b") : "c"), "a");
}

#[test]
fn an_absolute_path_in_the_false_arm_is_not_the_separator() {
    let yes = false;
    assert_eq!(pick!(yes ? 0 : ::core::mem::size_of::<u32>()), 4);
}
