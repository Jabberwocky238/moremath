//! ⭐ Independent work: an [`Execute`] is one task, an [`Executor`] runs a batch of them and
//! hands back their outputs in task order. Two executors implement them side by side:
//! [`ThreadSingle`](crate::ThreadSingle) (the calling thread, `no_std`) and
//! [`ThreadMulti`](crate::ThreadMulti) (a thread pool, `std`).
//!
//! Two lifetimes, unrelated to each other:
//!
//! * `'t`, the tasks': what a task borrows lives for `'t`, and so does its output — at least
//!   (`Output: 't`): the output may be kept as long as anything the task borrowed.
//! * `'x`, the executor's borrow in [`Executor::execute`]: only for the call. Nothing returned
//!   borrows from the executor, so the outputs outlive it if `'t` does.

use alloc::vec::Vec;

/// One task, living for `'t`, consumed by running it; its output lives at least as long.
pub trait Execute<'t>: Send + 't {
    type Output: Send + 't;

    fn execute(self) -> Self::Output;
}

/// A closure is a task: it lives for `'t`, so must what it returns.
impl<'t, T: Send + 't, F: FnOnce() -> T + Send + 't> Execute<'t> for F {
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

    /// Runs `tasks`, borrowing the executor for `'x` only: the outputs are the tasks' (`'t`),
    /// not the executor's.
    fn execute<'x, 't, E: Execute<'t>>(&'x self, tasks: Vec<E>) -> Vec<E::Output>;
}
