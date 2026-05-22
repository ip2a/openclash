use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const COMMON_RESOURCES: &[&str] = &["Country.mmdb", "mixin.yaml", "zip/yacd.tar.xz"];

fn main() {
    println!("cargo:rerun-if-changed=resources");
    println!("cargo:rerun-if-changed=checksums.txt");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_OS");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ARCH");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("missing manifest dir"));
    let source_dir = manifest_dir.join("resources");
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("missing target os");
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").expect("missing target arch");
    let relative_out_dir = PathBuf::from("target")
        .join("openclash-resources")
        .join(format!("{}-{}", target_os, target_arch));
    let out_dir = manifest_dir.join(&relative_out_dir);

    if out_dir.exists() {
        fs::remove_dir_all(&out_dir).expect("failed to clean embedded resource directory");
    }
    fs::create_dir_all(&out_dir).expect("failed to create embedded resource directory");

    for resource in COMMON_RESOURCES.iter().chain(target_resources().iter()) {
        copy_resource(&source_dir, &out_dir, resource);
    }

    println!(
        "cargo:rustc-env=OPENCLASH_EMBED_DIR={}",
        relative_out_dir.display()
    );
}

fn target_resources() -> &'static [&'static str] {
    match (
        env::var("CARGO_CFG_TARGET_OS").as_deref(),
        env::var("CARGO_CFG_TARGET_ARCH").as_deref(),
    ) {
        (Ok("linux"), Ok("x86_64")) => &[
            "zip/mihomo-linux-amd64-compatible.gz",
            "zip/subconverter_linux64.tar.gz",
            "zip/yq_linux_amd64.tar.gz",
        ],
        (Ok("linux"), Ok("aarch64")) => &["zip/mihomo-linux-arm64.gz"],
        (Ok("macos"), Ok("aarch64")) => &["zip/mihomo-darwin-arm64.gz"],
        (Ok("windows"), Ok("x86")) => &["zip/mihomo-windows-386.zip"],
        _ => &[],
    }
}

fn copy_resource(source_dir: &Path, out_dir: &Path, relative_path: &str) {
    let source = source_dir.join(relative_path);
    if !source.exists() {
        panic!("missing resource: {}", source.display());
    }

    let target = out_dir.join(relative_path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).expect("failed to create embedded resource parent directory");
    }
    fs::copy(&source, &target).unwrap_or_else(|error| {
        panic!(
            "failed to copy resource {} -> {}: {}",
            source.display(),
            target.display(),
            error
        )
    });
}
