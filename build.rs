fn main() {
    prost_build::compile_protos(&["proto/nexusroot.proto"], &["proto/"])
        .expect("Failed to compile protobuf");
}
