//! A remote endpoint for `tracing-console`
//!
//! The subscriber currently spawns two threads two manage the endpoint.
//!
//! When an application interacts with `tracing`, under the hood,
//! the current subscriber is called.
//! This happens, for example, when a span is created (`span!(...)`)
//! or an event is issued (`warn!("Cookies are empty")`).
//!
//! Those calls get translated to a message and sent to an aggregator thread.
//! This aggregator thread then passes those messages to a network thread,
//! which communicates with the client/console.
//!
//! # Network
//! The following information will not be send to the console, but tracked locally:
//!  - `span.enter()/exit()`, tracked via Thread-Local-Storage.
//!  - `span.clone()/` and dropping, currently involves a mutex access
//!  
//! # Thread overview:
//!
//! ```schematic,ignore
//! ┌──────────────────┐ span!(...) ┌───────────────────┐
//! │Application Thread│----------->│                   │
//! └──────────────────┘            │                   │
//! ┌──────────────────┐ warn!(...) │                   │      ┌──────────────────┐
//! │Application Thread│----------->│ Aggregator Thread │----->│  Network Thread  │
//! └──────────────────┘            │                   │      └──────────────────┘
//! ┌──────────────────┐ debug!(..) │                   │
//! │Application Thread│----------->│                   │
//! └──────────────────┘            └───────────────────┘
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! # fn main() {
//! use console_subscriber::BackgroundThreadHandle;
//! use std::thread;
//!
//! let handle = BackgroundThreadHandle::new();
//! let subscriber = handle.new_subscriber();
//! std::thread::spawn(|| {
//!     tracing::subscriber::with_default(subscriber, || {
//!         use tracing::{event, Level};
//!
//!         event!(Level::INFO, "something has happened!");
//!     });
//! });
//!
//! handle.run_background("[::1]:50051").join().unwrap();
//! # }
//! ```

mod messages;
mod server;
mod subscriber;

use tracing_core::span;

use std::collections::HashMap;
use std::num::NonZeroU64;

pub use server::*;

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct ThreadId(pub usize);

impl From<ThreadId> for messages::ThreadId {
    fn from(id: ThreadId) -> Self {
        messages::ThreadId { id: id.0 as u64 }
    }
}

#[derive(Debug)]
pub struct Span {
    refcount: usize,
    follows: Vec<SpanId>,
}

#[derive(Debug)]
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
