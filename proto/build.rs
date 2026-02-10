fn main() -> Result<(), Box<dyn std::error::Error>> {
    // proto 包的根目录就是 proto 文件所在的目录
    // 为每个 proto 文件生成独立的文件，避免类型冲突
    
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    
    // 生成到 src/gen 子目录
    let out_dir = manifest_dir.join("src").join("gen");
    std::fs::create_dir_all(&out_dir)?;
    
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
    
    // 不使用 extern_path，让生成的代码使用相对路径
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .type_attribute("loquat.kernel.", "#[allow(clippy::all)]")
        .out_dir(&kernel_out)
        .compile(&["kernel.proto"], &["."])?;
    
    // 生成到 src/gen/engine 子目录
    let engine_out = out_dir.join("engine");
    std::fs::create_dir_all(&engine_out)?;
    
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .type_attribute("loquat.engine.", "#[allow(clippy::all)]")
        .out_dir(&engine_out)
        .compile(&["engine.proto"], &["."])?;
    
    Ok(())
}
