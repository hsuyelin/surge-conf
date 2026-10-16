//! Rule synchronization tool for Surge configuration
//!
//! This tool downloads rule sets from upstream repositories and organizes
//! them into categorized directories with proper headers.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::Result;

use surge_sync::{
    current_timestamp, download_text, ensure_dir, gh_annotate, has_text_changed, log_status,
    log_sub, project_root,
    rules::{ai_additions, normalize_rules},
    LogLevel, Timer,
};

/// Rule category for directory organization
#[derive(Debug, Clone, Copy)]
enum RuleCategory {
    Adblock,
    Ai,
    Apple,
    Media,
    Social,
    Gaming,
    Proxy,
}

impl RuleCategory {
    fn as_str(&self) -> &'static str {
        match self {
            RuleCategory::Adblock => "adblock",
            RuleCategory::Ai => "ai",
            RuleCategory::Apple => "apple",
            RuleCategory::Media => "media",
            RuleCategory::Social => "social",
            RuleCategory::Gaming => "gaming",
            RuleCategory::Proxy => "proxy",
        }
    }
}

/// Rule source definition
struct RuleSource {
    name: &'static str,
    url: &'static str,
    category: RuleCategory,
}

/// Predefined rule sources extracted from my.conf
fn get_rule_sources() -> Vec<RuleSource> {
    vec![
        // Adblock
        RuleSource {
            name: "adblock4limbo",
            url: "https://raw.githubusercontent.com/limbopro/Adblock4limbo/main/Adblock4limbo_surge.list",
            category: RuleCategory::Adblock,
        },

        // AI
        RuleSource {
            name: "ai",
            url: "https://ruleset.skk.moe/List/non_ip/ai.conf",
            category: RuleCategory::Ai,
        },

        // Apple
        RuleSource {
            name: "appleCn",
            url: "https://ruleset.skk.moe/List/non_ip/apple_cn.conf",
            category: RuleCategory::Apple,
        },
        RuleSource {
            name: "appleServices",
            url: "https://ruleset.skk.moe/List/non_ip/apple_services.conf",
            category: RuleCategory::Apple,
        },
        RuleSource {
            name: "appleServicesIp",
            url: "https://ruleset.skk.moe/List/ip/apple_services.conf",
            category: RuleCategory::Apple,
        },

        // Media
        RuleSource {
            name: "emby",
            url: "https://raw.githubusercontent.com/blackmatrix7/ios_rule_script/refs/heads/master/rule/Surge/Emby/Emby.list",
            category: RuleCategory::Media,
        },
        RuleSource {
            name: "youtube",
            url: "https://raw.githubusercontent.com/blackmatrix7/ios_rule_script/master/rule/Surge/YouTube/YouTube.list",
            category: RuleCategory::Media,
        },
        RuleSource {
            name: "spotify",
            url: "https://raw.githubusercontent.com/blackmatrix7/ios_rule_script/master/rule/Surge/Spotify/Spotify.list",
            category: RuleCategory::Media,
        },
        RuleSource {
            name: "bilibili",
            url: "https://raw.githubusercontent.com/blackmatrix7/ios_rule_script/master/rule/Surge/BiliBili/BiliBili.list",
            category: RuleCategory::Media,
        },
        RuleSource {
            name: "streamNonIp",
            url: "https://ruleset.skk.moe/List/non_ip/stream.conf",
            category: RuleCategory::Media,
        },
        RuleSource {
            name: "streamIp",
            url: "https://ruleset.skk.moe/List/ip/stream.conf",
            category: RuleCategory::Media,
        },

        // Social
        RuleSource {
            name: "telegram",
            url: "https://raw.githubusercontent.com/blackmatrix7/ios_rule_script/master/rule/Surge/Telegram/Telegram.list",
            category: RuleCategory::Social,
        },
        RuleSource {
            name: "discord",
            url: "https://raw.githubusercontent.com/blackmatrix7/ios_rule_script/master/rule/Surge/Discord/Discord.list",
            category: RuleCategory::Social,
        },

        // Gaming
        RuleSource {
            name: "game",
            url: "https://raw.githubusercontent.com/blackmatrix7/ios_rule_script/master/rule/Surge/Game/Game.list",
            category: RuleCategory::Gaming,
        },

        // Proxy
        RuleSource {
            name: "global",
            url: "https://raw.githubusercontent.com/blackmatrix7/ios_rule_script/refs/heads/master/rule/Surge/Global/Global_All_No_Resolve.list",
            category: RuleCategory::Proxy,
        },
        RuleSource {
            name: "china",
            url: "https://raw.githubusercontent.com/blackmatrix7/ios_rule_script/refs/heads/master/rule/Surge/China/China_All_No_Resolve.list",
            category: RuleCategory::Proxy,
        },
    ]
}

/// Count the number of rule entries in the content
fn count_entries(content: &str) -> usize {
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("//")
        })
        .count()
}

/// Generate a standardized header for a rule file
fn generate_header(name: &str, upstream_url: &str, entry_count: usize) -> String {
    format!(
        r#"#########################################
# {}
# Last Updated: {}
# Entries: {}
# Upstream: {}
# GitHub: https://github.com/hsuyelin/surge-conf
#########################################
"#,
        name,
        current_timestamp(),
        entry_count,
        upstream_url
    )
}

/// Strip existing header comments and return clean content with original rules
fn strip_header(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut start_idx = 0;

    // Skip leading comment blocks
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            start_idx = i + 1;
        } else {
            break;
        }
    }

    // Find where actual rules start (skip blank lines after header)
    while start_idx < lines.len() && lines[start_idx].trim().is_empty() {
        start_idx += 1;
    }

    lines[start_idx..].join("\n")
}

/// Download and process a single rule file
/// Returns Ok(true) if the file was updated, Ok(false) if skipped (unchanged)
fn sync_rule(source: &RuleSource, rules_dir: &Path) -> Result<bool> {
    let category_dir = rules_dir.join(source.category.as_str());
    ensure_dir(&category_dir)?;

    // Always use .conf extension
    let filename = format!("{}.conf", source.name);
    let file_path = category_dir.join(&filename);

    // Download content
    let content = download_text(source.url)?;

    // Strip original header and count entries
    let rule_content = strip_header(&content);
    let rule_content = if source.name == "ai" {
        let existing = fs::read_to_string(&file_path)?;
        let local = ai_additions(&existing)
            .ok_or_else(|| anyhow::anyhow!("ai.conf missing local AI rules markers"))?;
        format!("# BEGIN LOCAL AI RULES\n{local}# END LOCAL AI RULES\n{rule_content}")
    } else {
        rule_content
    };
    let entry_count = count_entries(&rule_content);

    // Generate new header
    let header = generate_header(source.name, source.url, entry_count);

    // Write file with new header + original rules
    let final_content = format!("{}\n{}", header, rule_content);

    // Check if content has actually changed (ignoring timestamp)
    if file_path.exists() {
        let existing_content = fs::read_to_string(&file_path)?;
        if !has_text_changed(&final_content, &existing_content) {
            return Ok(false);
        }
    }

    fs::write(&file_path, final_content)?;
    Ok(true)
}

fn main() -> Result<()> {
    log_status("Syncing", "rules from upstream...", LogLevel::Info);
    let timer = Timer::start("syncing");

    let root = project_root()?;
    let rules_dir = root.join("rules");
    ensure_dir(&rules_dir)?;

    let sources = get_rule_sources();
    let mut success_count = 0;
    let mut updated_count = 0;
    let total = sources.len();

    if std::env::args().any(|arg| arg == "--normalize-only") {
        let normalized = normalize_rules(&root)?;
        log_sub(&format!("Normalized {normalized} rule sets"));
        return Ok(());
    }

    // Upstream files are downloaded before normalization; retain an already-normalized
    // file's header when the resulting rules are unchanged.
    let mut previous = HashMap::new();
    for source in &sources {
        let path = rules_dir
            .join(source.category.as_str())
            .join(format!("{}.conf", source.name));
        if let Ok(content) = fs::read_to_string(&path) {
            previous.insert(path, content);
        }
    }

    for source in &sources {
        log_sub(&format!("Downloading {}", source.name));

        match sync_rule(source, &rules_dir) {
            Ok(changed) => {
                success_count += 1;
                if changed {
                    updated_count += 1;
                    log_sub(&format!("{} updated", source.name));
                } else {
                    log_sub(&format!("{} unchanged, skipped", source.name));
                }
            }
            Err(e) => {
                gh_annotate("warning", &format!("Failed to sync {}: {}", source.name, e));
                // Continue with other rules - skip failed ones
            }
        }
    }

    let normalized = normalize_rules(&root)?;
    log_sub(&format!("Normalized {normalized} rule sets"));
    for (path, original) in previous {
        let current = fs::read_to_string(&path)?;
        if !has_text_changed(&current, &original) {
            fs::write(path, original)?;
        }
    }

    timer.stop(success_count);

    log_status(
        "Summary",
        &format!(
            "{} updated, {} unchanged, {} failed",
            updated_count,
            success_count - updated_count,
            total - success_count
        ),
        LogLevel::Info,
    );

    if success_count < total {
        log_status(
            "Warning",
            &format!("{} rules failed to sync", total - success_count),
            LogLevel::Warning,
        );
    }

    Ok(())
}
