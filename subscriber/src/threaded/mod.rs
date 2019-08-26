
//! The subscriber spawns two threads two manage the endpoint.
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

mod server;
mod subscriber;

pub use server::*;
pub use subscriber::*;