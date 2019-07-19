use tracing_core::field::Visit;
use tracing_core::span::{self, Attributes, Record};
use tracing_core::Event;
use tracing_core::{Field, Level};

use std::collections::HashMap;
use std::fmt::{Debug, Write};

#[derive(Debug, Default)]
pub struct ThreadStore {
    pub lines: Vec<EventEntry>,
    pub name: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventEntry {
    level: Level,
    collected_fields: String,
}

impl EventEntry {
    pub fn level(&self) -> &Level {
        &self.level
    }

    pub fn display(&self) -> &str {
        &self.collected_fields
    }
}

/// Modelled after `tokio_trace::Subscriber`
///
/// Some methods differ in that they also take a thread id.
/// This is because the specific thread in which they execute is important.
///
/// The thread id cannot be assumed to be implicitly given by the calling thread,
/// since this information might be sourced out of process.
///
#[derive(Debug)]
pub struct Store {
    stacks: HashMap<ThreadId, Vec<span::Id>>,
    pub threads: HashMap<ThreadId, ThreadStore>,
    spans: Vec<Span>,
    reusable: Vec<span::Id>,
    updated: bool,
}

impl Store {
    pub fn new() -> Store {
        Store {
            stacks: HashMap::new(),
            threads: HashMap::new(),
            spans: Vec::new(),
            reusable: Vec::new(),
            updated: false,
        }
    }

    pub fn register_thread_name(&mut self, thread: ThreadId, name: String) {
        let store = self.threads.entry(thread).or_default();
        store.name = Some(name);
    }

    pub fn updated(&self) -> bool {
        self.updated
    }

    pub fn clear(&mut self) {
        self.updated = false;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct ThreadId(pub usize);

impl ThreadId {
    pub(crate) fn new(id: usize) -> ThreadId {
        ThreadId(id)
    }
}

#[derive(Debug)]
struct Span {
    id: span::Id,
    ref_count: u64,
    follows: Vec<span::Id>,
}

impl Span {
    fn clear(&mut self) {
        self.ref_count = 1;
        self.follows.clear();
    }
}

impl Store {
    fn id_to_index(id: &span::Id) -> usize {
        (id.into_u64() - 1) as usize
    }

    pub(crate) fn new_span(&mut self, _span: &Attributes) -> span::Id {
        if let Some(id) = self.reusable.pop() {
            id
        } else {
            let id = span::Id::from_u64((self.spans.len() + 1) as u64);
            self.spans.push(Span {
                id: id.clone(),
                ref_count: 1,
                follows: Vec::new(),
            });
            id
        }
    }
    pub(crate) fn clone_span(&mut self, span_id: &span::Id) -> span::Id {
        self.spans[Store::id_to_index(span_id)].ref_count += 1;
        span_id.clone()
    }
    pub(crate) fn drop_span(&mut self, span_id: span::Id) {
        let mut span = &mut self.spans[Store::id_to_index(&span_id)];
        span.ref_count -= 1;
        if span.ref_count == 0 {
            self.reusable.push(span_id.clone());
            span.clear();
        }
    }
    pub(crate) fn record_follows_from(&mut self, span_id: &span::Id, follows: &span::Id) {
        self.updated = true;
        let span = &mut self.spans[Store::id_to_index(span_id)];
        span.follows.push(follows.clone());
    }

    pub(crate) fn record(&mut self, _thread: ThreadId, _span_id: &span::Id, _values: &Record) {
        self.updated = true;
        unimplemented!("Record")
    }
    pub(crate) fn enter(&mut self, thread: ThreadId, span_id: &span::Id) {
        self.stacks
            .entry(thread)
            .or_insert(Vec::new())
            .push(span_id.clone());
    }
    pub(crate) fn exit(&mut self, thread: ThreadId, _span_id: &span::Id) {
        let stack = self.stacks.entry(thread).or_insert(Vec::new());
        stack.pop().expect("Popped an already empty thread");
        // Prevent oom caused by many short lived threads that don't contain a span anymore
        // In case a new span enters in such a thread thread,
        // the entry will simply get renewed in `enter`
        if stack.len() == 0 {
            self.stacks.remove(&thread);
        }
    }

    pub(crate) fn event(&mut self, thread: ThreadId, event: &Event) {
        self.updated = true;
        let store = self.threads.entry(thread).or_default();
        let mut formatter = StringFormatter {
            buffer: String::new(),
        };
        event.record(&mut formatter);
        store.lines.push(EventEntry {
            level: event.metadata().level().clone(),
            collected_fields: formatter.buffer,
        });
    }
}

struct StringFormatter {
    buffer: String,
}

impl Visit for StringFormatter {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        write!(self.buffer, r#"{}("{:?}")"#, field.name(), value).expect("Formatting failed");
    }
}
