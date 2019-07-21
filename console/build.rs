fn main() {
    tower_grpc_build::Config::new()
        .enable_client(true)
        .enable_server(false)
        .build(&["../proto/tracing.proto"], &["../proto/"])
        .unwrap_or_else(|e| panic!("protobuf compilation failed: {}", e));
    println!("cargo:rerun-if-changed=../proto/tracing.proto");
}
