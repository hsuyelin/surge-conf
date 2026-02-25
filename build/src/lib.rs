//! Common utilities for Surge sync tools
//!
//! Provides logging utilities with Cargo-style output and color support for GitHub Actions.

use std::time::Instant;

/// ANSI color codes for terminal output
pub mod colors {
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const RED: &str = "\x1b[31m";
    pub const CYAN: &str = "\x1b[36m";
    pub const BOLD: &str = "\x1b[1m";
    pub const RESET: &str = "\x1b[0m";
}

/// Log levels for structured output
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// Print a Cargo-style status message
///
/// # Arguments
/// * `status` - The status label (e.g., "Syncing", "Downloading")
/// * `message` - The message content
/// * `level` - The log level for color coding
pub fn log_status(status: &str, message: &str, level: LogLevel) {
    let color = match level {
        LogLevel::Info => colors::CYAN,
        LogLevel::Success => colors::GREEN,
        LogLevel::Warning => colors::YELLOW,
        LogLevel::Error => colors::RED,
    };

    println!("{}{:>12}{} {}", color, status, colors::RESET, message);
}

/// Print a sub-item with arrow prefix
///
/// # Arguments
/// * `message` - The message to display
pub fn log_sub(message: &str) {
    println!("    {}===>{}  {}", colors::CYAN, colors::RESET, message);
}

/// Print a GitHub Actions annotation
///
/// # Arguments
/// * `level` - The annotation level (warning, error)
/// * `message` - The annotation message
pub fn gh_annotate(level: &str, message: &str) {
    println!("::{}::{}", level, message);
}

/// Timer for measuring operation duration
pub struct Timer {
    start: Instant,
    label: String,
}

impl Timer {
    /// Create and start a new timer
    pub fn start(label: &str) -> Self {
        Self {
            start: Instant::now(),
            label: label.to_string(),
        }
    }

    /// Stop the timer and log the elapsed time
    pub fn stop(self, count: usize) {
        let elapsed = self.start.elapsed();
        log_status(
            "Finished",
            &format!("{} {} in {:.2}s", self.label, count, elapsed.as_secs_f64()),
            LogLevel::Success,
        );
    }
}

/// Download content from a URL with error handling
///
/// # Arguments
/// * `url` - The URL to download from
///
/// # Returns
/// * `Ok(Vec<u8>)` - The downloaded content as bytes
/// * `Err` - If the download fails
pub fn download_url(url: &str) -> anyhow::Result<Vec<u8>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client.get(url).send()?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP {} for {}", response.status(), url);
    }

    Ok(response.bytes()?.to_vec())
}

/// Download text content from a URL
///
/// # Arguments
/// * `url` - The URL to download from
///
/// # Returns
/// * `Ok(String)` - The downloaded content as text
/// * `Err` - If the download fails
pub fn download_text(url: &str) -> anyhow::Result<String> {
    let bytes = download_url(url)?;
    Ok(String::from_utf8(bytes)?)
}

/// Convert a name to lowercase camelCase format
///
/// # Arguments
/// * `name` - The original name
///
/// # Returns
/// * The converted name in camelCase
pub fn to_camel_case(name: &str) -> String {
    let parts: Vec<&str> = name.split(['_', '-', ' ']).collect();
    if parts.is_empty() {
        return String::new();
    }

    let mut result = parts[0].to_lowercase();
    for part in parts.iter().skip(1) {
        if !part.is_empty() {
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                result.push(first.to_ascii_uppercase());
                result.extend(chars.map(|c| c.to_ascii_lowercase()));
            }
        }
    }
    result
}

/// Get the current timestamp in ISO format
pub fn current_timestamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Ensure a directory exists, creating it if necessary
pub fn ensure_dir(path: &std::path::Path) -> anyhow::Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Compare two text contents ignoring "# Last Updated:" lines.
/// Returns true if meaningful content has changed.
pub fn has_text_changed(new_content: &str, existing_content: &str) -> bool {
    let filter = |line: &&str| {
        let trimmed = line.trim();
        !trimmed.starts_with("# Last Updated:")
    };
    let new_lines: Vec<&str> = new_content.lines().filter(filter).collect();
    let old_lines: Vec<&str> = existing_content.lines().filter(filter).collect();
    new_lines != old_lines
}

/// Compare new binary data against an existing file on disk.
/// Returns true if the content is different or the file doesn't exist.
pub fn has_binary_changed(new_data: &[u8], file_path: &std::path::Path) -> bool {
    match std::fs::read(file_path) {
        Ok(existing) => existing != new_data,
        Err(_) => true, // File doesn't exist, treat as changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("chat_gpt"), "chatGpt");
        assert_eq!(to_camel_case("you-tube"), "youTube");
        assert_eq!(to_camel_case("DISCORD"), "discord");
        assert_eq!(to_camel_case("Apple 1"), "apple1");
    }

    #[test]
    fn test_has_text_changed_identical_content() {
        let old = "#########################################\n# test\n# Last Updated: 2026-02-24 02:09:28\n# Entries: 10\n#########################################\nRULE1\nRULE2\n";
        let new = "#########################################\n# test\n# Last Updated: 2026-02-25 14:00:00\n# Entries: 10\n#########################################\nRULE1\nRULE2\n";
        assert!(!has_text_changed(new, old));
    }

    #[test]
    fn test_has_text_changed_different_content() {
        let old = "#########################################\n# test\n# Last Updated: 2026-02-24 02:09:28\n# Entries: 10\n#########################################\nRULE1\nRULE2\n";
        let new = "#########################################\n# test\n# Last Updated: 2026-02-25 14:00:00\n# Entries: 11\n#########################################\nRULE1\nRULE2\nRULE3\n";
        assert!(has_text_changed(new, old));
    }

    #[test]
    fn test_has_text_changed_new_file() {
        let new = "# test\n# Last Updated: 2026-02-25\nRULE1\n";
        assert!(has_text_changed(new, ""));
    }
}
