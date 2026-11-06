//! ⭐ Independent work: an [`Execute`] is one task, an [`Executor`] runs a batch of them and
//! hands back their outputs in task order. Two executors implement them side by side:
//! [`ThreadSingle`](crate::ThreadSingle) (the calling thread, `no_std`) and
//! [`ThreadMulti`](crate::ThreadMulti) (a thread pool, `std`).

use alloc::vec::Vec;

/// One task, consumed by running it.
pub trait Execute: Send {
    type Output: Send;

    fn execute(self) -> Self::Output;
}

/// A closure is a task.
impl<T: Send, F: FnOnce() -> T + Send> Execute for F {
    type Output = T;

    #[inline]
    fn execute(self) -> T {
        self()
    }
}

/// Runs tasks; the outputs come back in the order the tasks went in.
pub trait Executor {
    /// How many tasks it may run at once.
    fn threads(&self) -> usize;

    fn execute<E: Execute>(&self, tasks: Vec<E>) -> Vec<E::Output>;
}
