use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");

    let proto_root = PathBuf::from("../protobuf/src");
    let proto2 = proto_root.join("google/protobuf/test_messages_proto2.proto");
    let proto3 = proto_root.join("google/protobuf/test_messages_proto3.proto");

    assert!(
        !(!proto2.exists() || !proto3.exists()),
        "conformance protos not found; ensure protobuf submodule is present"
    );

    println!("cargo:rerun-if-changed={}", proto2.display());
    println!("cargo:rerun-if-changed={}", proto3.display());
    println!(
        "cargo:rerun-if-changed={}",
        proto_root.join("google/protobuf").display()
    );

    let mut config = prost_build::Config::new();
    config.type_attribute(
        ".",
        "#[derive(::prost_canonical_serde::CanonicalSerialize, ::prost_canonical_serde::CanonicalDeserialize)]",
    );
    config.btree_map(["."]);

    let fds = config.load_fds(&[proto2, proto3], &[proto_root])?;
    prost_canonical_serde_build::add_json_name_attributes(&mut config, &fds);
    config.compile_fds(fds)?;

    let conformance_proto = PathBuf::from("../protobuf/conformance/conformance.proto");
    assert!(
        conformance_proto.exists(),
        "conformance proto not found; ensure protobuf submodule is present"
    );

    println!("cargo:rerun-if-changed={}", conformance_proto.display());

    let mut conformance_config = prost_build::Config::new();
    conformance_config.compile_protos(&[conformance_proto], &[PathBuf::from("../protobuf")])?;

    Ok(())
}
