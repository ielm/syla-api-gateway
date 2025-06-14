// Include the generated proto code for API Gateway
include!(concat!(env!("OUT_DIR"), "/proto_mod.rs"));

// Include proto types from other services
pub mod execution {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/syla.execution.v1.rs"));
    }
}

pub mod common {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/syla.common.v1.rs"));
    }
}