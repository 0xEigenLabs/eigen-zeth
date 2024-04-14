fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
            &["proto/prover/v1/prover.proto"],
            &["proto/prover/v1", "proto/include"],
        )?;
    Ok(())
}