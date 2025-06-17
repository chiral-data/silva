fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .out_dir(std::env::var("OUT_DIR")?)
        .compile(
            &["proto/chiral.proto"],
            &["proto"],
        )?;
    Ok(())
}
