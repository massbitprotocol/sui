use std::{
    env,
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    let out_dir = if env::var("DUMP_GENERATED_GRPC").is_ok() {
        PathBuf::from("")
    } else {
        PathBuf::from(env::var("OUT_DIR")?)
    };
    build_gg20_service(&out_dir)?;
    build_multisig_service(&out_dir)?;
    Ok(())
}
fn build_gg20_service(out_dir: &PathBuf) -> Result<()> {
    let gg20_service = anemo_build::manual::Service::builder()
        .name("Gg20Peer")
        .package("gg20")
        .method(
            anemo_build::manual::Method::builder()
                .name("keygen")
                .route_name("KeyGen")
                .request_type("types::gg20::KeygenRequest")
                .response_type("types::gg20::KeygenResponse")
                .codec_path("anemo::rpc::codec::BincodeCodec")
                // .codec_path("anemo::rpc::codec::JsonCodec")
                // .server_handler_return_raw_bytes(true)
                .build(),
        )
        .method(
            anemo_build::manual::Method::builder()
                .name("sign")
                .route_name("Sign")
                .request_type("types::gg20::SignRequest")
                .response_type("types::gg20::SignResponse")
                .codec_path("anemo::rpc::codec::BincodeCodec")
                .build(),
        )
        .method(
            anemo_build::manual::Method::builder()
                .name("recover")
                .route_name("Recover")
                .request_type("types::gg20::RecoverRequest")
                .response_type("types::gg20::RecoverResponse")
                .codec_path("anemo::rpc::codec::BincodeCodec")
                .build(),
        )
        .method(
            anemo_build::manual::Method::builder()
                .name("keypresence")
                .route_name("KeyPresence")
                .request_type("types::gg20::KeyPresenceRequest")
                .response_type("types::gg20::KeyPresenceResponse")
                .codec_path("anemo::rpc::codec::BincodeCodec")
                .build(),
        )
        .build();
    anemo_build::manual::Builder::new()
        .out_dir(out_dir)
        .compile(&[gg20_service]);
    Ok(())
}

fn build_multisig_service(out_dir: &PathBuf) -> Result<()> {
    let multisig_service = anemo_build::manual::Service::builder()
        .name("MultisigPeer")
        .package("multisig")
        .method(
            anemo_build::manual::Method::builder()
                .name("keygen")
                .route_name("KeyGen")
                .request_type("types::multisig::KeygenRequest")
                .response_type("types::multisig::KeygenResponse")
                .codec_path("anemo::rpc::codec::BincodeCodec")
                // .codec_path("anemo::rpc::codec::JsonCodec")
                // .server_handler_return_raw_bytes(true)
                .build(),
        )
        .method(
            anemo_build::manual::Method::builder()
                .name("sign")
                .route_name("Sign")
                .request_type("types::multisig::SignRequest")
                .response_type("types::multisig::SignResponse")
                .codec_path("anemo::rpc::codec::BincodeCodec")
                .build(),
        )
        .method(
            anemo_build::manual::Method::builder()
                .name("keypresence")
                .route_name("KeyPresence")
                .request_type("types::multisig::KeyPresenceRequest")
                .response_type("types::multisig::KeyPresenceResponse")
                .codec_path("anemo::rpc::codec::BincodeCodec")
                .build(),
        )
        .build();
    anemo_build::manual::Builder::new()
        .out_dir(out_dir)
        .compile(&[multisig_service]);
    Ok(())
}
