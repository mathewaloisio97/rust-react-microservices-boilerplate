//! Authentication domain event definitions and broadcasting abstractions.
//!
//! Provides the core data structures and contract traits required to publish
//! state changes—such as session revocations—outward to downstream system services.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Broadcasted to downstream microservices when a user session is explicitly terminated.
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionRevokedEvent {
    /// The unique, SHA-256 hashed signature of the revoked session token.
    pub sid: String,
}

/// Defines the boundary contract for dispatching domain events across the message fabric.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publishes a session revocation event to notify downstream consumers.
    ///
    /// # Errors
    /// Returns an error if the underlying message broker handles or delivery mechanisms fail.
    async fn publish_session_revoked(
        &self,
        event: &SessionRevokedEvent,
    ) -> Result<(), anyhow::Error>;
}
