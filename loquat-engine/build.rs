// Proto编译暂时禁用，需要安装protoc
// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     tonic_build::configure()
//         .build_server(true)
//         .build_client(true)
//         .compile_well_known_types(true)
//         .compile(
//             &[
//                 "../proto/common.proto",
//                 "../proto/kernel.proto",
//                 "../proto/engine.proto",
//             ],
//             &["../proto/"],
//         )?;
//     Ok(())
// }

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
}
