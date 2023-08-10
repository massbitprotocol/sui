fn main() {
    println!("cargo:rerun-if-changed=proto");
    tonic_build::configure()
        .out_dir("src/proto")
        .compile(&["proto/scalar.proto", "proto/abci.proto"], &["proto"])
        .expect("Failed to compile proto(s)");
    // build_json_codec_service();
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
