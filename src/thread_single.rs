//! [`ThreadSingle`]: the one-thread side of [`ThreadMulti`](crate::ThreadMulti) — the same
//! [`Executor`], every task run on the calling thread in order. `no_std`.

use crate::{Execute, Executor};
use alloc::vec::Vec;

/// ⭐ Runs each task on the calling thread, in task order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ThreadSingle;

impl Executor for ThreadSingle {
    #[inline]
    fn threads(&self) -> usize {
        1
    }

    #[inline]
    fn execute<'x, 't, E: Execute<'t>>(&'x self, tasks: Vec<E>) -> Vec<E::Output> {
        tasks.into_iter().map(Execute::execute).collect()
    }
}
