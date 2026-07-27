//! RabbitMQ connection management and event publishing.

use crate::events::EmailDispatchEvent;
use lapin::{options::*, BasicProperties, Channel, Connection, ConnectionProperties};
use tracing::info;

/// Manages an active AMQP channel for publishing system events.
pub struct AmqpBroker {
    channel: Channel,
}

impl AmqpBroker {
    /// Connects to the RabbitMQ broker and ensures required queues exist.
    ///
    /// # Errors
    /// Returns a [`lapin::Error`] if the connection fails or queue declaration rejects.
    pub async fn new(url: &str) -> Result<Self, lapin::Error> {
        let conn = Connection::connect(url, ConnectionProperties::default()).await?;
        let channel = conn.create_channel().await?;

        // Declare durable queue to persist messages across broker restarts
        channel
            .queue_declare(
                "email_dispatch".into(),
                QueueDeclareOptions {
                    durable: true,
                    ..QueueDeclareOptions::default()
                },
                lapin::types::FieldTable::default(),
            )
            .await?;

        info!("Connected to RabbitMQ.");
        Ok(Self { channel })
    }

    /// Serializes and publishes an email dispatch event.
    ///
    /// # Errors
    /// Returns an error if serialization fails or the broker rejects the confirmation.
    pub async fn publish_dispatch(&self, event: &EmailDispatchEvent) -> Result<(), anyhow::Error> {
        let payload = serde_json::to_vec(event)?;

        // First await sends the payload; second await waits for broker confirmation
        self.channel
            .basic_publish(
                "".into(),
                "email_dispatch".into(),
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await?
            .await?;

        Ok(())
    }
}
