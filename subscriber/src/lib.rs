//! A remote endpoint for `tracing-console`

// Borrowed from `tracing`

#[macro_use]
macro_rules! try_lock {
    ($lock:expr) => {
        try_lock!($lock, else return)
    };
    ($lock:expr, else $els:expr) => {
        match $lock {
            Ok(l) => l,
            Err(_) if std::thread::panicking() => $els,
            Err(_) => panic!("lock poisoned"),
        }
    };
}

pub mod future;
mod messages;
pub mod threaded;

use tracing_core::span;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::num::NonZeroU64;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::thread;

static THREAD_COUNTER: AtomicUsize = AtomicUsize::new(1);

thread_local! {
    static THREAD_ID: Cell<usize> = Cell::new(0);

    static STACK: RefCell<Vec<SpanId>> = RefCell::new(Vec::new());
}

fn get_thread_id(console: &impl ThreadNameRegister) -> ThreadId {
    THREAD_ID.with(|id| {
        let thread_id = id.get();
        if thread_id == 0 {
            let new_id = THREAD_COUNTER.fetch_add(1, Ordering::SeqCst);
            if let Some(name) = thread::current().name() {
                console.register_thread_name(ThreadId(new_id), name.to_string());
            }
            id.set(new_id);
            ThreadId(new_id)
        } else {
            ThreadId(thread_id)
        }
    })
}

trait ThreadNameRegister {
    fn register_thread_name(&self, id: ThreadId, name: String);
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct ThreadId(pub usize);

impl From<ThreadId> for messages::ThreadId {
    fn from(id: ThreadId) -> Self {
        messages::ThreadId { id: id.0 as u64 }
    }
}

#[derive(Debug)]
pub struct Span {
    refcount: AtomicUsize,
    follows: Vec<SpanId>,
}

pub(crate) enum SpanState {
    Active(Span),
    Free { next_id: Option<SpanId> },
}

impl SpanState {
    pub(crate) fn as_active(&self) -> Option<&Span> {
        match self {
            SpanState::Active(span) => Some(span),
            SpanState::Free { .. } => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpanId(NonZeroU64);

impl SpanId {
    fn new(id: u64) -> SpanId {
        SpanId(NonZeroU64::new(id).expect("IDs must be nonzero"))
    }

    fn as_index(&self) -> usize {
        (self.0.get() - 1) as usize
    }

    fn as_span(&self) -> span::Id {
        span::Id::from_u64(self.0.get())
    }

    fn as_message(&self) -> messages::SpanId {
        messages::SpanId { id: self.0.get() }
    }
}
