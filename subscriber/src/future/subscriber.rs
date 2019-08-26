use tracing_core::span;
use tracing_core::Event;
use tracing_core::Subscriber;
use tracing_core::{Interest, Metadata};

use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Once, RwLock};
use std::thread;

use chrono::prelude::*;

use futures::sync::mpsc::UnboundedSender;

use crate::messages::listen_response::Variant;
use crate::messages::Recorder;
use crate::*;

static THREAD_COUNTER: AtomicUsize = AtomicUsize::new(1);

thread_local! {
    static THREAD_ID_INIT: Once = Once::new();
    static THREAD_ID: Cell<usize> = Cell::new(0);

    static STACK: RefCell<Vec<SpanId>> = RefCell::new(Vec::new());
}

fn get_thread_id(console: &ConsoleForwarder) -> ThreadId {
    THREAD_ID_INIT.with(|init_guard| {
        init_guard.call_once(|| {
            THREAD_ID.with(|id| {
                let new_id = THREAD_COUNTER.fetch_add(1, Ordering::SeqCst);
                if let Some(name) = thread::current().name() {
                    console.register_thread_name(ThreadId(new_id), name.to_string());
                }
                id.set(new_id);
            })
        });
        THREAD_ID.with(|id| ThreadId(id.get()))
    })
}

pub struct ConsoleForwarder {
    pub(crate) tx: UnboundedSender<Variant>,
    pub(crate) registry: Arc<RwLock<crate::future::server::Registry>>,
}

impl ConsoleForwarder {
    fn register_thread_name(&self, id: ThreadId, name: String) {
        self.registry.write().unwrap().thread_names.insert(id, name);
    }
}

impl Subscriber for ConsoleForwarder {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }
    fn new_span(&self, span: &span::Attributes) -> span::Id {
        let id = self.registry.write().unwrap().new_id();
        let mut rec = Recorder::default();
        span.record(&mut rec);

        self.tx
            .unbounded_send(Variant::NewSpan(messages::NewSpan {
                attributes: Some(span.into()),
                span: Some(id.as_message()),
                timestamp: Some(messages::Timestamp {
                    nano: Utc::now().timestamp_nanos(),
                }),
                values: rec.0,
            }))
            .expect("Channel not closed");

        id.as_span()
    }
    fn record(&self, span: &span::Id, values: &span::Record) {
        let mut recorder = messages::Recorder::default();
        values.record(&mut recorder);

        self.tx
            .unbounded_send(Variant::Record(messages::Record {
                span: Some(span.into()),
                values: recorder.0,
                thread: Some(get_thread_id(self).into()),
                timestamp: Some(messages::Timestamp {
                    nano: Utc::now().timestamp_nanos(),
                }),
            }))
            .expect("Channel not closed");
    }
    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        self.tx
            .unbounded_send(Variant::Follows(messages::RecordFollowsFrom {
                span: Some(span.into()),
                follows: Some(follows.into()),
            }))
            .expect("Channel not closed");
    }
    fn event(&self, event: &Event) {
        let mut recorder = messages::Recorder::default();
        event.record(&mut recorder);
        let fields = event
            .fields()
            .map(|field| messages::Field {
                name: field.name().to_string(),
            })
            .collect();
        let attributes = messages::Attributes {
            is_contextual: event.is_contextual(),
            is_root: event.is_root(),
            metadata: Some(event.metadata().into()),
            parent: event.parent().map(|p| p.into()),
        };

        self.tx
            .unbounded_send(Variant::Event(messages::Event {
                span: STACK.with(|stack| stack.borrow().last().map(SpanId::as_message)),
                values: recorder.0,
                thread: Some(get_thread_id(self).into()),
                attributes: Some(attributes),
                fields,
                timestamp: Some(messages::Timestamp {
                    nano: Utc::now().timestamp_nanos(),
                }),
            }))
            .expect("Channel not closed");
    }
    fn enter(&self, span: &span::Id) {
        STACK.with(|stack| stack.borrow_mut().push(SpanId::new(span.into_u64())))
    }
    fn exit(&self, _: &span::Id) {
        STACK.with(|stack| stack.borrow_mut().pop());
    }
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        match self.enabled(metadata) {
            true => Interest::always(),
            false => Interest::never(),
        }
    }
    fn clone_span(&self, id: &span::Id) -> span::Id {
        let index = SpanId::new(id.into_u64()).as_index();
        self.registry.read().unwrap().spans[index]
            .as_active()
            .expect("SpanId points to active span")
            .refcount
            .fetch_add(1, Ordering::SeqCst);
        id.clone()
    }
    fn drop_span(&self, ref id: span::Id) {
        let span_id = SpanId::new(id.into_u64());
        let index = span_id.as_index();
        let old_count = try_lock!(self.registry.read()).spans[index]
            .as_active()
            .expect("SpanId points to active span")
            .refcount
            .fetch_sub(1, Ordering::SeqCst);
        if old_count == 1 {
            let mut registry = try_lock!(self.registry.write());
            let next_id = std::mem::replace(&mut registry.next_id, Some(span_id));
            std::mem::replace(&mut registry.spans[index], SpanState::Free { next_id });
        }
    }
}
