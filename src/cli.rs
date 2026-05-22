use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "openclash")]
#[command(about = "Linux Clash/Mihomo proxy management tool")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Launch terminal dashboard
    Tui,

    /// Start the Rust-hosted Web dashboard
    #[command(visible_alias = "ui")]
    Web {
        /// Host/interface to bind
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port to listen on
        #[arg(short, long, default_value = "3737")]
        port: u16,

        /// Don't auto-open the browser
        #[arg(long)]
        no_open: bool,
    },

    /// Start proxy service and enable system proxy
    On,

    /// Stop proxy service and disable system proxy
    Off,

    /// Restart proxy service
    Restart,

    /// Show service status
    Status,

    /// System proxy management
    #[command(subcommand)]
    Proxy(ProxyCommands),

    /// Tun mode management
    #[command(subcommand)]
    Tun(TunCommands),

    /// Mixin configuration management
    #[command(subcommand)]
    Mixin(MixinCommands),

    /// Set or view Web UI secret
    Secret {
        /// New secret value
        secret: Option<String>,
    },

    /// Subscription update management
    #[command(subcommand)]
    Update(UpdateCommands),

    /// Switch language (zh/en)
    Lang {
        /// Language code: zh or en
        language: Option<String>,
    },

    /// Test proxy connectivity through Google/YouTube/GitHub
    Test {
        /// Test URL (default: http://www.google.com/generate_204)
        #[arg(short, long)]
        url: Option<String>,

        /// Use specific proxy port (default: auto-detect from runtime config)
        #[arg(short, long)]
        port: Option<u16>,
    },
}

#[derive(Subcommand)]
pub enum ProxyCommands {
    /// Enable system proxy
    On,
    /// Disable system proxy
    Off,
    /// Show system proxy status
    Status,
}

#[derive(Subcommand)]
pub enum TunCommands {
    /// Enable Tun mode
    On,
    /// Disable Tun mode
    Off,
    /// Show Tun status
    Status,
}

#[derive(Subcommand)]
pub enum MixinCommands {
    /// Edit mixin config with default editor
    Edit,
    /// View runtime merged config
    Runtime,
    /// View mixin config (default)
    View,
}

#[derive(Subcommand)]
pub enum UpdateCommands {
    /// Update subscription from URL
    Sync {
        /// Subscription URL
        url: Option<String>,
    },
    /// Setup auto update via cron
    Auto,
    /// View update log
    Log,
}
