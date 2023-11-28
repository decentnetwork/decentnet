// use ffi_gen::FfiGen;
// use std::{env::consts::OS, path::PathBuf};

fn main() {
    //     prost_build::compile_protos(&["protobuf/decentnet.proto"], &["protobuf"]).unwrap();
    //     if OS == "windows" {
    //         ffi_gen();
    //     }
}

// fn ffi_gen() {
//     let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
//     let path = dir.join("api.rsh");
//     let ffigen = FfiGen::new(path).unwrap();
//     let dart = dir.join("bindings").join("dart").join("bindings.dart");
//     ffigen
//         .generate_dart(dart, "decentnet", "decentnet")
//         .unwrap();
// }
