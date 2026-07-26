//! RabbitMQ infrastructure event publisher implementation.
//!
//! Provides the concrete AMQP transport delivery mechanics for broadcasting
//! domain events over an active network fabric using an underlying `lapin` channel.

use crate::events::{EventPublisher, SessionRevokedEvent};
use async_trait::async_trait;
use lapin::{options::*, BasicProperties, Channel, Connection, ConnectionProperties, ExchangeKind};
use tracing::info;

/// An AMQP-backed messaging coordinator managing domain event delivery.
pub struct AmqpEventPublisher {
    /// Active network channel channel used to dispatch messages to the message broker.
    channel: Channel,
}

impl AmqpEventPublisher {
    /// Establishes a connection to the AMQP broker and declares the underlying exchange topography.
    ///
    /// # Errors
    /// Returns a [`lapin::Error`] if network handshakes or resource declarations fail.
    pub async fn new(url: &str) -> Result<Self, lapin::Error> {
        let conn = Connection::connect(url, ConnectionProperties::default()).await?;
        let channel = conn.create_channel().await?;

        // Declare a durable fanout exchange so all interconnected services can bind individual
        // queues and natively receive decoupled duplicates of safety broadcasts.
        channel
            .exchange_declare(
                "security.events".into(),
                ExchangeKind::Fanout,
                ExchangeDeclareOptions {
                    durable: true,
                    ..ExchangeDeclareOptions::default()
                },
                lapin::types::FieldTable::default(),
            )
            .await?;

        info!("Infrastructure: Connected to RabbitMQ successfully. Security Broadcast Exchange Ready.");
        Ok(Self { channel })
    }
}

#[async_trait]
impl EventPublisher for AmqpEventPublisher {
    /// Serializes and dispatches a session revocation event across the security exchange fabric.
    ///
    /// # Errors
    /// Returns an error if structural serialization fails or message transmission drops.
    async fn publish_session_revoked(
        &self,
        event: &SessionRevokedEvent,
    ) -> Result<(), anyhow::Error> {
        let payload = serde_json::to_vec(event)?;

        self.channel
            .basic_publish(
                "security.events".into(),
                "".into(), // A fanout exchange configuration broadcasts across all bound queues, bypassing routing keys.
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await?;

        Ok(())
    }
}
