use std::env;
use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let original_out_dir = PathBuf::from(env::var("OUT_DIR")?);
    tonic_build::configure()
        .build_server(false)
        .file_descriptor_set_path(original_out_dir.join("rls.bin"))
        .compile(
            &["envoy/service/ratelimit/v3/rls.proto"],
            &[
                "vendor/protobufs/data-plane-api",
                "vendor/protobufs/protoc-gen-validate",
                "vendor/protobufs/xds",
            ],
        )?;
    Ok(())
}
