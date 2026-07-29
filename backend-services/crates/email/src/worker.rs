//! Background worker for consuming and processing email dispatch events.
//!
//! Spawns a background task that listens to an AMQP queue, parses incoming
//! dispatch events, and transmits outbound emails via SMTP using Lettre.

use crate::events::EmailDispatchEvent;
use futures_util::stream::StreamExt;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use tracing::{error, info, warn};

pub async fn start_email_worker(
    amqp_url: String,
    smtp_host: String,
    smtp_port: u16,
    smtp_username: Option<String>,
    smtp_password: Option<String>,
) {
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

            let mut builder =
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(smtp_host.clone())
                    .port(smtp_port);

            if let (Some(user), Some(pass)) = (&smtp_username, &smtp_password) {
                let creds = Credentials::new(user.clone(), pass.clone());
                builder = builder.credentials(creds);
            }

            let mailer = builder.build();

            info!("Email worker actively listening for AMQP dispatch events.");

            while let Some(delivery_opt) = consumer.next().await {
                if let Ok(delivery) = delivery_opt {
                    if let Ok(event) = serde_json::from_slice::<EmailDispatchEvent>(&delivery.data)
                    {
                        info!("Processing email dispatch to {}", event.target_email);

                        let from_addr = "YourApp <noreply@your_app.online>"
                            .parse::<lettre::message::Mailbox>()
                            .unwrap();

                        match event.target_email.parse::<lettre::message::Mailbox>() {
                            Ok(to_addr) => {
                                let email_res = Message::builder()
                                    .from(from_addr)
                                    .to(to_addr)
                                    .subject("YourApp - Complete Your Verification")
                                    .body(format!(
                                        "Click the secure link below to verify your email address:\nhttp://localhost:5173/verify?email={}&user_id={}&code={}\n\nType: {}\nUser ID: {}",
                                        urlencoding::encode(&event.target_email),
                                        urlencoding::encode(&event.user_id),
                                        urlencoding::encode(&event.verification_code),
                                        event.verification_type,
                                        event.user_id
                                    ));

                                if let Ok(email) = email_res {
                                    match mailer.send(email).await {
                                        Ok(_) => {
                                            delivery.ack(BasicAckOptions::default()).await.ok();
                                        }
                                        Err(e) => {
                                            error!(
                                                "SMTP delivery failure: {:?}. Re-queuing message.",
                                                e
                                            );
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
                        delivery
                            .reject(BasicRejectOptions { requeue: false })
                            .await
                            .ok();
                    }
                }
            }
            warn!("AMQP consumer stream closed. Attempting reconnect...");
        }
    });
}
