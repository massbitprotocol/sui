fn main() {
    println!("cargo:rerun-if-changed=proto");
    tonic_build::configure()
        .out_dir("src/proto")
        .compile(&["proto/abci.proto"], &["proto"])
        .expect("Failed to compile proto(s)");
    // build_json_codec_service();
}
