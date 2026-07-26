pub mod security {
    /// The fallback HMAC-SHA256 secret used for local development.
    /// MUST NOT be used in production environments.
    pub const DEFAULT_HV_SECRET: &str = "local_dev_hv_secret";
}
