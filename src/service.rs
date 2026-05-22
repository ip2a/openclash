use crate::config::{merge_configs, UserConfig};
use crate::resources;
use crate::utils::{ensure_dir, failcat, okcat, okcat_with_emoji};
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::fs;
use std::io;
use std::process::{Command, Stdio};
use zip::ZipArchive;

/// Ensure all resources are extracted and runtime config is ready
pub fn ensure_ready(config: &UserConfig) -> Result<()> {
    ensure_resources(config)?;

    let raw_path = config.config_raw();
    if !raw_path.exists() {
        if resources::exists("config.yaml") {
            resources::extract("config.yaml", &raw_path)
                .context("Failed to extract default config.yaml")?;
        } else {
            let mixin_content = fs::read_to_string(&config.config_mixin())
                .unwrap_or_else(|_| include_str!("../resources/mixin.yaml").to_string());
            fs::write(&raw_path, mixin_content)?;
        }
    }

    let runtime_path = config.config_runtime();
    if !runtime_path.exists() {
        let mixin_content = fs::read_to_string(&config.config_mixin()).unwrap_or_default();
        let raw_content = fs::read_to_string(&raw_path).unwrap_or_default();
        let runtime = merge_configs(&[&mixin_content, &raw_content, &mixin_content])?;
        runtime.to_file(&runtime_path)?;
    }

    Ok(())
}

/// Check if kernel process is running by reading PID file and sending signal 0
pub fn is_running(config: &UserConfig) -> Result<bool> {
    let pid_file = config.pid_file();
    if !pid_file.exists() {
        return Ok(false);
    }

    let pid_str = fs::read_to_string(&pid_file).unwrap_or_default();
    let pid: i32 = match pid_str.trim().parse() {
        Ok(p) if p > 0 => p,
        _ => return Ok(false),
    };

    is_process_running(pid)
}

/// Start the kernel process
pub fn start(config: &UserConfig) -> Result<()> {
    if is_running(config)? {
        return Ok(());
    }
    fs::remove_file(config.pid_file()).ok();

    let runtime_path = config.config_runtime();
    if !runtime_path.exists() {
        anyhow::bail!("运行时配置不存在: {}", runtime_path.display());
    }

    ensure_dir(&config.run_dir())?;

    let log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(config.kernel_log())?;

    let mut cmd = Command::new(config.kernel_path());
    cmd.args([
        "-d",
        &config.clash_base_dir.to_string_lossy(),
        "-f",
        &runtime_path.to_string_lossy(),
    ])
    .stdout(Stdio::from(log_file.try_clone()?))
    .stderr(Stdio::from(log_file))
    .current_dir(&config.clash_base_dir);

    let child = cmd
        .spawn()
        .with_context(|| format!("Failed to start kernel: {}", config.kernel_path().display()))?;

    let pid = child.id() as i32;

    fs::write(config.pid_file(), pid.to_string())
        .with_context(|| format!("Failed to write PID file: {}", config.pid_file().display()))?;

    std::thread::sleep(std::time::Duration::from_millis(800));

    if !is_running(config)? {
        let log = fs::read_to_string(config.kernel_log()).unwrap_or_default();
        let recent: Vec<&str> = log.lines().rev().take(20).collect();
        anyhow::bail!(
            "内核启动失败\n{}",
            recent.iter().rev().cloned().collect::<Vec<_>>().join("\n")
        );
    }

    Ok(())
}

/// Stop the kernel process
pub fn stop(config: &UserConfig) -> Result<()> {
    if !is_running(config)? {
        fs::remove_file(config.pid_file()).ok();
        return Ok(());
    }

    let pid_str = fs::read_to_string(config.pid_file()).unwrap_or_default();
    let pid: i32 = pid_str
        .trim()
        .parse()
        .with_context(|| format!("Invalid PID in file: {}", config.pid_file().display()))?;

    stop_process(pid, false)?;

    let start = std::time::Instant::now();
    while is_running(config)? {
        std::thread::sleep(std::time::Duration::from_millis(200));
        if start.elapsed().as_secs() > 5 {
            stop_process(pid, true)?;
            std::thread::sleep(std::time::Duration::from_millis(500));
            break;
        }
    }

    fs::remove_file(config.pid_file()).ok();
    Ok(())
}

/// Restart the kernel process
pub fn restart(config: &UserConfig) -> Result<()> {
    stop(config)?;
    start(config)?;
    Ok(())
}

/// Show service status
pub fn show_status(config: &UserConfig, _extra_args: &[String]) -> Result<()> {
    if is_running(config)? {
        let pid = fs::read_to_string(config.pid_file()).unwrap_or_default();
        okcat(&format!("服务状态：运行中 (PID: {})", pid.trim()));

        if let Ok(port) = get_proxy_port(config) {
            okcat(&format!("代理端口: {}", port));
        }
        if let Ok(port) = get_ui_port(config) {
            okcat(&format!("控制端口: {}", port));
        }
    } else {
        failcat("服务状态：未运行");
    }

    let log_path = config.kernel_log();
    if log_path.exists() {
        let log = fs::read_to_string(&log_path).unwrap_or_default();
        let lines: Vec<&str> = log.lines().collect();
        let recent = lines.iter().rev().take(10).cloned().collect::<Vec<_>>();
        if !recent.is_empty() {
            println!("\n最近日志:");
            for line in recent.iter().rev() {
                println!("  {}", line);
            }
        }
    }

    Ok(())
}

pub fn read_recent_logs(config: &UserConfig, limit: usize) -> Vec<String> {
    let log_path = config.kernel_log();
    if !log_path.exists() {
        return Vec::new();
    }

    let log = fs::read_to_string(log_path).unwrap_or_default();
    let mut lines: Vec<String> = log.lines().map(|line| line.to_string()).collect();
    if lines.len() > limit {
        lines = lines.split_off(lines.len() - limit);
    }
    lines
}

/// Get the mixed proxy port from runtime config, with fallback and conflict handling
pub fn get_proxy_port(config: &UserConfig) -> Result<u16> {
    let runtime_config = crate::config::RuntimeConfig::from_file(&config.config_runtime())?;
    let mut port = runtime_config
        .get_u16("mixed-port")
        .unwrap_or(config.mixed_port);

    if crate::utils::is_already_in_use(port, &config.kernel_name)? {
        let new_port = crate::utils::get_random_port()?;
        crate::utils::failcat(&format!(
            "⚠️ 代理端口 {} 被占用，已更换为 {}",
            port, new_port
        ));

        let mut rt = runtime_config;
        rt.set("mixed-port", serde_yaml::Value::Number(new_port.into()));
        rt.to_file(&config.config_runtime())?;
        port = new_port;
    }

    Ok(port)
}

/// Get the UI port from runtime config, with fallback and conflict handling
pub fn get_ui_port(config: &UserConfig) -> Result<u16> {
    let runtime_config = crate::config::RuntimeConfig::from_file(&config.config_runtime())?;
    let ext = runtime_config
        .get_string("external-controller")
        .unwrap_or_default();
    let mut port = ext
        .split(':')
        .last()
        .and_then(|p| p.parse().ok())
        .unwrap_or(config.ui_port);

    if crate::utils::is_already_in_use(port, &config.kernel_name)? {
        let new_port = crate::utils::get_random_port()?;
        crate::utils::failcat(&format!(
            "⚠️ 控制端口 {} 被占用，已更换为 {}",
            port, new_port
        ));

        let mut rt = runtime_config;
        rt.set(
            "external-controller",
            serde_yaml::Value::String(format!("0.0.0.0:{}", new_port)),
        );
        rt.to_file(&config.config_runtime())?;
        port = new_port;
    }

    Ok(port)
}

/// Extract all embedded resources
fn ensure_resources(config: &UserConfig) -> Result<()> {
    let kernel_path = config.kernel_path();
    if !kernel_path.exists() || kernel_needs_refresh(&kernel_path)? {
        okcat_with_emoji("📦", "正在解压内核...");
        ensure_dir(&config.bin_dir())?;

        let resource_name = kernel_resource_name()?;
        let archive_name = resource_name
            .rsplit('/')
            .next()
            .context("Invalid kernel resource name")?;
        let archive_path = config.bin_dir().join(archive_name);
        resources::extract(resource_name, &archive_path)?;
        extract_kernel_archive(&archive_path, &kernel_path)?;
        fs::remove_file(&archive_path).ok();
    }

    let subconverter_bin = config.subconverter_path();
    if !subconverter_bin.exists() {
        if resources::exists("zip/subconverter_linux64.tar.gz") {
            let tar_path = config.bin_dir().join("subconverter.tar.gz");
            resources::extract("zip/subconverter_linux64.tar.gz", &tar_path)?;
            Command::new("tar")
                .args([
                    "-xf",
                    &tar_path.to_string_lossy(),
                    "-C",
                    &config.bin_dir().to_string_lossy(),
                ])
                .output()?;
            fs::remove_file(&tar_path).ok();
        }
    }

    let yq_bin = config.bin_dir().join("yq");
    if !yq_bin.exists() {
        if resources::exists("zip/yq_linux_amd64.tar.gz") {
            let tar_path = config.bin_dir().join("yq.tar.gz");
            resources::extract("zip/yq_linux_amd64.tar.gz", &tar_path)?;
            Command::new("tar")
                .args([
                    "-xf",
                    &tar_path.to_string_lossy(),
                    "-C",
                    &config.bin_dir().to_string_lossy(),
                ])
                .output()?;
            fs::remove_file(&tar_path).ok();

            let entries = fs::read_dir(&config.bin_dir())?;
            for entry in entries {
                let entry = entry?;
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("yq_") {
                    fs::rename(entry.path(), &yq_bin)?;
                    break;
                }
            }
        }
    }

    let public_dir = config.public_dir();
    if !public_dir.exists() || fs::read_dir(&public_dir)?.next().is_none() {
        ensure_dir(&public_dir)?;
        if resources::exists("zip/yacd.tar.xz") {
            let tar_path = public_dir.join("yacd.tar.xz");
            resources::extract("zip/yacd.tar.xz", &tar_path)?;
            Command::new("tar")
                .args([
                    "-xf",
                    &tar_path.to_string_lossy(),
                    "-C",
                    &public_dir.to_string_lossy(),
                ])
                .output()?;
            fs::remove_file(&tar_path).ok();
        }
    }

    let geoip_path = config.clash_base_dir.join("Country.mmdb");
    if !geoip_path.exists() {
        resources::extract("Country.mmdb", &geoip_path)?;
    }

    let mixin_path = config.config_mixin();
    if !mixin_path.exists() {
        resources::extract("mixin.yaml", &mixin_path)?;
    }

    Ok(())
}

fn kernel_needs_refresh(kernel_path: &std::path::Path) -> Result<bool> {
    if !kernel_path.exists() {
        return Ok(false);
    }

    let bytes = fs::read(kernel_path).with_context(|| {
        format!("Failed to inspect kernel binary: {}", kernel_path.display())
    })?;

    Ok(match std::env::consts::OS {
        "macos" => is_elf_binary(&bytes),
        "linux" => is_macho_binary(&bytes),
        "windows" => is_elf_binary(&bytes) || is_macho_binary(&bytes),
        _ => false,
    })
}

fn is_elf_binary(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x7F, b'E', b'L', b'F'])
}

fn is_macho_binary(bytes: &[u8]) -> bool {
    bytes.len() >= 4
        && matches!(
            [bytes[0], bytes[1], bytes[2], bytes[3]],
            [0xFE, 0xED, 0xFA, 0xCE]
                | [0xFE, 0xED, 0xFA, 0xCF]
                | [0xCE, 0xFA, 0xED, 0xFE]
                | [0xCF, 0xFA, 0xED, 0xFE]
                | [0xCA, 0xFE, 0xBA, 0xBE]
                | [0xBE, 0xBA, 0xFE, 0xCA]
                | [0xCA, 0xFE, 0xBA, 0xBF]
                | [0xBF, 0xBA, 0xFE, 0xCA]
        )
}

fn extract_kernel_archive(archive_path: &std::path::Path, kernel_path: &std::path::Path) -> Result<()> {
    let archive_name = archive_path.to_string_lossy();
    if archive_name.ends_with(".gz") {
        let mut decoder = GzDecoder::new(
            fs::File::open(archive_path)
                .with_context(|| format!("Failed to open kernel archive: {}", archive_path.display()))?,
        );
        let mut output = fs::File::create(kernel_path)
            .with_context(|| format!("Failed to create kernel: {}", kernel_path.display()))?;
        io::copy(&mut decoder, &mut output)
            .with_context(|| format!("Failed to extract kernel: {}", kernel_path.display()))?;
    } else if archive_name.ends_with(".zip") {
        let file = fs::File::open(archive_path)
            .with_context(|| format!("Failed to open kernel archive: {}", archive_path.display()))?;
        let mut archive = ZipArchive::new(file)
            .with_context(|| format!("Failed to read kernel zip: {}", archive_path.display()))?;
        if archive.len() == 0 {
            anyhow::bail!("Kernel zip is empty: {}", archive_path.display());
        }
        let mut entry = archive
            .by_index(0)
            .with_context(|| format!("Failed to read kernel zip entry: {}", archive_path.display()))?;
        let mut output = fs::File::create(kernel_path)
            .with_context(|| format!("Failed to create kernel: {}", kernel_path.display()))?;
        io::copy(&mut entry, &mut output)
            .with_context(|| format!("Failed to extract kernel: {}", kernel_path.display()))?;
    } else {
        anyhow::bail!("Unsupported kernel archive format: {}", archive_path.display());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(kernel_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(kernel_path, perms)?;
    }

    Ok(())
}

fn kernel_resource_name() -> Result<&'static str> {
    let candidates: &[&str] = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => &[
            "zip/mihomo-linux-amd64-compatible.gz",
            "zip/mihomo-linux-amd64-compatible-v1.19.2.gz",
            "zip/mihomo-linux-amd64.gz",
        ],
        ("linux", "aarch64") => &["zip/mihomo-linux-arm64.gz"],
        ("macos", "aarch64") => &["zip/mihomo-darwin-arm64.gz"],
        ("windows", "x86") => &["zip/mihomo-windows-386.zip"],
        _ => &[],
    };

    for candidate in candidates {
        if resources::exists(candidate) {
            return Ok(candidate);
        }
    }

    anyhow::bail!(
        "当前平台暂未打包 mihomo 内核资源: {}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

#[cfg(unix)]
fn is_process_running(pid: i32) -> Result<bool> {
    unsafe {
        let result = libc::kill(pid, 0);
        Ok(result == 0)
    }
}

#[cfg(windows)]
fn is_process_running(pid: i32) -> Result<bool> {
    let filter = format!("PID eq {}", pid);
    let output = Command::new("tasklist")
        .args(["/FI", &filter, "/FO", "CSV", "/NH"])
        .output()
        .context("Failed to query process list")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().any(|line| line.contains(&format!(",\"{}\"", pid))))
}

#[cfg(unix)]
fn stop_process(pid: i32, force: bool) -> Result<()> {
    let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
    unsafe {
        libc::kill(pid, signal);
    }
    Ok(())
}

#[cfg(windows)]
fn stop_process(pid: i32, force: bool) -> Result<()> {
    let mut args = vec!["/PID".to_string(), pid.to_string(), "/T".to_string()];
    if force {
        args.push("/F".to_string());
    }
    Command::new("taskkill")
        .args(args.iter().map(|value| value.as_str()))
        .output()
        .context("Failed to stop process")?;
    Ok(())
}
