use crate::store::{Store, ThreadId};
use tokio_trace_core::span::{Attributes, Id, Record};
use tokio_trace_core::{Event, Metadata, Subscriber};

use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Once};

const THREAD_COUNTER: AtomicUsize = AtomicUsize::new(1);

thread_local! {
    static THREAD_ID_INIT: Once = Once::new();
    static THREAD_ID: Cell<usize> = Cell::new(0);
}

fn get_thread_id() -> ThreadId {
    THREAD_ID_INIT.with(|init_guard| {
        init_guard.call_once(|| {
            THREAD_ID.with(|id| {
                // TODO: Maybe reuse thread ids? - We don't know when a thread is dead
                let new_id = THREAD_COUNTER.fetch_add(1, Ordering::Relaxed);
                id.set(new_id);
            })
        });
        THREAD_ID.with(|id| ThreadId::new(id.get()))
    })
}

pub struct InProcessStore {
    store: Arc<Mutex<Store>>,
}

impl InProcessStore {
    pub fn new(store: Arc<Mutex<Store>>) -> InProcessStore {
        InProcessStore { store }
    }
}

impl Subscriber for InProcessStore {
    fn enabled(&self, metadata: &Metadata) -> bool {
        true
    }
    fn new_span(&self, span: &Attributes) -> Id {
        self.store.lock().unwrap().new_span(span)
    }
    fn record(&self, span: &Id, values: &Record) {
        self.store
            .lock()
            .unwrap()
            .record(get_thread_id(), span, values)
    }
    fn record_follows_from(&self, span: &Id, follows: &Id) {
        self.store
            .lock()
            .unwrap()
            .record_follows_from(span, follows)
    }
    fn event(&self, event: &Event) {
        self.store.lock().unwrap().event(get_thread_id(), event)
    }
    fn enter(&self, span: &Id) {
        self.store.lock().unwrap().enter(get_thread_id(), span)
    }
    fn exit(&self, span: &Id) {
        self.store.lock().unwrap().exit(get_thread_id(), span)
    }
    fn clone_span(&self, id: &Id) -> Id {
        self.store.lock().unwrap().clone_span(id)
    }
    fn drop_span(&self, id: Id) {
        self.store.lock().unwrap().drop_span(id)
    }
}
