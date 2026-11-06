#![cfg(feature = "std")]

use core::num::NonZeroUsize;
use core::sync::atomic::{AtomicUsize, Ordering};
use moremath::{Executor, ThreadMulti, ThreadSingle};

fn pool() -> ThreadMulti {
    ThreadMulti::from(NonZeroUsize::new(4).unwrap())
}

#[test]
fn outputs_in_task_order() {
    let x = pool();
    let tasks: Vec<_> = (0..1000u64).map(|i| move || i * i).collect();
    assert_eq!(x.execute(tasks), (0..1000u64).map(|i| i * i).collect::<Vec<_>>());
}

#[test]
fn nested_calls_split_further() {
    let x = pool();
    let outer: Vec<_> = (0..16u64)
        .map(|i| {
            let x = &x;
            move || x.execute((0..64u64).map(|j| move || i * 100 + j).collect::<Vec<_>>()).iter().sum::<u64>()
        })
        .collect();
    let want: Vec<u64> = (0..16u64).map(|i| (0..64u64).map(|j| i * 100 + j).sum()).collect();
    assert_eq!(x.execute(outer), want);
}

#[test]
fn tasks_borrow_from_the_caller() {
    let x = pool();
    let data: Vec<u32> = (0..10_000).collect();
    let seen = AtomicUsize::new(0);
    let tasks: Vec<_> = data
        .chunks(100)
        .map(|c| {
            let seen = &seen;
            move || {
                seen.fetch_add(c.len(), Ordering::Relaxed);
                c.iter().map(|v| u64::from(*v)).sum::<u64>()
            }
        })
        .collect();
    let total: u64 = x.execute(tasks).into_iter().sum();
    assert_eq!(total, (0..10_000u64).sum());
    assert_eq!(seen.into_inner(), 10_000);
}

#[test]
fn a_task_panic_reaches_the_caller() {
    let x = pool();
    let tasks: Vec<Box<dyn FnOnce() -> u32 + Send>> = (0..8u32).map(|i| Box::new(move || if i == 5 { panic!("five") } else { i }) as Box<dyn FnOnce() -> u32 + Send>).collect();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| x.execute(tasks)));
    assert_eq!(*r.unwrap_err().downcast::<&str>().unwrap(), "five");
    // The pool still works after.
    assert_eq!(x.execute(vec![|| 1, || 2]), vec![1, 2]);
}

#[test]
fn single_and_multi_agree() {
    let tasks = || (0..100u32).map(|i| move || i.wrapping_mul(2_654_435_761)).collect::<Vec<_>>();
    assert_eq!(ThreadSingle.execute(tasks()), pool().execute(tasks()));
}
