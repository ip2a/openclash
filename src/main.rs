mod cli;
mod config;
mod controller;
mod dashboard;
mod i18n;
mod mixin;
mod proxy;
mod resources;
mod service;
mod subscription;
mod test_proxy;
mod tui;
mod tun;
mod utils;
mod web;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, MixinCommands, ProxyCommands, TunCommands, UpdateCommands};
use colored::Colorize;
use config::UserConfig;
use i18n::{msg, set_language};
use utils::{failcat, okcat};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("{} {}", "❌".red(), e);
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Tui => {
            tui::run_tui()?;
        }

        Commands::Web {
            host,
            port,
            no_open,
        } => {
            web::run(host, port, no_open).await?;
        }

        Commands::On => {
            let config = UserConfig::load()?;
            service::ensure_ready(&config)?;
            service::start(&config)?;
            proxy::set_system_proxy(&config)?;
            okcat(&msg("proxy_on"));
        }

        Commands::Off => {
            let config = UserConfig::load()?;
            service::stop(&config)?;
            proxy::unset_system_proxy(&config)?;
            okcat(&msg("proxy_off"));
        }

        Commands::Restart => {
            let config = UserConfig::load()?;
            service::ensure_ready(&config)?;
            service::stop(&config)?;
            proxy::unset_system_proxy(&config)?;
            service::start(&config)?;
            proxy::set_system_proxy(&config)?;
        }

        Commands::Status => {
            let config = UserConfig::load()?;
            check_ready(&config)?;
            service::show_status(&config, &[])?;
        }

        Commands::Proxy(cmd) => {
            let config = UserConfig::load()?;
            check_ready(&config)?;
            match cmd {
                ProxyCommands::On => proxy::set_system_proxy(&config)?,
                ProxyCommands::Off => proxy::unset_system_proxy(&config)?,
                ProxyCommands::Status => proxy::show_status(&config)?,
            }
        }

        Commands::Tun(cmd) => {
            let config = UserConfig::load()?;
            check_ready(&config)?;
            match cmd {
                TunCommands::On => tun::tun_on(&config)?,
                TunCommands::Off => tun::tun_off(&config)?,
                TunCommands::Status => tun::tun_status(&config)?,
            }
        }

        Commands::Mixin(cmd) => {
            let config = UserConfig::load()?;
            check_ready(&config)?;
            match cmd {
                MixinCommands::Edit => mixin::edit_mixin(&config)?,
                MixinCommands::Runtime => mixin::view_runtime(&config)?,
                MixinCommands::View => mixin::view_mixin(&config)?,
            }
        }

        Commands::Secret { secret } => {
            let mut config = UserConfig::load()?;
            check_ready(&config)?;
            mixin::set_secret(&mut config, secret)?;
        }

        Commands::Update(cmd) => {
            let mut config = UserConfig::load()?;
            check_ready(&config)?;
            match cmd {
                UpdateCommands::Sync { url } => subscription::update_sync(&mut config, url)?,
                UpdateCommands::Auto => subscription::setup_auto_update(&config)?,
                UpdateCommands::Log => subscription::view_log(&config)?,
            }
        }

        Commands::Lang { language } => match language.as_deref() {
            Some("zh" | "en") => {
                set_language(language.as_deref().unwrap())?;
                okcat(&msg("lang_switched"));
            }
            None => {
                okcat(&msg("current_lang"));
            }
            _ => {
                failcat(&msg("lang_usage"));
            }
        },

        Commands::Test { url, port } => {
            let config = UserConfig::load()?;
            test_proxy::test_connectivity(&config, url, port)?;
        }
    }

    Ok(())
}

fn check_ready(config: &UserConfig) -> Result<()> {
    service::ensure_ready(config)
}
