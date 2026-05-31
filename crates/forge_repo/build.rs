fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("PROTOC_GEN_CONFIGURE_OPTIONS: {:?}", std::env::var("PROTOC_GEN_CONFIGURE_OPTIONS"));
    unsafe {
        std::env::set_var("PROTOC_GEN_CONFIGURE_OPTIONS", "--experimental_allow_proto3_optional");
    }
    println!("PROTOC_GEN_CONFIGURE_OPTIONS after set: {:?}", std::env::var("PROTOC_GEN_CONFIGURE_OPTIONS"));
    tonic_prost_build::compile_protos("proto/forge.proto")?;
    Ok(())
}
