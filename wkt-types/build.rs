use std::env;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

use prost::Message;
use prost_types::FileDescriptorSet;

fn main() {
    #[cfg(feature = "vendored-protoc")]
    std::env::set_var("PROTOC", protobuf_src::protoc());

    let dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    build(&dir, "pbtime", true);
    build(&dir, "pbstruct", false);
    build(&dir, "pbany", false);
    build(&dir, "pbempty", false);
    build(&dir, "pbmask", false);
}

fn build(dir: &Path, proto: &str, include_uniffi: bool) {
    let out = dir.join(proto);
    let target = env::var("TARGET").unwrap();
    create_dir_all(&out).unwrap();
    let source = format!("proto/{proto}.proto");
    let descriptor_file = out.join("descriptors.bin");
    let mut prost_build = prost_build::Config::new();

    #[cfg(feature = "vendored-protox")]
    {
        let file_descriptors = protox::compile([source.clone()], ["proto/".to_string()]).unwrap();
        std::fs::write(&descriptor_file, file_descriptors.encode_to_vec()).unwrap();
        prost_build.skip_protoc_run();
    }

    let has_uniffi = cfg!(feature = "uniffi");
    let has_wasm = cfg!(feature = "wasm-bindgen");

    let type_attribute = match (include_uniffi, &target == "wasm32-unknown-unknown") {
        (true, true) => "#[wasm_bindgen::prelude::wasm_bindgen]",
        (true, false) => "#[derive(uniffi::Record)]",
        _ => "",
    };

    prost_build
        .compile_well_known_types()
        .type_attribute(
            "google.protobuf.Empty",
            "#[derive(serde_derive::Serialize, serde_derive::Deserialize)]",
        )
        .type_attribute(
            "google.protobuf.FieldMask",
            "#[derive(serde_derive::Serialize, serde_derive::Deserialize)]",
        )
        // .type_attribute(
        //     ".",
        //     if has_uniffi && include_uniffi {
        //         "#[derive(uniffi::Record)]"
        //     } else {
        //         ""
        //     },
        // )
        .type_attribute(
            "google.protobuf.Timestamp",
            // type_derives(&target, has_wasm && include_uniffi),
            type_attribute,
        )
        .file_descriptor_set_path(&descriptor_file)
        .out_dir(&out)
        .compile_protos(&[source], &["proto/".to_string()])
        .unwrap();

    let descriptor_bytes = std::fs::read(descriptor_file).unwrap();
    let descriptor = FileDescriptorSet::decode(&descriptor_bytes[..]).unwrap();

    prost_wkt_build::add_serde(out, descriptor);
}

fn type_derives(target: &str, should_include: bool) -> &'static str {
    if !should_include {
        return "";
    }
    match target {
        "wasm32-unknown-unknown" => {
            r#"
#[wasm_bindgen::prelude::wasm_bindgen]
"#
        }
        _ => "#[derive(uniffi::Record)]",
    }
}
