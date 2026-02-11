fn main() -> Result<(), Box<dyn std::error::Error>> {
    // proto 包的根目录就是 proto 文件所在的目录
    // 为每个 proto 文件生成独立的文件，避免类型冲突
    
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    
    // 生成到 src/gen 子目录
    let out_dir = manifest_dir.join("src").join("gen");
    std::fs::create_dir_all(&out_dir)?;
    
    // 生成到 src/gen/v1 子目录（package.proto）
    let v1_out = out_dir.join("v1");
    std::fs::create_dir_all(&v1_out)?;
    
    tonic_build::configure()
        .build_server(false)  // package 只有 message，没有 service
        .build_client(false)
        .type_attribute("loquat.v1.", "#[derive(Serialize, Deserialize)]")
        .type_attribute("loquat.v1.", "#[serde(rename_all = \"snake_case\")]")
        .out_dir(&v1_out)
        .compile(&["package.proto"], &["."])?;
    
    // 生成到 src/gen/common 子目录
    let common_out = out_dir.join("common");
    std::fs::create_dir_all(&common_out)?;
    
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&common_out)
        .compile(&["common.proto"], &["."])?;
    
    // 生成到 src/gen/kernel 子目录
    let kernel_out = out_dir.join("kernel");
    std::fs::create_dir_all(&kernel_out)?;
    
    // kernel 依赖 package.proto
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .type_attribute("loquat.kernel.", "#[allow(clippy::all)]")
        .out_dir(&kernel_out)
        .compile(&["kernel.proto"], &["."])?;
    
    // 生成到 src/gen/engine 子目录
    let engine_out = out_dir.join("engine");
    std::fs::create_dir_all(&engine_out)?;
    
    // engine 依赖 package.proto
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .type_attribute("loquat.engine.", "#[allow(clippy::all)]")
        .out_dir(&engine_out)
        .compile(&["engine.proto"], &["."])?;
    
    Ok(())
}
