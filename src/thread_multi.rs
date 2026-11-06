//! [`ThreadMulti`]: the pool side of [`ThreadSingle`](crate::ThreadSingle) — the same
//! [`Executor`], lock-free, over a pool of OS threads started once and reused by every
//! [`execute`](Executor::execute). A call waits for every pool thread before it
//! returns, so its tasks may borrow anything that outlives the call.
//!
//! Nothing here locks: a batch is published through atomics, each task is handed to one worker
//! by `fetch_add`, and threads sleep and wake with `park` / `unpark`.

use crate::{Execute, Executor};
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::num::NonZeroUsize;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::thread::{JoinHandle, Thread};

/// A task's panic, carried to the caller.
type Panic = Box<dyn Any + Send>;

/// ⭐ A thread pool as an [`Executor`]. `n` workers: `n − 1` pool threads and the calling
/// thread. Each worker takes the next task not yet taken, so uneven tasks spread over them.
///
/// One batch runs on the pool at a time: a call made while another runs (from another thread,
/// or from inside one of its tasks) runs its own tasks on its own thread, one after another.
pub struct ThreadMulti {
    shared: Arc<Shared>,
    pool: Vec<JoinHandle<()>>,
    /// A batch is on the pool.
    running: AtomicBool,
}

impl Default for ThreadMulti {
    /// One worker per CPU.
    fn default() -> Self {
        Self::from(std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN))
    }
}

impl From<NonZeroUsize> for ThreadMulti {
    /// Starts `n − 1` pool threads.
    fn from(n: NonZeroUsize) -> Self {
        let shared = Arc::new(Shared::default());
        let pool = (1..n.get())
            .map(|_| {
                let shared = Arc::clone(&shared);
                std::thread::spawn(move || shared.serve())
            })
            .collect();
        Self { shared, pool, running: AtomicBool::new(false) }
    }
}

impl Executor for ThreadMulti {
    /// The pool and the calling thread.
    #[inline]
    fn threads(&self) -> usize {
        self.pool.len() + 1
    }

    fn execute<'x, 't, E: Execute<'t>>(&'x self, tasks: Vec<E>) -> Vec<E::Output> {
        if self.pool.is_empty() || tasks.len() <= 1 || self.running.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            return tasks.into_iter().map(Execute::execute).collect();
        }
        let _running = Running(&self.running);
        let slots: Vec<Slot<'t, E>> = tasks.into_iter().map(Slot::from).collect();
        let next = AtomicUsize::new(0);
        let work = || {
            while let Some(slot) = slots.get(next.fetch_add(1, Ordering::Relaxed)) {
                slot.run();
            }
        };
        let batch = Batch { work: &work, caller: std::thread::current(), busy: AtomicUsize::new(self.pool.len()), panic: AtomicPtr::new(ptr::null_mut()) };
        // From here the pool threads hold `batch`: whatever happens, wait for them.
        let waiting = Waiting(&batch);
        self.shared.job.store(ptr::from_ref(&batch).cast_mut().cast(), Ordering::Release);
        self.shared.batch.fetch_add(1, Ordering::Release);
        for t in &self.pool {
            t.thread().unpark();
        }
        work();
        batch.wait();
        if let Some(p) = batch.take_panic() {
            resume_unwind(p);
        }
        drop(waiting);
        slots.into_iter().map(Slot::output).collect()
    }
}

impl Drop for ThreadMulti {
    fn drop(&mut self) {
        self.shared.quit.store(true, Ordering::Release);
        for t in self.pool.drain(..) {
            t.thread().unpark();
            // A pool thread never panics: a task's panic is caught and resumed in the caller.
            let _ = t.join();
        }
    }
}

/// What the pool threads and [`ThreadMulti`] share.
#[derive(Default)]
struct Shared {
    /// Counts batches, so a pool thread joins each one once.
    batch: AtomicUsize,
    /// The current [`Batch`], published before `batch` counts it.
    job: AtomicPtr<()>,
    quit: AtomicBool,
}

impl Shared {
    /// A pool thread: joins each batch once, until told to quit.
    fn serve(&self) {
        let mut seen = 0;
        loop {
            let now = self.batch.load(Ordering::Acquire);
            if self.quit.load(Ordering::Acquire) {
                return;
            }
            if now == seen {
                std::thread::park();
                continue;
            }
            seen = now;
            // SAFETY: a batch waits for every pool thread to count itself out of `busy` before
            // it returns, so the `Batch` behind `job` lives at least until `busy` is decremented.
            let batch = unsafe { &*self.job.load(Ordering::Acquire).cast::<Batch<'_>>() };
            if let Err(p) = catch_unwind(AssertUnwindSafe(batch.work)) {
                batch.keep(p);
            }
            // Past the decrement the batch may be gone: take the caller's handle first.
            let caller = batch.caller.clone();
            if batch.busy.fetch_sub(1, Ordering::AcqRel) == 1 {
                caller.unpark();
            }
        }
    }
}

/// One batch on the pool, on the caller's stack.
struct Batch<'w> {
    work: &'w (dyn Fn() + Sync),
    caller: Thread,
    /// Pool threads not done with `work` yet.
    busy: AtomicUsize,
    /// The first panic a pool thread caught (`Box<Panic>` raw, or null).
    panic: AtomicPtr<Panic>,
}

impl Batch<'_> {
    /// Waits until no pool thread is in the batch any more.
    fn wait(&self) {
        while self.busy.load(Ordering::Acquire) != 0 {
            std::thread::park();
        }
    }

    /// Keeps `p` unless a panic is kept already.
    fn keep(&self, p: Panic) {
        let p = Box::into_raw(Box::new(p));
        if self.panic.compare_exchange(ptr::null_mut(), p, Ordering::AcqRel, Ordering::Acquire).is_err() {
            // SAFETY: `p` came from `Box::into_raw` just above and was never shared.
            drop(unsafe { Box::from_raw(p) });
        }
    }

    fn take_panic(&self) -> Option<Panic> {
        let p = self.panic.swap(ptr::null_mut(), Ordering::Acquire);
        // SAFETY: a non-null `panic` came from `Box::into_raw` in `keep`; the swap took it.
        (!p.is_null()).then(|| *unsafe { Box::from_raw(p) })
    }
}

/// Waits for the pool threads when dropped, even while the caller unwinds.
struct Waiting<'b, 'w>(&'b Batch<'w>);

impl Drop for Waiting<'_, '_> {
    fn drop(&mut self) {
        self.0.wait();
        drop(self.0.take_panic());
    }
}

/// Clears [`ThreadMulti::running`] when dropped.
struct Running<'r>(&'r AtomicBool);

impl Drop for Running<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// A task until a worker runs it, then its output. `fetch_add` hands each slot to one worker.
struct Slot<'t, E: Execute<'t>>(UnsafeCell<Stage<'t, E>>);

enum Stage<'t, E: Execute<'t>> {
    Task(E),
    Running,
    Done(E::Output),
}

// SAFETY: a slot is touched only by the one worker its index was handed to, and read back by
// the caller only after every worker is done; the task and its output are `Send`.
unsafe impl<'t, E: Execute<'t>> Sync for Slot<'t, E> {}

impl<'t, E: Execute<'t>> From<E> for Slot<'t, E> {
    #[inline]
    fn from(task: E) -> Self {
        Self(UnsafeCell::new(Stage::Task(task)))
    }
}

impl<'t, E: Execute<'t>> Slot<'t, E> {
    /// Runs the task in place of its output. Only the worker the slot was handed to calls it.
    fn run(&self) {
        // SAFETY: see `Slot`: no other reference to the cell exists while this one does.
        let stage = unsafe { &mut *self.0.get() };
        let Stage::Task(task) = core::mem::replace(stage, Stage::Running) else { unreachable!("each task is taken once") };
        *stage = Stage::Done(task.execute());
    }

    fn output(self) -> E::Output {
        match self.0.into_inner() {
            Stage::Done(o) => o,
            _ => unreachable!("every task ran"),
        }
    }
}
