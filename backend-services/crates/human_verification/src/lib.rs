//! Human verification and CAPTCHA coordination subsystem.
//!
//! Validates client-side tokens against external providers (e.g., Google reCAPTCHA, Cloudflare Turnstile)
//! and issues stateless cryptographic vouchers upon successful verification.

pub mod config;
pub mod grpc;
pub mod providers;
