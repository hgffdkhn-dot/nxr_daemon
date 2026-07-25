fn main() {
    prost_build::compile_protos(
        &["../proto/nexusroot.proto"],
        &["../proto/"],
    )
    .unwrap();
}
