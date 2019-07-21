use std::thread;

use console::storage::*;
use console::ui;

fn main() -> Result<(), failure::Error> {
    // Share store between the gRPC client and the app
    let grpc_handle = StoreHandle::default();
    let app_handle = grpc_handle.clone();

    // Fetch events, spans, etc.
    thread::spawn(|| console::connection::listen(grpc_handle, "http://[::1]:50051"));

    let mut app = ui::App::new(app_handle)?;
    app.run()?;

    Ok(())
}
