use console_subscriber::future::GrpcEndpoint;

use std::thread;
use std::time::Duration;

use tokio::prelude::*;
use tokio::runtime::Runtime;

fn main() {
    let (handle, future) = GrpcEndpoint::new();
    let subscriber = handle.new_subscriber();
    
    let mut rt = Runtime::new().unwrap();
    rt.spawn(future);

    thread::Builder::new()
        .name("Server".to_string())
        .spawn(|| {
            tracing::subscriber::with_default(subscriber, || {
                thread::sleep(Duration::from_millis(1000));
                let kind = tracing_test::ApplicationKind::YakShave;
                loop {
                    thread::sleep(Duration::from_millis(2000));
                    println!("Emitting");
                    kind.emit();
                }
            });
        })
        .expect("Couldn't start background thread");

    rt.spawn(handle.into_server("[::1]:50051"));
    rt.shutdown_on_idle().wait().unwrap();
}
