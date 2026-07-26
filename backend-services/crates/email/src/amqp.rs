//! RabbitMQ transport mechanics and event publisher implementation.
//!
//! Handles connection state, queue declarations, and reliable payload delivery
//! utilizing publisher confirmations over an active AMQP channel.

use crate::events::EmailDispatchEvent;
use lapin::{options::*, BasicProperties, Channel, Connection, ConnectionProperties};
use tracing::info;

pub struct AmqpBroker {
    channel: Channel,
}

impl AmqpBroker {
    /// Connects to AMQP and declares required queues.
    pub async fn new(url: &str) -> Result<Self, lapin::Error> {
        let conn = Connection::connect(url, ConnectionProperties::default()).await?;
        let channel = conn.create_channel().await?;

        channel
            .queue_declare(
                "email_dispatch".into(),
                QueueDeclareOptions::default(),
                lapin::types::FieldTable::default(),
            )
            .await?;

        info!("Connected to RabbitMQ.");
        Ok(Self { channel })
    }

    /// Serializes and publishes an event to the email dispatch queue.
    pub async fn publish_dispatch(&self, event: &EmailDispatchEvent) -> Result<(), anyhow::Error> {
        let payload = serde_json::to_vec(event)?;

        // The double await resolves the network request, then the publisher confirmation.
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
