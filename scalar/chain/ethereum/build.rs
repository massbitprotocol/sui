fn main() {
    println!("cargo:rerun-if-changed=proto");

    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .out_dir("src/protobuf")
        .compile(
            &[
                "proto/codec.proto",
                "proto/sfcodec.proto",
                "proto/axelar/evm/v1beta1/events.proto",
                "proto/axelar/evm/v1beta1/genesis.proto",
                "proto/axelar/evm/v1beta1/params.proto",
                "proto/axelar/evm/v1beta1/query.proto",
                "proto/axelar/evm/v1beta1/service.proto",
                "proto/axelar/evm/v1beta1/tx.proto",
                "proto/axelar/evm/v1beta1/types.proto",
            ],
            &["proto"],
        )
        .expect("Failed to compile Firehose Ethereum proto(s)");
}
