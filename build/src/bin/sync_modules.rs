//! Module synchronization tool for Surge configuration
//!
//! This tool downloads Surge modules from upstream repositories.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use surge_sync::{
    current_timestamp, download_text, ensure_dir, gh_annotate, log_status, log_sub, LogLevel, Timer,
};

/// Module category for directory organization
#[derive(Debug, Clone, Copy)]
enum ModuleCategory {
    Enhance,  // Enhancement modules
    Adblock,  // Ad blocking modules
    Utility,  // Utility modules
    Subtitle, // Subtitle modules
}

impl ModuleCategory {
    fn as_str(&self) -> &'static str {
        match self {
            ModuleCategory::Enhance => "enhance",
            ModuleCategory::Adblock => "adblock",
            ModuleCategory::Utility => "utility",
            ModuleCategory::Subtitle => "subtitle",
        }
    }
}

/// Module source definition
struct ModuleSource {
    name: &'static str,
    url: &'static str,
    category: ModuleCategory,
}

/// Predefined module sources
fn get_module_sources() -> Vec<ModuleSource> {
    vec![
        // Enhance
        ModuleSource {
            name: "googleRedirect",
            url: "https://raw.githubusercontent.com/QingRex/LoonKissSurge/refs/heads/main/Surge/Beta/Google%E9%87%8D%E5%AE%9A%E5%90%91.beta.sgmodule",
            category: ModuleCategory::Enhance,
        },
        ModuleSource {
            name: "bilibili",
            url: "https://raw.githubusercontent.com/kokoryh/Sparkle/refs/heads/master/release/surge/module/bilibili.sgmodule",
            category: ModuleCategory::Enhance,
        },
        ModuleSource {
            name: "telegramIp",
            url: "https://raw.githubusercontent.com/Repcz/Tool/X/Surge/Module/Function/FKTG.sgmodule",
            category: ModuleCategory::Enhance,
        },
        ModuleSource {
            name: "googleCaptcha",
            url: "https://raw.githubusercontent.com/NobyDa/Script/master/Surge/Module/GoogleCAPTCHA.sgmodule",
            category: ModuleCategory::Enhance,
        },

        // Adblock
        ModuleSource {
            name: "baiduIndex",
            url: "https://raw.githubusercontent.com/Keywos/rule/main/script/baidu_index/bd.sgmodule",
            category: ModuleCategory::Adblock,
        },
        ModuleSource {
            name: "spotify",
            url: "https://raw.githubusercontent.com/001ProMax/Surge/refs/heads/main/Module/AD/Spotify.sgmodule",
            category: ModuleCategory::Adblock,
        },

        // Utility
        ModuleSource {
            name: "hideVpnIcon",
            url: "https://raw.githubusercontent.com/QingRex/LoonKissSurge/refs/heads/main/Surge/Official/%E9%9A%90%E8%97%8F%E7%8A%B6%E6%80%81%E6%A0%8F%20VPN%20%E5%9B%BE%E6%A0%87.official.sgmodule",
            category: ModuleCategory::Utility,
        },
        ModuleSource {
            name: "wechatUnblock",
            url: "https://raw.githubusercontent.com/zZPiglet/Task/master/UnblockURLinWeChat.sgmodule",
            category: ModuleCategory::Utility,
        },
        ModuleSource {
            name: "spotifyHifi",
            url: "https://raw.githubusercontent.com/app2smile/rules/master/module/spotify.module",
            category: ModuleCategory::Utility,
        },
        ModuleSource {
            name: "ipPurity",
            url: "https://raw.githubusercontent.com/Likhixang/Egerny/refs/heads/main/sgmodule/IPPure.sgmodule",
            category: ModuleCategory::Utility,
        },

        // Subtitle
        ModuleSource {
            name: "youtube",
            url: "https://github.com/DualSubs/YouTube/releases/latest/download/DualSubs.YouTube.sgmodule",
            category: ModuleCategory::Subtitle,
        },
        ModuleSource {
            name: "universal",
            url: "https://github.com/DualSubs/Universal/releases/latest/download/DualSubs.Universal.sgmodule",
            category: ModuleCategory::Subtitle,
        },
    ]
}

/// Generate a standardized header for a module file
fn generate_header(name: &str, upstream_url: &str) -> String {
    format!(
        r#"#########################################
# {}
# Last Updated: {}
# Upstream: {}
# GitHub: https://github.com/hsuyelin/surge-conf
#########################################
"#,
        name,
        current_timestamp(),
        upstream_url
    )
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

/// Download and process a single module file
fn sync_module(source: &ModuleSource, modules_dir: &Path) -> Result<()> {
    let category_dir = modules_dir.join(source.category.as_str());
    ensure_dir(&category_dir)?;

    let filename = format!("{}.sgmodule", source.name);
    let file_path = category_dir.join(&filename);

    // Download content
    let content = download_text(source.url)?;

    // Generate new header
    let header = generate_header(source.name, source.url);

    // Write file with new header + original content
    let final_content = format!("{}\n{}", header, content);
    fs::write(&file_path, final_content)?;

    Ok(())
}

fn main() -> Result<()> {
    log_status("Syncing", "modules from upstream...", LogLevel::Info);
    let timer = Timer::start("syncing");

    let root = get_project_root();
    let modules_dir = root.join("modules");
    ensure_dir(&modules_dir)?;

    let sources = get_module_sources();
    let mut success_count = 0;
    let total = sources.len();

    for source in &sources {
        log_sub(&format!("Downloading {}", source.name));

        match sync_module(source, &modules_dir) {
            Ok(_) => {
                success_count += 1;
            }
            Err(e) => {
                gh_annotate("warning", &format!("Failed to sync {}: {}", source.name, e));
                // Continue with other modules - skip failed ones
            }
        }
    }

    timer.stop(success_count);

    if success_count < total {
        log_status(
            "Warning",
            &format!("{} modules failed to sync", total - success_count),
            LogLevel::Warning,
        );
    }

    Ok(())
}
