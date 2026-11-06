//! [`ThreadMulti`]: the pool side of [`ThreadSingle`](crate::ThreadSingle) — the same
//! [`Executor`], lock-free, over a pool of OS threads started once and reused by every
//! [`execute`](Executor::execute).
//!
//! Calls nest: a task may call [`execute`](Executor::execute) again, so work splits into smaller
//! tasks as deep as it likes. Every call in flight is a *batch* listed in a fixed table; a
//! thread with nothing to do takes the next untaken task of any listed batch. The caller of a
//! batch takes its own tasks first, then helps the other batches until its own are done, so a
//! nested call never idles a worker.
//!
//! Nothing locks: tasks are handed out by `fetch_add`, batches listed with compare-and-swap,
//! and threads sleep and wake with `park` / `unpark`. A call returns only once all its tasks are
//! done and no thread looks at its batch any more, so tasks may borrow anything that outlives
//! the call.

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

/// ⭐ A thread pool as an [`Executor`]. `n` workers: `n − 1` pool threads and whichever thread
/// calls. Calls nest (a task may [`execute`](Executor::execute) more tasks); each worker takes
/// the next task not yet taken, so uneven tasks spread over them.
pub struct ThreadMulti {
    shared: Arc<Shared>,
    pool: Vec<JoinHandle<()>>,
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
        Self { shared, pool }
    }
}

impl Executor for ThreadMulti {
    /// The pool and the calling thread.
    #[inline]
    fn threads(&self) -> usize {
        self.pool.len() + 1
    }

    fn execute<'x, 't, E: Execute<'t>>(&'x self, tasks: Vec<E>) -> Vec<E::Output> {
        if self.pool.is_empty() || tasks.len() <= 1 {
            return tasks.into_iter().map(Execute::execute).collect();
        }
        let batch = Batch::from(tasks);
        // With the table full, the caller runs its batch alone.
        if let Some(entry) = self.shared.list(&batch.header) {
            let listed = Listed { shared: &self.shared, entry, batch: &batch };
            for t in &self.pool {
                t.thread().unpark();
            }
            drop(listed);
        } else {
            while batch.help() {}
        }
        batch.outputs()
    }
}

impl Drop for ThreadMulti {
    fn drop(&mut self) {
        self.shared.quit.store(true, Ordering::Release);
        for t in self.pool.drain(..) {
            t.thread().unpark();
            // A pool thread never panics: a task's panic is caught and resumed in its caller.
            let _ = t.join();
        }
    }
}

/// How many batches may be in flight at once; a call past that runs its tasks alone.
const ENTRIES: usize = 64;

/// What the pool threads and every caller share: the batches in flight.
struct Shared {
    entries: Box<[Entry]>,
    quit: AtomicBool,
}

impl Default for Shared {
    fn default() -> Self {
        Self { entries: (0..ENTRIES).map(|_| Entry::default()).collect(), quit: AtomicBool::new(false) }
    }
}

/// One place in the table: a listed batch (or null), and how many threads may be looking at it.
/// A thread counts itself in `users` before it reads `batch`; the caller clears `batch`, then
/// waits for `users` to drain, so no thread reads a batch that is gone.
#[derive(Default)]
struct Entry {
    batch: AtomicPtr<Header>,
    users: AtomicUsize,
}

impl Shared {
    /// Lists `batch` in a free entry.
    fn list(&self, batch: &Header) -> Option<&Entry> {
        let p = ptr::from_ref(batch).cast_mut();
        self.entries.iter().find(|e| e.batch.compare_exchange(ptr::null_mut(), p, Ordering::SeqCst, Ordering::Relaxed).is_ok())
    }

    /// Takes and runs one task of any listed batch; `false` when none is left untaken.
    fn help(&self) -> bool {
        self.entries.iter().any(|e| {
            if e.batch.load(Ordering::Relaxed).is_null() {
                return false;
            }
            e.users.fetch_add(1, Ordering::SeqCst);
            let p = e.batch.load(Ordering::SeqCst);
            // SAFETY: counted in `users`, so the batch behind a non-null `p` stays listed and
            // alive until the count drops (`Listed::drop`).
            let ran = !p.is_null() && unsafe { ((*p).help)(p) };
            e.users.fetch_sub(1, Ordering::SeqCst);
            ran
        })
    }

    /// A pool thread: helps while there is work, sleeps when there is none, until told to quit.
    fn serve(&self) {
        while !self.quit.load(Ordering::Acquire) {
            if !self.help() {
                std::thread::park();
            }
        }
    }
}

/// A batch while it is listed. Dropped — even while unwinding — it waits for the batch: the
/// caller takes its own untaken tasks first, then helps any listed batch (or sleeps, until the
/// last task done wakes it) while its own tasks still run elsewhere; then it unlists the batch
/// and waits until no thread looks at it.
struct Listed<'s, 'b, 't, E: Execute<'t>> {
    shared: &'s Shared,
    entry: &'s Entry,
    batch: &'b Batch<'t, E>,
}

impl<'t, E: Execute<'t>> Drop for Listed<'_, '_, 't, E> {
    fn drop(&mut self) {
        while self.batch.help() {}
        while !self.batch.done() {
            if !self.shared.help() {
                std::thread::park();
            }
        }
        self.entry.batch.store(ptr::null_mut(), Ordering::SeqCst);
        while self.entry.users.load(Ordering::SeqCst) != 0 {
            core::hint::spin_loop();
        }
    }
}

/// What the table holds of a batch: how to run one of its tasks, whatever its type.
struct Header {
    /// [`Batch::help`] of the batch this header starts.
    help: unsafe fn(*const Header) -> bool,
}

/// One call's tasks, on the caller's stack. `header` comes first, so a `*const Header` from the
/// table is a `*const Batch`.
#[repr(C)]
struct Batch<'t, E: Execute<'t>> {
    header: Header,
    slots: Vec<Slot<'t, E>>,
    /// The next task to hand out.
    next: AtomicUsize,
    /// Tasks not done yet.
    pending: AtomicUsize,
    /// Woken when the last task is done.
    caller: Thread,
    /// The first panic a task raised (`Box<Panic>` raw, or null).
    panic: AtomicPtr<Panic>,
}

impl<'t, E: Execute<'t>> From<Vec<E>> for Batch<'t, E> {
    fn from(tasks: Vec<E>) -> Self {
        let pending = AtomicUsize::new(tasks.len());
        let slots = tasks.into_iter().map(Slot::from).collect();
        Self { header: Header { help: Self::help_erased }, slots, next: AtomicUsize::new(0), pending, caller: std::thread::current(), panic: AtomicPtr::new(ptr::null_mut()) }
    }
}

impl<'t, E: Execute<'t>> Batch<'t, E> {
    /// [`Batch::help`] reached through the table.
    ///
    /// # Safety
    /// `h` is the header of a live `Batch<'t, E>`.
    unsafe fn help_erased(h: *const Header) -> bool {
        // SAFETY: `#[repr(C)]` with `header` first; the caller vouches it is alive.
        unsafe { &*h.cast::<Self>() }.help()
    }

    /// Takes and runs the next untaken task; `false` when none is left. A panic is kept for the
    /// caller; the last task done wakes it.
    fn help(&self) -> bool {
        let Some(slot) = self.slots.get(self.next.fetch_add(1, Ordering::Relaxed)) else { return false };
        if let Err(p) = catch_unwind(AssertUnwindSafe(|| slot.run())) {
            self.keep(p);
        }
        if self.pending.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.caller.unpark();
        }
        true
    }

    fn done(&self) -> bool {
        self.pending.load(Ordering::Acquire) == 0
    }

    /// Keeps `p` unless a panic is kept already.
    fn keep(&self, p: Panic) {
        let p = Box::into_raw(Box::new(p));
        if self.panic.compare_exchange(ptr::null_mut(), p, Ordering::AcqRel, Ordering::Acquire).is_err() {
            // SAFETY: `p` came from `Box::into_raw` just above and was never shared.
            drop(unsafe { Box::from_raw(p) });
        }
    }

    /// The outputs in task order, or the first panic resumed. Every task is done.
    fn outputs(self) -> Vec<E::Output> {
        let p = self.panic.swap(ptr::null_mut(), Ordering::Acquire);
        if !p.is_null() {
            // SAFETY: a non-null `panic` came from `Box::into_raw` in `keep`; the swap took it.
            resume_unwind(*unsafe { Box::from_raw(p) });
        }
        self.slots.into_iter().map(Slot::output).collect()
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
// the caller only after every task is done; the task and its output are `Send`.
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
