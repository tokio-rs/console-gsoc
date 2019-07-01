use crate::store::{Store, ThreadId};
use tracing_core::span::{Attributes, Id, Record};
use tracing_core::{Event, Metadata, Subscriber};

use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Once};
use std::thread;

static THREAD_COUNTER: AtomicUsize = AtomicUsize::new(1);

thread_local! {
    static THREAD_ID_INIT: Once = Once::new();
    static THREAD_ID: Cell<usize> = Cell::new(0);
}

pub struct InProcessStore {
    store: Arc<Mutex<Store>>,
}

fn get_thread_id(store: &mut Store) -> ThreadId {
    THREAD_ID_INIT.with(|init_guard| {
        init_guard.call_once(|| {
            THREAD_ID.with(|id| {
                // TODO: Maybe reuse thread ids? - We don't know when a thread is dead
                let new_id = THREAD_COUNTER.fetch_add(1, Ordering::SeqCst);
                if let Some(name) = thread::current().name() {
                    store.register_thread_name(ThreadId::new(new_id), name.to_string());
                }
                id.set(new_id);
            })
        });
        THREAD_ID.with(|id| ThreadId::new(id.get()))
    })
}

impl InProcessStore {
    pub fn new(store: Arc<Mutex<Store>>) -> InProcessStore {
        InProcessStore { store }
    }
}

impl Subscriber for InProcessStore {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }
    fn new_span(&self, span: &Attributes) -> Id {
        self.store.lock().unwrap().new_span(span)
    }
    fn record(&self, span: &Id, values: &Record) {
        let mut store = self.store.lock().unwrap();
        let id = get_thread_id(&mut store);
        store.record(id, span, values)
    }
    fn record_follows_from(&self, span: &Id, follows: &Id) {
        self.store
            .lock()
            .unwrap()
            .record_follows_from(span, follows)
    }
    fn event(&self, event: &Event) {
        let mut store = self.store.lock().unwrap();
        let id = get_thread_id(&mut store);
        store.event(id, event)
    }
    fn enter(&self, span: &Id) {
        let mut store = self.store.lock().unwrap();
        let id = get_thread_id(&mut store);
        store.enter(id, span)
    }
    fn exit(&self, span: &Id) {
        let mut store = self.store.lock().unwrap();
        let id = get_thread_id(&mut store);
        store.exit(id, span)
    }
    fn clone_span(&self, id: &Id) -> Id {
        self.store.lock().unwrap().clone_span(id)
    }
    fn drop_span(&self, id: Id) {
        self.store.lock().unwrap().drop_span(id)
    }
}
