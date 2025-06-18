use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let proto_path = "proto/chiral.proto";

    if !Path::new(proto_path).exists() {
        panic!("Proto file '{}' does not exist!", proto_path);
    }

    tonic_build::compile_protos(proto_path)?;
    Ok(())
}