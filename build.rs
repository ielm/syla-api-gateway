use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Print build info for debugging
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=proto/syla.proto");
    
    // Get OUT_DIR from cargo
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    
    // Create a subdirectory in OUT_DIR for our generated files
    let proto_out = out_dir.join("proto");
    std::fs::create_dir_all(&proto_out)?;
    
    // Configure tonic-build with proper settings
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(&proto_out)
        .file_descriptor_set_path(out_dir.join("descriptor.bin"))
        .protoc_arg("--experimental_allow_proto3_optional")
        // Add serde derives to types that don't contain protobuf types
        .type_attribute("syla.v1.ExecutionStatus", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute("syla.v1.Language", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute("syla.v1.WorkspaceType", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute("syla.v1.WorkspaceStatus", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute("syla.v1.HealthCheckResponse.HealthStatus", "#[derive(serde::Serialize, serde::Deserialize)]")
        // Compile the proto files
        .compile_protos(
            &["proto/syla.proto"],
            &["proto"],  // Now includes google and common via symlinks
        )?;
    
    // Generate a mod.rs file in OUT_DIR that properly includes the generated code
    let mod_file_content = r#"
// Generated module for Syla proto
pub mod syla {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/proto/syla.v1.rs"));
    }
}

// Re-export for convenience
pub use syla::v1::*;
pub use syla::v1::syla_gateway_server::{SylaGateway, SylaGatewayServer};
"#;
    
    std::fs::write(out_dir.join("proto_mod.rs"), mod_file_content)?;
    
    println!("cargo:warning=Proto compilation completed successfully");
    
    Ok(())
}