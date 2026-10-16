//! GeoIP database synchronization tool for Surge configuration
//!
//! This tool downloads the GeoIP MaxMind database (mmdb) from upstream
//! and saves it locally.

use std::fs;
use std::path::Path;

use anyhow::Result;

use surge_sync::{
    download_url, ensure_dir, gh_annotate, has_binary_changed, log_status, log_sub, project_root,
    LogLevel, Timer,
};

/// GeoIP database source configuration
const GEOIP_SOURCE: &str = "https://github.com/Hackl0us/GeoIP2-CN/raw/release/Country.mmdb";
const GEOIP_FILENAME: &str = "Country.mmdb";

/// Download the GeoIP database and save it locally
/// Returns Ok(true) if the file was updated, Ok(false) if skipped (unchanged)
fn download_geoip(geoip_dir: &Path) -> Result<bool> {
    log_sub(&format!("Downloading {}", GEOIP_FILENAME));

    let data = download_url(GEOIP_SOURCE)?;
    let file_path = geoip_dir.join(GEOIP_FILENAME);

    // Check if content has actually changed
    if !has_binary_changed(&data, &file_path) {
        log_sub(&format!("{} unchanged, skipped", GEOIP_FILENAME));
        return Ok(false);
    }

    fs::write(&file_path, &data)?;
    log_sub(&format!("Saved {} ({} bytes)", GEOIP_FILENAME, data.len()));

    Ok(true)
}

fn main() -> Result<()> {
    log_status("Syncing", "GeoIP database from upstream...", LogLevel::Info);
    let timer = Timer::start("syncing");

    let root = project_root()?;
    let geoip_dir = root.join("geoip");
    ensure_dir(&geoip_dir)?;

    match download_geoip(&geoip_dir) {
        Ok(changed) => {
            timer.stop(1);
            if changed {
                log_status("Updated", "GeoIP database", LogLevel::Success);
            } else {
                log_status("Skipped", "GeoIP database (unchanged)", LogLevel::Info);
            }
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
