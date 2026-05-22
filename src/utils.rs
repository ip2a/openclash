use anyhow::{Context, Result};
use colored::Colorize;
use std::env;
use std::fs;
use std::net::{TcpListener, UdpSocket};
use std::path::Path;
use std::process::Command;

/// Print success message
pub fn okcat(msg: &str) {
    println!("{} {}", "✅".green(), msg.bright_blue());
}

/// Print success message with custom emoji
pub fn okcat_with_emoji(emoji: &str, msg: &str) {
    println!("{} {}", emoji.green(), msg.bright_blue());
}

/// Print failure message to stderr
pub fn failcat(msg: &str) {
    eprintln!("{} {}", "❌".red(), msg.bright_magenta());
}

/// Print failure message with custom emoji
pub fn failcat_with_emoji(emoji: &str, msg: &str) {
    eprintln!("{} {}", emoji.red(), msg.bright_magenta());
}

/// Print error and quit
pub fn error_quit(msg: &str) -> ! {
    eprintln!("{} {}", "💥".red(), msg.bright_red());
    std::process::exit(1);
}

/// Check if running as root
pub fn is_root() -> bool {
    env::var("USER").map(|u| u == "root").unwrap_or(false)
}

/// Check if a port is already bound
pub fn is_bind(port: u16) -> Result<bool> {
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(listener) => {
            drop(listener);
            Ok(false)
        }
        Err(_) => Ok(true),
    }
}

/// Check if port is in use by a process other than the specified one
pub fn is_already_in_use(port: u16, process: &str) -> Result<bool> {
    if !is_bind(port)? {
        return Ok(false);
    }

    let process = process.to_ascii_lowercase();
    for line in port_listing_lines()? {
        if !line.contains(&format!(":{}", port)) {
            continue;
        }

        if let Some(owner) = owning_process_name(&line)? {
            if !owner.to_ascii_lowercase().contains(&process) {
                return Ok(true);
            }
        } else {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Get a random available port
pub fn get_random_port() -> Result<u16> {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

/// Read file to string, return empty if not exists
pub fn read_file_or_empty(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

/// Ensure directory exists
pub fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory: {}", path.display()))?;
    }
    Ok(())
}

/// Get user's home directory
pub fn home_dir() -> Result<std::path::PathBuf> {
    dirs::home_dir().context("Failed to get home directory")
}

/// Get the .openclash config directory
pub fn openclash_dir() -> Result<std::path::PathBuf> {
    let home = home_dir()?;
    let dir = home.join(".openclash");
    ensure_dir(&dir)?;
    Ok(dir)
}

/// Check if a command exists
#[allow(dead_code)]
pub fn command_exists(cmd: &str) -> bool {
    which::which(cmd).is_ok()
}

/// Get local IP address
pub fn get_local_ip() -> Result<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").context("Failed to create UDP socket")?;
    socket
        .connect("8.8.8.8:80")
        .context("Failed to detect local IP")?;
    Ok(socket.local_addr()?.ip().to_string())
}

/// Get public IP address (may fail behind proxy)
pub fn get_public_ip() -> Result<String> {
    let output = Command::new("curl")
        .args([
            "-s",
            "--noproxy",
            "*",
            "--connect-timeout",
            "2",
            "api64.ipify.org",
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let ip = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !ip.is_empty() {
                return Ok(ip);
            }
        }
        _ => {}
    }

    Ok(String::from("公网"))
}

fn port_listing_lines() -> Result<Vec<String>> {
    if cfg!(windows) {
        let output = Command::new("netstat")
            .args(["-ano"])
            .output()
            .context("Failed to run netstat")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Ok(stdout.lines().map(|line| line.to_string()).collect());
    }

    let output = if which::which("ss").is_ok() {
        Command::new("ss")
            .args(["-lnptu"])
            .output()
            .context("Failed to run ss")?
    } else {
        Command::new("netstat")
            .args(["-lnptu"])
            .output()
            .context("Failed to run netstat")?
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(|line| line.to_string()).collect())
}

fn owning_process_name(line: &str) -> Result<Option<String>> {
    if cfg!(windows) {
        let pid = line
            .split_whitespace()
            .last()
            .filter(|value| value.chars().all(|ch| ch.is_ascii_digit()));
        let Some(pid) = pid else {
            return Ok(None);
        };
        let filter = format!("PID eq {}", pid);
        let output = Command::new("tasklist")
            .args(["/FI", &filter, "/FO", "CSV", "/NH"])
            .output()
            .context("Failed to query tasklist")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let name = stdout
            .lines()
            .next()
            .and_then(|row| row.trim().strip_prefix('"'))
            .and_then(|row| row.split("\",").next())
            .map(|value| value.to_string());
        return Ok(name);
    }

    if let Some(start) = line.find("users:((\"") {
        let rest = &line[start + 10..];
        if let Some(end) = rest.find('\"') {
            return Ok(Some(rest[..end].to_string()));
        }
    }

    Ok(None)
}
