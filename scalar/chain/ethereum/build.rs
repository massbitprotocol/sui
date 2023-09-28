use ethers::{
    prelude::{abigen, Abigen},
    providers::{Http, Provider},
    types::Address,
};
use ethers_solc::{utils, Project, ProjectPathsConfig};
use eyre::Result;
use std::sync::Arc;
fn main() {
    println!("cargo:rerun-if-changed=proto");
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .finish();

    // use that subscriber to process traces emitted after this point
    // let subscriber = FmtSubscriber::builder()
    //     .with_max_level(Level::INFO)
    //     .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    compile_project_sols();
    match abi_generator() {
        Ok(_) => {
            println!("Generate abis successfully");
        }
        Err(e) => {
            panic!("Generate abis error {:?}", &e);
        }
    }
    // tonic_build::configure()
    //     .protoc_arg("--experimental_allow_proto3_optional")
    //     .out_dir("src/protobuf")
    //     .compile(
    //         &[
    //             "proto/codec.proto",
    //             "proto/sfcodec.proto",
    //             "proto/axelar/evm/v1beta1/events.proto",
    //             "proto/axelar/evm/v1beta1/genesis.proto",
    //             "proto/axelar/evm/v1beta1/params.proto",
    //             "proto/axelar/evm/v1beta1/query.proto",
    //             "proto/axelar/evm/v1beta1/service.proto",
    //             "proto/axelar/evm/v1beta1/tx.proto",
    //             "proto/axelar/evm/v1beta1/types.proto",
    //         ],
    //         &["proto"],
    //     )
    //     .expect("Failed to compile Firehose Ethereum proto(s)");
}

fn compile_project_sols() {
    // configure the project with all its paths, solc, cache etc.
    let root = utils::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap();
    let solidity = root.join("solidity");
    let paths = ProjectPathsConfig::builder()
        .root(&solidity)
        .sources(&solidity.join("src"))
        .tests(solidity.join("tests"))
        .scripts(solidity.join("scripts"))
        //.lib(root.join("lib1"))
        //.lib(root.join("lib2"))
        .build()
        .unwrap();
    // let paths = ProjectPathsConfig::hardhat(env_dir);
    println!("Paths {:?}", &paths);
    // println!("Input files {:?}", paths.read_input_files().unwrap());
    let project = Project::builder()
        .paths(paths)
        .no_artifacts()
        .ephemeral()
        .no_artifacts()
        .build()
        .unwrap();
    match project.compile() {
        Ok(res) => {
            println!("Compile result {:?}", res);
        }
        Err(e) => {
            panic!("Compile error {:?}", e);
        }
    }

    // Tell Cargo that if a source file changes, to rerun this build script.
    project.rerun_if_sources_changed();
}

fn abi_generator() -> Result<()> {
    let abi_source =
        "./solidity/artifacts/contracts/call-contract/ExecutableSample.sol/ExecutableSample.json";
    let root = utils::canonicalize(env!("CARGO_MANIFEST_DIR"))?.join("src/abis");
    let out_file = root.join("ExecutableSample.rs");
    if out_file.exists() {
        std::fs::remove_file(&out_file)?;
    }
    Abigen::new("ExecutableSample", abi_source)?
        .generate()?
        .write_to_file(out_file)?;
    Ok(())
}
