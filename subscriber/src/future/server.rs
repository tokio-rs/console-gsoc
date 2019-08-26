use std::sync::{Arc, Mutex, RwLock};

use crate::future::*;
use crate::messages::listen_response::Variant;
use crate::*;

use futures::sink::{Sink, Wait};
use futures::sync::mpsc::*;
use futures::Future;
use futures::Stream;

use tower_hyper::server::{Http, Server};

use tower_grpc::codegen::server::grpc::{Request, Response};

use tokio::net::TcpListener;

#[derive(Default)]
pub(crate) struct Registry {
    pub spans: Vec<SpanState>,
    pub next_id: Option<SpanId>,

    pub thread_names: HashMap<ThreadId, String>,
}

impl Registry {
    pub(crate) fn new_id(&mut self) -> SpanId {
        let span = SpanState::Active(Span {
            refcount: AtomicUsize::new(1),
            follows: vec![],
        });
        if let Some(id) = &self.next_id {
            let old_span = std::mem::replace(&mut self.spans[id.as_index()], span);
            match old_span {
                SpanState::Free { next_id } => {
                    let id = id.clone();
                    std::mem::replace(&mut self.next_id, next_id);
                    id
                }
                _ => unreachable!("BUG: next_id points to active span"),
            }
        } else {
            self.spans.push(span);
            SpanId::new(self.spans.len() as u64)
        }
    }
}

#[derive(Clone)]
/// A factory for ConsoleForwarder
pub struct GrpcEndpoint {
    sender: UnboundedSender<Variant>,
    tx: Arc<Mutex<Vec<Wait<UnboundedSender<messages::ListenResponse>>>>>,

    registry: Arc<RwLock<crate::future::server::Registry>>,
}

impl GrpcEndpoint {
    pub fn new() -> (
        GrpcEndpoint,
        impl Future<Item = (), Error = ()> + Send + Sync + 'static,
    ) {
        let (sender, rx) = unbounded();
        let handle = GrpcEndpoint {
            sender,
            tx: Arc::default(),
            registry: Arc::default(),
        };
        let stream_tx = handle.tx.clone();

        let future = rx
            .for_each(move |message| {
                let mut senders = stream_tx.lock().unwrap();
                let mut closed = vec![];
                for (i, sender) in senders.iter_mut().enumerate() {
                    let response = messages::ListenResponse {
                        variant: Some(message.clone()),
                    };
                    if sender.send(response).is_err() {
                        // Connection reset, mark for removal
                        closed.push(i);
                    }
                }
                // Traverse in reverse order, to keep index valid during removal
                for &i in closed.iter().rev() {
                    let _ = senders.remove(i);
                }
                Ok(())
            })
            .map_err(|_| {});
        (handle, future)
    }

    pub fn into_server(
        self,
        addr: &str,
    ) -> impl Future<Item = (), Error = ()> + Send + Sync + 'static {
        let service = messages::server::ConsoleForwarderServer::new(self);
        let mut server = Server::new(service);
        let http = Http::new().http2_only(true).clone();

        let bind = TcpListener::bind(&addr.parse().expect("Invalid address")).expect("bind");

        bind.incoming()
            .for_each(move |sock| {
                if let Err(e) = sock.set_nodelay(true) {
                    return Err(e);
                }

                let serve = server.serve_with(sock, http.clone());
                tokio::spawn(serve.map_err(|_| {
                    // Ignore connection reset
                }));

                Ok(())
            })
            .map_err(|e| eprintln!("accept error: {}", e))
    }

    pub fn new_subscriber(&self) -> ConsoleForwarder {
        ConsoleForwarder {
            tx: self.sender.clone(),
            registry: self.registry.clone(),
        }
    }
}

impl messages::server::ConsoleForwarder for GrpcEndpoint {
    type ListenStream =
        Box<dyn Stream<Item = messages::ListenResponse, Error = tower_grpc::Status> + Send + Sync>;
    type ListenFuture =
        futures::future::FutureResult<Response<Self::ListenStream>, tower_grpc::Status>;

    fn listen(&mut self, _request: Request<messages::ListenRequest>) -> Self::ListenFuture {
        let (tx, rx) = unbounded();
        self.tx.lock().unwrap().push(tx.wait());
        let rx = rx.map_err(|_| unimplemented!(""));
        futures::future::ok(Response::new(Box::new(rx)))
    }
}
