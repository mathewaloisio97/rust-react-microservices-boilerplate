//! Background worker for consuming and processing email dispatch events.
//!
//! Spawns a background task that listens to an AMQP queue, parses incoming
//! dispatch events, and transmits outbound emails via SMTP.

use crate::events::EmailDispatchEvent;
use futures_util::stream::StreamExt;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use tracing::{error, info, warn};

/// Starts an asynchronous loop that consumes from the email dispatch queue and sends SMTP messages.
///
/// Automatically reconnects to AMQP if the network connection or consumer stream drops.
pub async fn start_email_worker(amqp_url: String, smtp_host: String, smtp_port: u16) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

            let conn = match Connection::connect(&amqp_url, ConnectionProperties::default()).await {
                Ok(c) => c,
                Err(_) => continue,
            };
            let channel = match conn.create_channel().await {
                Ok(c) => c,
                Err(_) => continue,
            };

            let mut consumer = match channel
                .basic_consume(
                    "email_dispatch".into(),
                    "email_sender_worker".into(),
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await
            {
                Ok(c) => c,
                Err(_) => continue,
            };

            let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(smtp_host.clone())
                .port(smtp_port)
                .build();

            info!("Email worker listening for events.");

            while let Some(delivery_opt) = consumer.next().await {
                if let Ok(delivery) = delivery_opt {
                    if let Ok(event) = serde_json::from_slice::<EmailDispatchEvent>(&delivery.data)
                    {
                        info!("Processing dispatch to {}", event.target_email);

                        let from_addr = "YourApp <noreply@your_app.online>"
                            .parse::<lettre::message::Mailbox>()
                            .unwrap();
                        match event.target_email.parse::<lettre::message::Mailbox>() {
                            Ok(to_addr) => {
                                let email_res = Message::builder()
                                    .from(from_addr)
                                    .to(to_addr)
                                    .subject("YourApp - Your Verification Code")
                                    .body(format!(
                                        "Your verification code is: {}\n\nType: {}",
                                        event.verification_code, event.verification_type
                                    ));

                                if let Ok(email) = email_res {
                                    match mailer.send(email).await {
                                        Ok(_) => {
                                            delivery.ack(BasicAckOptions::default()).await.ok();
                                        }
                                        Err(e) => {
                                            error!("SMTP failure: {:?}. Re-queuing message.", e);
                                            delivery
                                                .nack(BasicNackOptions {
                                                    requeue: true,
                                                    multiple: false,
                                                })
                                                .await
                                                .ok();
                                            tokio::time::sleep(tokio::time::Duration::from_secs(4))
                                                .await;
                                        }
                                    }
                                } else {
                                    // Reject malformed email payloads without re-queuing.
                                    delivery
                                        .reject(BasicRejectOptions { requeue: false })
                                        .await
                                        .ok();
                                }
                            }
                            Err(_) => {
                                delivery
                                    .reject(BasicRejectOptions { requeue: false })
                                    .await
                                    .ok();
                            }
                        }
                    } else {
                        // Reject unparseable event payloads without re-queuing.
                        delivery
                            .reject(BasicRejectOptions { requeue: false })
                            .await
                            .ok();
                    }
                }
            }
            warn!("AMQP consumer stream closed. Reconnecting...");
        }
    });
}
