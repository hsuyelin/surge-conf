//! Icon synchronization tool for Surge configuration
//!
//! This tool downloads icons from upstream repositories and generates
//! an icons.json index file.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use serde::{Deserialize, Serialize};

use surge_sync::{
    current_timestamp, download_url, ensure_dir, gh_annotate, log_status, log_sub, LogLevel, Timer,
};

/// Icon entry in the JSON index
#[derive(Serialize, Deserialize, Clone)]
struct IconEntry {
    name: String,
    url: String,
}

/// Icon index JSON structure
#[derive(Serialize, Deserialize)]
struct IconIndex {
    name: String,
    description: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    icons: Vec<IconEntry>,
}

/// Icon category for directory organization
#[derive(Debug, Clone, Copy)]
enum IconCategory {
    Apps,
    Country,
    Policy,
}

impl IconCategory {
    fn as_str(&self) -> &'static str {
        match self {
            IconCategory::Apps => "apps",
            IconCategory::Country => "country",
            IconCategory::Policy => "policy",
        }
    }
}

/// Predefined icon URLs extracted from my.conf
fn get_icon_sources() -> Vec<(&'static str, &'static str, IconCategory)> {
    vec![
        // Apps
        ("chatgpt", "https://raw.githubusercontent.com/fmz200/wool_scripts/main/icons/apps/ChatGPT.png", IconCategory::Apps),
        ("youtube", "https://raw.githubusercontent.com/fmz200/wool_scripts/main/icons/apps/YouTube_02.png", IconCategory::Apps),
        ("spotify", "https://raw.githubusercontent.com/fmz200/wool_scripts/main/icons/apps/Spotify_02.png", IconCategory::Apps),
        ("telegram", "https://raw.githubusercontent.com/fmz200/wool_scripts/main/icons/apps/Telegram_03.png", IconCategory::Apps),
        ("bilibiliTv", "https://raw.githubusercontent.com/fmz200/wool_scripts/main/icons/apps/BiliBiliTV.png", IconCategory::Apps),
        ("discord", "https://raw.githubusercontent.com/fmz200/wool_scripts/main/icons/apps/Discord.png", IconCategory::Apps),
        ("game", "https://raw.githubusercontent.com/fmz200/wool_scripts/main/icons/apps/Game.png", IconCategory::Apps),
        ("google", "https://raw.githubusercontent.com/fmz200/wool_scripts/main/icons/apps/Google_02.png", IconCategory::Apps),
        ("apple", "https://raw.githubusercontent.com/Koolson/Qure/master/IconSet/Color/Apple_1.png", IconCategory::Apps),

        // Country
        ("hk", "https://raw.githubusercontent.com/erdongchanyo/icon/main/Policy-Country/HK02.png", IconCategory::Country),
        ("tw", "https://raw.githubusercontent.com/erdongchanyo/icon/main/Policy-Country/TW.png", IconCategory::Country),
        ("jp", "https://raw.githubusercontent.com/erdongchanyo/icon/main/Policy-Country/JP.png", IconCategory::Country),
        ("kr", "https://raw.githubusercontent.com/erdongchanyo/icon/main/Policy-Country/KR.png", IconCategory::Country),
        ("sg", "https://raw.githubusercontent.com/erdongchanyo/icon/main/Policy-Country/SG.png", IconCategory::Country),
        ("us", "https://raw.githubusercontent.com/erdongchanyo/icon/main/Policy-Country/US.png", IconCategory::Country),
        ("uk", "https://raw.githubusercontent.com/erdongchanyo/icon/main/Policy-Country/UK.png", IconCategory::Country),
        ("in", "https://raw.githubusercontent.com/erdongchanyo/icon/main/Policy-Country/IN.png", IconCategory::Country),

        // Policy
        ("surge", "https://raw.githubusercontent.com/Irrucky/Tool/main/Surge/icon/surge_2.png", IconCategory::Policy),
        ("final", "https://raw.githubusercontent.com/erdongchanyo/icon/main/Policy-Filter/Final01.png", IconCategory::Policy),
        ("vpn", "https://raw.githubusercontent.com/blackmatrix7/ios_rule_script/master/icon/color/vpn.png", IconCategory::Policy),
        ("gMedia", "https://raw.githubusercontent.com/erdongchanyo/icon/main/Policy-Filter/GMedia.png", IconCategory::Policy),
        ("emby", "https://raw.githubusercontent.com/erdongchanyo/icon/main/Policy-Filter/Emby.png", IconCategory::Policy),
    ]
}

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

/// Download a single icon and save it to the appropriate directory
fn download_icon(
    name: &str,
    url: &str,
    category: IconCategory,
    icons_dir: &Path,
) -> Result<PathBuf> {
    let category_dir = icons_dir.join(category.as_str());
    ensure_dir(&category_dir)?;

    // Get file extension from URL
    let extension = url.rsplit('.').next().unwrap_or("png");

    let filename = format!("{}.{}", name, extension);
    let file_path = category_dir.join(&filename);

    // Download the icon
    let data = download_url(url)?;
    fs::write(&file_path, data)?;

    Ok(file_path)
}

/// Generate the icons.json index file
fn generate_index(icons: &[(String, String, IconCategory)], icons_dir: &Path) -> Result<()> {
    let github_base = "https://raw.githubusercontent.com/hsuyelin/surge-conf/main/icons";

    let entries: Vec<IconEntry> = icons
        .iter()
        .map(|(name, _, category)| {
            // Get the file extension (default to png)
            let extension = "png";
            IconEntry {
                name: name.clone(),
                url: format!(
                    "{}/{}/{}.{}",
                    github_base,
                    category.as_str(),
                    name,
                    extension
                ),
            }
        })
        .collect();

    let index = IconIndex {
        name: "Surge Icons".to_string(),
        description: "Icons collected from the internet, copyright belongs to original authors"
            .to_string(),
        updated_at: current_timestamp(),
        icons: entries,
    };

    let json = serde_json::to_string_pretty(&index)?;
    fs::write(icons_dir.join("icons.json"), json)?;

    Ok(())
}

fn main() -> Result<()> {
    log_status("Syncing", "icons from upstream...", LogLevel::Info);
    let timer = Timer::start("syncing");

    let root = get_project_root();
    let icons_dir = root.join("icons");
    ensure_dir(&icons_dir)?;

    let sources = get_icon_sources();
    let mut success_count = 0;
    let mut downloaded_icons: Vec<(String, String, IconCategory)> = Vec::new();

    for (name, url, category) in &sources {
        log_sub(&format!("Downloading {}.png", name));

        match download_icon(name, url, *category, &icons_dir) {
            Ok(_) => {
                success_count += 1;
                downloaded_icons.push((name.to_string(), url.to_string(), *category));
            }
            Err(e) => {
                gh_annotate("warning", &format!("Failed to download {}: {}", name, e));
                // Continue with other icons
            }
        }
    }

    // Generate index file
    log_sub("Generating icons.json");
    generate_index(&downloaded_icons, &icons_dir)?;

    timer.stop(success_count);

    if success_count < sources.len() {
        log_status(
            "Warning",
            &format!("{} icons failed to download", sources.len() - success_count),
            LogLevel::Warning,
        );
    }

    Ok(())
}
