//! Build-time helpers for configuring `prost_build`.
//!
//! These helpers attach `proto_name` and `json_name` attributes so the derive
//! macros can serialize both forms correctly.
//!
//! # Example
//! ```rust,ignore
//! use std::path::PathBuf;
//!
//! let mut config = prost_build::Config::new();
//! config.type_attribute(
//!     "demo.Example",
//!     "#[derive(prost_canonical_serde::CanonicalSerialize, prost_canonical_serde::CanonicalDeserialize)]",
//! );
//! let includes = [PathBuf::from("proto")];
//! let fds = config.load_fds(&[PathBuf::from("proto/example.proto")], &includes)?;
//! add_json_name_attributes(&mut config, &fds);
//! config.compile_fds(fds)?;
//! ```
use prost_types::{DescriptorProto, FileDescriptorSet};

/// Adds `prost_canonical_serde` field attributes with proto/json names.
pub fn add_json_name_attributes(config: &mut prost_build::Config, fds: &FileDescriptorSet) {
    for file in &fds.file {
        let package = file.package.as_deref().unwrap_or("");
        for message in &file.message_type {
            if let Some(name) = message.name.as_deref() {
                let fq_name = if package.is_empty() {
                    name.to_string()
                } else {
                    format!("{package}.{name}")
                };
                add_message_field_attributes(config, &fq_name, message);
            }
        }
    }
}

fn add_message_field_attributes(
    config: &mut prost_build::Config,
    fq_message_name: &str,
    message: &DescriptorProto,
) {
    for field in &message.field {
        let Some(proto_name) = field.name.as_deref() else {
            continue;
        };
        let json_name = field.json_name.as_deref().unwrap_or(proto_name);
        let proto_lit = format!("{proto_name:?}");
        let json_lit = format!("{json_name:?}");
        let attr =
            format!("#[prost_canonical_serde(proto_name = {proto_lit}, json_name = {json_lit})]");
        let field_path = format!("{fq_message_name}.{proto_name}");
        config.field_attribute(field_path, attr.clone());

        if let Some(oneof_index) = field.oneof_index {
            let Ok(oneof_index) = usize::try_from(oneof_index) else {
                continue;
            };
            let Some(oneof) = message.oneof_decl.get(oneof_index) else {
                continue;
            };
            let Some(oneof_name) = oneof.name.as_deref() else {
                continue;
            };
            let oneof_fq = format!("{fq_message_name}.{oneof_name}");
            let oneof_field_path = format!("{oneof_fq}.{proto_name}");
            config.field_attribute(oneof_field_path, attr);
        }
    }

    for nested in &message.nested_type {
        if let Some(name) = nested.name.as_deref() {
            let nested_fq = format!("{fq_message_name}.{name}");
            add_message_field_attributes(config, &nested_fq, nested);
        }
    }
}
