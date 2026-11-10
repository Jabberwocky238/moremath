use moremath::{Candidates, Cost};

struct Fixed(Option<u64>);

impl Cost for Fixed {
    type Error = ();
    type Unit = u64;

    fn cost(&self) -> Result<u64, ()> {
        self.0.ok_or(())
    }
}

#[test]
fn cheapest_or_picks_the_least_and_falls_back_when_any_is_unknown() {
    assert_eq!([Fixed(Some(5)), Fixed(Some(3)), Fixed(Some(3))].cheapest_or(0), 1, "最小、并列取靠前");
    assert_eq!([Fixed(Some(1)), Fixed(None)].cheapest_or(1), 1, "有一个估不出就照规则");
    assert_eq!(<[Fixed]>::cheapest_or(&[], 7), 7, "没有候选也照规则");
}
