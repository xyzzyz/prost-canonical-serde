fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/example.proto");
    println!("cargo:rerun-if-changed=proto/kitchen_sink.proto");
    let mut config = prost_build::Config::new();
    config.type_attribute(
        ".",
        "#[derive(::prost_canonical_serde::CanonicalSerialize, ::prost_canonical_serde::CanonicalDeserialize)]",
    );

    let fds = config.load_fds(
        &["proto/example.proto", "proto/kitchen_sink.proto"],
        &["proto"],
    )?;
    prost_canonical_serde_build::add_json_name_attributes(&mut config, &fds);
    config.out_dir("src");
    config.compile_fds(fds)?;

    Ok(())
}
