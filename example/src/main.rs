use console_subscriber::*;

use std::thread;
use std::time::Duration;

fn main() {
    let handle = BackgroundThreadHandle::new();
    let subscriber = handle.new_subscriber();

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
    handle.run_background("[::1]:50051").join().unwrap();
}
