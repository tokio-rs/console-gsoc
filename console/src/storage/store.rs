use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::storage::messages::listen_response::Variant;
use crate::storage::messages::*;

/// # IDs
/// The subscriber obviously want to reuse span ids, to preserve memory
/// The console however, must preserve history.
/// As a result, we internally assign and map our own ids.
/// When a span id is reused in the subscriber, a new span message is send.
/// This replaces the entry in the `id_map` and a new internal id is assigned.
///
/// The console itself won't reuse ids.
/// In the future, old/unused span information will be flushed to disk.
/// Currently, the console doesn't do such kind of memory optimization.
#[derive(Debug, Default)]
pub struct Store {
    events: Vec<EventEntry>,
    spans: Vec<Span>,

    updated: bool,
    id_counter: usize,
    id_map: HashMap<u64, InternalId>,
}

impl Store {
    pub fn new() -> Store {
        Store::default()
    }

    pub fn updated(&self) -> bool {
        self.updated
    }

    pub fn clear(&mut self) {
        self.updated = false;
    }

    pub fn events(&self) -> &[EventEntry] {
        &self.events
    }

    pub fn spans(&self) -> &[Span] {
        &self.spans
    }
}

/// See `Store` documentation
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InternalId(pub usize);

#[derive(Debug)]
pub struct Span {
    pub id: InternalId,
    pub span: NewSpan,
    pub parent_id: Option<InternalId>,

    records: Vec<Record>,
    follows: Vec<SpanId>,
}

impl ValueContainer for Span {
    fn value_by_name(&self, name: &str) -> Option<&value::Value> {
        self.span.value_by_name(name).or_else(|| {
            self.records
                .iter()
                .find_map(|record| record.value_by_name(name))
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventEntry {
    pub span: Option<InternalId>,
    pub event: Event,
}

impl EventEntry {
    pub fn level(&self) -> Option<Level> {
        Level::from_i32(self.event.attributes.as_ref()?.metadata.as_ref()?.level)
    }
}

/// Convenience Wrapper around `Arc<Mutex<Store>>`
#[derive(Clone, Default)]
pub struct StoreHandle(pub Arc<Mutex<Store>>);

impl StoreHandle {
    pub fn new() -> StoreHandle {
        StoreHandle::default()
    }

    /// Locks and updates the underlying `Store`
    pub fn handle(&self, variant: Variant) {
        let mut store = self.0.lock().unwrap();
        match variant {
            Variant::NewSpan(span) => store.new_span(span),
            Variant::Record(record) => store.record(record),
            Variant::Follows(follows) => store.record_follows_from(follows),
            Variant::Event(event) => store.event(event),
        }
    }
}

impl Store {
    fn new_span(&mut self, span: NewSpan) {
        // Update id mapping for span, see `Store` documentation
        self.id_map.insert(
            span.span
                .as_ref()
                .expect("BUG: No id assined to NewSpan")
                .id,
            InternalId(self.id_counter),
        );
        let parent_id = span
            .attributes
            .as_ref()
            .and_then(|attrs: &Attributes| attrs.parent.as_ref())
            .and_then(|id| self.id_map.get(&id.id))
            .cloned();

        self.spans.push(Span {
            id: InternalId(self.id_counter),
            span,
            parent_id,
            records: vec![],
            follows: vec![],
        });
        self.id_counter += 1;
    }

    fn record_follows_from(&mut self, follows: RecordFollowsFrom) {
        let span = self.id_map[&follows
            .span
            .as_ref()
            .expect("BUG: No id set on follows.span")
            .id];
        self.spans[span.0]
            .follows
            .push(follows.follows.expect("BUG: No id set on follows.follows"));
    }

    fn record(&mut self, record: Record) {
        self.updated = true;
        let span = self.id_map[&record
            .span
            .as_ref()
            .expect("BUG: No id set on record.span")
            .id];
        self.spans[span.0].records.push(record);
    }

    fn event(&mut self, event: Event) {
        self.updated = true;
        self.events.push(EventEntry {
            span: event.span.as_ref().map(|span| self.id_map[&span.id]),
            event,
        });
    }
}
