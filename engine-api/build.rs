fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .extern_path(".common.v1.Uuid", "::prost_uuid_doubleint::ProstUuid")
        .extern_path(".google.protobuf.Struct", "::prost_wkt_types::Struct")
        .compile_protos(
            &[
                "proto/common/v1/uuid.proto",
                "proto/workflow/v1/config.proto",
                "proto/workflow/v1/trigger.proto",
            ],
            &["proto"],
        )?;

    Ok(())
}
