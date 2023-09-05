fn main() {
    // println!("cargo:rerun-if-changed=proto");
    // tonic_build::configure()
    //     .out_dir("src/proto")
    //     .compile(&["proto/tofnd.proto"], &["proto"])
    //     .expect("Failed to compile proto(s)");
    // let greeter_service = anemo_build::manual::Service::builder()
    //     .name("Greeter")
    //     .package("example.helloworld")
    //     .method(
    //         anemo_build::manual::Method::builder()
    //             .name("say_hello")
    //             .route_name("SayHello")
    //             .request_type("crate::HelloRequest")
    //             .response_type("crate::HelloResponse")
    //             .codec_path("anemo::rpc::codec::BincodeCodec")
    //             // .codec_path("anemo::rpc::codec::JsonCodec")
    //             // .server_handler_return_raw_bytes(true)
    //             .build(),
    //     )
    //     .build();
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
                .request_type("crate::KeygenRequest")
                .response_type("crate::KeygenResponse")
                .codec_path("anemo::rpc::codec::BincodeCodec")
                // .codec_path("anemo::rpc::codec::JsonCodec")
                // .server_handler_return_raw_bytes(true)
                .build(),
        )
        .method(
            anemo_build::manual::Method::builder()
                .name("sign")
                .route_name("Sign")
                .request_type("crate::SignRequest")
                .response_type("crate::SignResponse")
                .codec_path("anemo::rpc::codec::BincodeCodec")
                // .codec_path("anemo::rpc::codec::JsonCodec")
                // .server_handler_return_raw_bytes(true)
                .build(),
        )
        .method(
            anemo_build::manual::Method::builder()
                .name("verify")
                .route_name("Verify")
                .request_type("crate::VerifyRequest")
                .response_type("crate::VerifyResponse")
                .codec_path("anemo::rpc::codec::BincodeCodec")
                // .codec_path("anemo::rpc::codec::JsonCodec")
                // .server_handler_return_raw_bytes(true)
                .build(),
        )
        .build();

    anemo_build::manual::Builder::new().compile(&[tss_service]);
}
