use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Embedded resources bundled at compile time
#[derive(RustEmbed)]
#[folder = "$OPENCLASH_EMBED_DIR"]
pub struct Resources;

/// Get embedded file content
pub fn get(name: &str) -> Option<Cow<'static, [u8]>> {
    Resources::get(name).map(|f| f.data)
}

/// Extract embedded file to target path
pub fn extract(name: &str, target: &Path) -> anyhow::Result<()> {
    let data = get(name).ok_or_else(|| anyhow::anyhow!("Embedded resource not found: {}", name))?;

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(target)?;
    file.write_all(&data)?;
    Ok(())
}

/// Check if embedded resource exists
pub fn exists(name: &str) -> bool {
    get(name).is_some()
}
