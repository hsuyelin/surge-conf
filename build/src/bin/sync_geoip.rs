//! GeoIP database synchronization tool for Surge configuration
//!
//! This tool downloads the GeoIP MaxMind database (mmdb) from upstream
//! and saves it locally.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use surge_sync::{download_url, ensure_dir, gh_annotate, log_status, log_sub, LogLevel, Timer};

/// GeoIP database source configuration
const GEOIP_SOURCE: &str = "https://github.com/Hackl0us/GeoIP2-CN/raw/release/Country.mmdb";
const GEOIP_FILENAME: &str = "Country.mmdb";

/// Get the project root directory
fn get_project_root() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap());

    // If running from build directory, go up one level
    if manifest_dir.ends_with("build") {
        manifest_dir.parent().unwrap().to_path_buf()
    } else {
        manifest_dir
    }
}

/// Download the GeoIP database and save it locally
fn download_geoip(geoip_dir: &Path) -> Result<()> {
    log_sub(&format!("Downloading {}", GEOIP_FILENAME));

    let data = download_url(GEOIP_SOURCE)?;
    let file_path = geoip_dir.join(GEOIP_FILENAME);
    fs::write(&file_path, &data)?;

    log_sub(&format!("Saved {} ({} bytes)", GEOIP_FILENAME, data.len()));

    Ok(())
}

fn main() -> Result<()> {
    log_status("Syncing", "GeoIP database from upstream...", LogLevel::Info);
    let timer = Timer::start("syncing");

    let root = get_project_root();
    let geoip_dir = root.join("geoip");
    ensure_dir(&geoip_dir)?;

    match download_geoip(&geoip_dir) {
        Ok(_) => {
            timer.stop(1);
        }
        Err(e) => {
            gh_annotate(
                "error",
                &format!("Failed to download GeoIP database: {}", e),
            );
            log_status(
                "Error",
                &format!("Failed to download GeoIP database: {}", e),
                LogLevel::Error,
            );
            return Err(e);
        }
    }

    Ok(())
}
