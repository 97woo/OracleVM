fn main() -> Result<(), Box<dyn std::error::Error>> {
    // aggregator proto 파일 컴파일
    tonic_build::configure()
        .build_server(false)
        .compile(
            &["../proto/aggregator.proto"],
            &["../proto"],
        )?;
    Ok(())
}