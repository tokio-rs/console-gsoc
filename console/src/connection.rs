use crate::storage::*;

use futures::{Future, Stream};

use hyper::client::connect::{Destination, HttpConnector};

use tower_grpc::Request;
use tower_hyper::{client, util};
use tower_util::MakeService;

/// Connects to the remote endpoint
/// Internally locks and updates the `Store`
///
/// Blocks until the connection is reset by the endpoint
pub fn listen(store: StoreHandle, addr: &str) {
    let uri: http::Uri = addr.parse().unwrap();

    let dst = Destination::try_from_uri(uri.clone()).unwrap();
    let connector = util::Connector::new(HttpConnector::new(4));
    let settings = client::Builder::new().http2_only(true).clone();
    let mut make_client = client::Connect::with_builder(connector, settings);

    let fetch_events = make_client
        .make_service(dst)
        .map_err(|e| panic!("connect error: {:?}", e))
        .and_then(move |conn| {
            use messages::client::ConsoleForwarder;

            let conn = tower_request_modifier::Builder::new()
                .set_origin(uri)
                .build(conn)
                .unwrap();

            // Wait until the client is ready...
            ConsoleForwarder::new(conn).ready()
        })
        .and_then(|mut client| client.listen(Request::new(ListenRequest {})))
        .and_then(move |stream_response| {
            stream_response.into_inner().for_each(move |response| {
                store.handle(response.variant.expect("No variant on response"));
                Ok(())
            })
        })
        .map_err(|_| {
            // TODO: Errors like connection reset are ignored for now
        });

    tokio::run(fetch_events);
}
