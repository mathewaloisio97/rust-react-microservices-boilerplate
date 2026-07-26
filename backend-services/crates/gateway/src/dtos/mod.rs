//! Data Transfer Objects (DTOs) for the Edge Gateway.
//!
//! This module defines the public-facing JSON schemas and validation contracts
//! for incoming HTTP requests and outgoing HTTP responses. These types are
//! decoupled from internal gRPC schemas to allow safe API versioning.

pub mod access_tokens;
pub mod email;
pub mod human_verification;
pub mod identity;

pub use access_tokens::*;
pub use email::*;
pub use human_verification::*;
pub use identity::*;
