//! Shared API contract definitions for the YourApp ecosystem.
//!
//! Consolidates all auto-generated Protobuf modules and gRPC client/server
//! stubs into a unified assembly to eliminate cross-domain circular dependencies.

pub mod auth {
    pub mod v1 {
        tonic::include_proto!("auth.v1");
    }
}

pub mod identity {
    pub mod v1 {
        tonic::include_proto!("identity.v1");
    }
}

pub mod human_verification {
    pub mod v1 {
        tonic::include_proto!("human_verification.v1");
    }
}

pub mod email {
    pub mod v1 {
        tonic::include_proto!("email.v1");
    }
}

pub mod access_tokens {
    pub mod v1 {
        tonic::include_proto!("access_tokens.v1");
    }
}
