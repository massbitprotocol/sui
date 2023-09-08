fn main() {
    println!("cargo:rerun-if-changed=proto");
    tonic_build::configure()
        .out_dir("src/proto")
        .compile(
            &[
                "proto/scalar.proto",
                "proto/abci.proto",
                "proto/tofnd.proto",
                "proto/multisig.proto",
            ],
            &["proto"],
        )
        .expect("Failed to compile proto(s)");
    build_tss_service();
    // build_json_codec_service();
}

fn build_tss_service() {
    let tss_service = anemo_build::manual::Service::builder()
        .name("TssPeer")
        .package("tss.network")
        // .method(
        //     anemo_build::manual::Method::builder()
        //         .name("say_hello")
        //         .route_name("SayHello")
        //         .request_type("crate::HelloRequest")
        //         .response_type("crate::HelloResponse")
        //         .codec_path("anemo::rpc::codec::BincodeCodec")
        //         // .codec_path("anemo::rpc::codec::JsonCodec")
        //         // .server_handler_return_raw_bytes(true)
        //         .build(),
        // )
        .method(
            anemo_build::manual::Method::builder()
                .name("keygen")
                .route_name("KeyGen")
                .request_type("crate::TssAnemoKeygenRequest")
                .response_type("crate::TssAnemoKeygenResponse")
                .codec_path("anemo::rpc::codec::BincodeCodec")
                // .codec_path("anemo::rpc::codec::JsonCodec")
                // .server_handler_return_raw_bytes(true)
                .build(),
        )
        .method(
            anemo_build::manual::Method::builder()
                .name("sign")
                .route_name("Sign")
                .request_type("crate::TssAnemoSignRequest")
                .response_type("crate::TssAnemoSignResponse")
                .codec_path("anemo::rpc::codec::BincodeCodec")
                // .codec_path("anemo::rpc::codec::JsonCodec")
                // .server_handler_return_raw_bytes(true)
                .build(),
        )
        .method(
            anemo_build::manual::Method::builder()
                .name("verify")
                .route_name("Verify")
                .request_type("crate::TssAnemoVerifyRequest")
                .response_type("crate::TssAnemoVerifyResponse")
                .codec_path("anemo::rpc::codec::BincodeCodec")
                // .codec_path("anemo::rpc::codec::JsonCodec")
                // .server_handler_return_raw_bytes(true)
                .build(),
        )
        .build();

    anemo_build::manual::Builder::new().compile(&[tss_service]);
}
// Manually define the json.helloworld.Greeter service which used a custom JsonCodec to use json
// serialization instead of protobuf for sending messages on the wire.
// This will result in generated client and server code which relies on its request, response and
// codec types being defined in a module `crate::common`.
//
// See the client/server examples defined in `src/json-codec` for more information.
// fn build_json_codec_service() {
//     let greeter_service = tonic_build::manual::Service::builder()
//         .name("Greeter")
//         .package("json.helloworld")
//         .method(
//             tonic_build::manual::Method::builder()
//                 .name("say_hello")
//                 .route_name("SayHello")
//                 .input_type("crate::common::HelloRequest")
//                 .output_type("crate::common::HelloResponse")
//                 .codec_path("crate::common::JsonCodec")
//                 .build(),
//         )
//         .build();

//     tonic_build::manual::Builder::new().compile(&[greeter_service]);
// }
