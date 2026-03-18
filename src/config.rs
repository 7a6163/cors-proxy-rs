use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "cors-proxy-rs", about = "A CORS Anywhere proxy server")]
pub struct Config {
    /// Port to listen on
    #[arg(short, long, env = "PORT", default_value_t = 3000)]
    pub port: u16,

    /// Maximum requests per minute per IP
    #[arg(long, env = "RATE_LIMIT", default_value_t = 60)]
    pub rate_limit_per_minute: u32,

    /// Allowed origins (comma-separated). If empty, all origins are allowed.
    #[arg(long, env = "ALLOWED_ORIGINS", value_delimiter = ',')]
    pub allowed_origins: Vec<String>,

    /// Maximum request body size in bytes (default: 10MB)
    #[arg(long, env = "MAX_BODY_SIZE", default_value_t = 10 * 1024 * 1024)]
    pub max_body_size: usize,

    /// Block requests to private/loopback IPs
    #[arg(long, env = "BLOCK_PRIVATE_IPS", default_value_t = true)]
    pub block_private_ips: bool,

    /// Request timeout in seconds
    #[arg(long, env = "TIMEOUT_SECS", default_value_t = 30)]
    pub timeout_secs: u64,
}
