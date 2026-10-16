//! Rule-set normalization, preserving Surge's first-match precedence.

use std::collections::HashSet;
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::has_text_changed;

/// Return the locally maintained AI additions from the marked section in ai.conf.
/// This section survives upstream refreshes and remains the only source of custom AI rules.
pub fn ai_additions(content: &str) -> Option<&str> {
    let (_, remaining) = content.split_once("# BEGIN LOCAL AI RULES\n")?;
    let (rules, _) = remaining.split_once("# END LOCAL AI RULES")?;
    Some(rules)
}
/// Use the local AI matchers to identify upstream copies in lower-priority rule sets.
/// This deliberately matches concrete domains, not broad substrings like `ai`.
fn ai_covers(ai: &Seen, line: &str) -> bool {
    if let Some((kind, value)) =
        domain_rule(line).filter(|(_, _)| line.trim().split(',').count() == 2)
    {
        let value = value.to_ascii_lowercase();
        if ai.keywords.iter().any(|word| value.contains(word)) {
            return true;
        }
        if kind == "DOMAIN-KEYWORD" {
            return false;
        }
        let mut suffix = value.as_str();
        loop {
            if ai.suffixes.contains(suffix) {
                return true;
            }
            let Some((_, rest)) = suffix.split_once('.') else {
                break;
            };
            suffix = rest;
        }
        return ai.exact.contains(&(kind.to_string(), value));
    }
    false
}

#[derive(Default)]
struct Seen {
    exact: HashSet<(String, String)>,
    suffixes: HashSet<String>,
    keywords: Vec<String>,
    networks: Vec<(IpAddr, u8)>,
}

pub(crate) fn domain_set_entry(line: &str) -> Option<(&'static str, &str)> {
    let entry = line.trim();
    if entry.is_empty() || entry.starts_with('#') || entry.starts_with("//") {
        return None;
    }
    if let Some(suffix) = entry.strip_prefix('.') {
        (!suffix.is_empty()).then_some(("DOMAIN-SUFFIX", suffix))
    } else {
        Some(("DOMAIN", entry))
    }
}
fn normalize_domain_set(content: &str, seen: &mut Seen) -> String {
    let mut output = String::with_capacity(content.len());
    for line in content.lines() {
        let keep = match domain_set_entry(line) {
            Some((kind, value)) => seen.insert(&format!("{kind},{value}")),
            None => true,
        };
        if keep {
            output.push_str(line);
            output.push('\n');
        }
    }
    output
}

fn domain_rule(line: &str) -> Option<(&str, &str)> {
    let (kind, rest) = line.split_once(',')?;
    let value = rest.split(',').next()?.trim();
    if value.is_empty() {
        return None;
    }
    match kind.trim() {
        "DOMAIN" | "DOMAIN-SUFFIX" | "DOMAIN-KEYWORD" => Some((kind.trim(), value)),
        _ => None,
    }
}

pub(crate) fn parse_network(line: &str) -> Option<(IpAddr, u8)> {
    let (kind, rest) = line.split_once(',')?;
    if kind != "IP-CIDR" && kind != "IP-CIDR6" {
        return None;
    }
    let address = rest.split(',').next()?.trim();
    let (ip, length) = match address.split_once('/') {
        Some((ip, length)) => (ip, Some(length)),
        None => (address, None),
    };
    let ip: IpAddr = ip.parse().ok()?;
    if (kind == "IP-CIDR") != ip.is_ipv4() {
        return None;
    }
    let length: u8 = match length {
        Some(length) => length.parse().ok()?,
        None if ip.is_ipv4() => 32,
        None => 128,
    };
    if length > if ip.is_ipv4() { 32 } else { 128 } {
        return None;
    }
    Some((ip, length))
}

pub(crate) fn network_covers(a: (IpAddr, u8), b: (IpAddr, u8)) -> bool {
    if a.1 > b.1 {
        return false;
    }
    match (a.0, b.0) {
        (IpAddr::V4(x), IpAddr::V4(y)) => {
            let mask = u32::MAX.checked_shl(u32::from(32 - a.1)).unwrap_or(0);
            u32::from(x) & mask == u32::from(y) & mask
        }
        (IpAddr::V6(x), IpAddr::V6(y)) => {
            let mask = u128::MAX.checked_shl(u32::from(128 - a.1)).unwrap_or(0);
            u128::from(x) & mask == u128::from(y) & mask
        }
        _ => false,
    }
}

impl Seen {
    fn insert(&mut self, line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            return true;
        }
        if let Some((kind, value)) =
            domain_rule(trimmed).filter(|(_, _)| trimmed.split(',').count() == 2)
        {
            let value = value.to_ascii_lowercase();
            if self.keywords.iter().any(|word| value.contains(word)) {
                return false;
            }
            if kind != "DOMAIN-KEYWORD" {
                let mut suffix = value.as_str();
                loop {
                    if self.suffixes.contains(suffix) {
                        return false;
                    }
                    let Some((_, remaining)) = suffix.split_once('.') else {
                        break;
                    };
                    suffix = remaining;
                }
            }
            if !self.exact.insert((kind.to_string(), value.clone())) {
                return false;
            }
            match kind {
                "DOMAIN-KEYWORD" => self.keywords.push(value),
                "DOMAIN-SUFFIX" => {
                    self.suffixes.insert(value);
                }
                _ => {}
            }
            return true;
        }
        if let Some(net) = parse_network(trimmed).filter(|_| {
            trimmed
                .split(',')
                .skip(2)
                .all(|option| option.trim() == "no-resolve")
        }) {
            if self
                .networks
                .iter()
                .any(|&existing| network_covers(existing, net))
            {
                return false;
            }
            self.networks.push(net);
            return true;
        }
        // Unrecognized matchers may have different flags or matching semantics.
        // Deduplicate only whole identical rule lines.
        self.exact.insert((String::new(), trimmed.to_string()))
    }
}

fn normalize(content: &str, seen: &mut Seen, ai: Option<&Seen>) -> String {
    let mut output = String::with_capacity(content.len());
    let mut count = 0;
    for line in content.lines() {
        if ai.is_some_and(|rules| ai_covers(rules, line)) {
            continue;
        }
        if seen.insert(line) {
            if !line.trim().is_empty()
                && !line.trim().starts_with('#')
                && !line.trim().starts_with("//")
            {
                count += 1;
            }
            if let Some(rest) = line.strip_prefix("# Entries: ") {
                // Entry counts are updated after filtering below.
                if rest.parse::<usize>().is_ok() {
                    output.push_str("# Entries: __COUNT__\n");
                    continue;
                }
            }
            output.push_str(line);
            output.push('\n');
        }
    }
    output.replace("# Entries: __COUNT__", &format!("# Entries: {count}"))
}

pub(crate) fn local_set_path(root: &Path, line: &str) -> Option<PathBuf> {
    let rest = line
        .strip_prefix("RULE-SET,")
        .or_else(|| line.strip_prefix("DOMAIN-SET,"))?;
    let url = rest.split(',').next()?;
    let relative = url
        .strip_prefix("https://raw.githubusercontent.com/hsuyelin/surge-conf/main/")
        .unwrap_or(url);
    if !relative.starts_with("rules/") || !relative.ends_with(".conf") || relative.contains("..") {
        return None;
    }
    Some(root.join(relative))
}

/// Deduplicate all local rule sets according to the order of the template's `[Rule]` section.
/// An earlier narrow rule never removes a later broad rule, since that would change unmatched traffic.
/// Returns the number of files changed.
///
/// # Errors
/// Fails if a referenced rule set cannot be read or written.
pub fn normalize_rules(root: &Path) -> Result<usize> {
    let template = fs::read_to_string(root.join("surge.conf")).context("reading surge.conf")?;
    let mut seen = Seen::default();
    let ai_path = root.join("rules/ai/ai.conf");
    let ai_content = fs::read_to_string(&ai_path).context("reading ai.conf")?;
    let mut ai = Seen::default();
    normalize(&ai_content, &mut ai, None);
    let mut in_rules = false;
    let mut changed = 0;
    let mut files = HashSet::new();
    for line in template.lines() {
        if line.trim() == "[Rule]" {
            in_rules = true;
            continue;
        }
        if line.starts_with('[') {
            in_rules = false;
        }
        if !in_rules || line.starts_with('#') {
            continue;
        }
        if let Some(path) = local_set_path(root, line) {
            let content =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            let output = if line.starts_with("DOMAIN-SET,") {
                normalize_domain_set(&content, &mut seen)
            } else {
                normalize(
                    &content,
                    &mut seen,
                    (path != ai_path && path.starts_with(root.join("rules/proxy"))).then_some(&ai),
                )
            };
            if has_text_changed(&output, &content) {
                fs::write(&path, output).with_context(|| format!("writing {}", path.display()))?;
                changed += 1;
            }
            files.insert(path);
        } else if !line.starts_with("RULE-SET,") && !line.starts_with("DOMAIN-SET,") {
            // Inline rules contain a policy; rule-set entries do not. Seed coverage
            // using the matcher only, so later identical/contained entries are removed.
            let mut fields = line.split(',');
            match (fields.next(), fields.next(), fields.next(), fields.next()) {
                (
                    Some(kind @ ("DOMAIN" | "DOMAIN-SUFFIX" | "DOMAIN-KEYWORD")),
                    Some(value),
                    Some(_policy),
                    None,
                ) => {
                    seen.insert(&format!("{kind},{value}"));
                }
                (
                    Some(kind @ ("IP-CIDR" | "IP-CIDR6")),
                    Some(value),
                    Some(_policy),
                    Some("no-resolve"),
                ) if fields.next().is_none() => {
                    seen.insert(&format!("{kind},{value},no-resolve"));
                }
                _ => {}
            }
        }
    }
    // Process unreferenced local rule sets too, without conflating their independent policy context.
    for category in fs::read_dir(root.join("rules"))? {
        let category = category?;
        if !category.file_type()?.is_dir() {
            continue;
        }
        for entry in fs::read_dir(category.path())? {
            let path = entry?.path();
            if path.extension().is_none_or(|ext| ext != "conf") || files.contains(&path) {
                continue;
            }
            let content = fs::read_to_string(&path)?;
            let output = normalize(&content, &mut Seen::default(), None);
            if has_text_changed(&output, &content) {
                fs::write(&path, output)?;
                changed += 1;
            }
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn domain_set_dedup_respects_exact_suffix_and_keywords() {
        let mut seen = Seen::default();
        assert!(seen.insert("DOMAIN-KEYWORD,cursor"));
        let result = normalize_domain_set("# domains\n.api.cursor.com\n.example.com\nwww.example.com\nnotexample.com\nexample.com\n", &mut seen);
        assert_eq!(result, "# domains\n.example.com\nnotexample.com\n");
    }

    #[test]
    fn keyword_suffix_domain_and_label_boundaries() {
        let mut seen = Seen::default();
        let output = normalize("DOMAIN-KEYWORD,cursor\nDOMAIN-SUFFIX,api.cursor.com\nDOMAIN-SUFFIX,example.com\nDOMAIN,www.example.com\nDOMAIN-SUFFIX,notexample.com\nDOMAIN-KEYWORD,ample\nDOMAIN-SUFFIX,other.net\n", &mut seen, None);
        assert!(output.contains("DOMAIN-SUFFIX,example.com\n"));
        assert!(output.contains("DOMAIN-SUFFIX,notexample.com\n"));
        assert!(output.contains("DOMAIN-SUFFIX,other.net\n"));
        assert!(!output.contains("api.cursor.com"));
        assert!(!output.contains("www.example.com"));
        assert!(output.contains("DOMAIN-KEYWORD,ample"));
    }

    #[test]
    fn narrow_precedence_and_cidr_containment() {
        let mut seen = Seen::default();
        let output = normalize("DOMAIN,api.example.com\nDOMAIN-SUFFIX,example.com\nIP-CIDR,10.0.0.0/8,no-resolve\nIP-CIDR,10.1.0.0/16,no-resolve\nIP-CIDR6,2607:6bc0::/32,no-resolve\nIP-CIDR6,2607:6bc0:1::/48,no-resolve\n", &mut seen, None);
        assert!(output.contains("DOMAIN,api.example.com\nDOMAIN-SUFFIX,example.com"));
        assert!(!output.contains("10.1.0.0"));
        assert!(!output.contains("2607:6bc0:1::"));
    }
    #[test]
    fn single_ip_and_address_families() {
        let ipv4 = parse_network("IP-CIDR,8.8.8.8");
        assert_eq!(ipv4, Some(("8.8.8.8".parse().expect("valid IPv4"), 32)));
        assert_eq!(
            parse_network("IP-CIDR6,2001:db8::1"),
            Some(("2001:db8::1".parse().expect("valid IPv6"), 128))
        );
        assert_eq!(parse_network("IP-CIDR,2001:db8::1"), None);
        assert_eq!(parse_network("IP-CIDR6,8.8.8.8"), None);
    }

    #[test]
    fn local_ai_section_survives_refresh_and_missing_markers_fail() {
        let content = "# header\n# BEGIN LOCAL AI RULES\nDOMAIN-SUFFIX,factory.ai\n# END LOCAL AI RULES\nDOMAIN-SUFFIX,chatgpt.com\n";
        assert_eq!(ai_additions(content), Some("DOMAIN-SUFFIX,factory.ai\n"));
        assert_eq!(ai_additions("DOMAIN-SUFFIX,chatgpt.com"), None);
    }

    #[test]
    fn ai_domains_are_exclusive_but_unrelated_domains_survive() {
        let mut ai = Seen::default();
        normalize(
            "DOMAIN-KEYWORD,cursor\nDOMAIN-SUFFIX,anthropic.com\n",
            &mut ai,
            None,
        );
        let result = normalize(
            "DOMAIN,api.anthropic.com\nDOMAIN-SUFFIX,cursor.sh\nDOMAIN-SUFFIX,gemini.yahoo.com\n",
            &mut Seen::default(),
            Some(&ai),
        );
        assert_eq!(result, "DOMAIN-SUFFIX,gemini.yahoo.com\n");
    }
}
