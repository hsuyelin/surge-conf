//! Inspect local Surge routing rules in profile order without network access.
use std::fs;
use std::net::IpAddr;
use std::path::Path;

use anyhow::{Context, Result};

use crate::rules::{domain_set_entry, local_set_path, network_covers, parse_network};

/// A local rule that matches an input, in profile evaluation order.
#[derive(Debug, PartialEq, Eq)]
pub struct Match {
    /// Configuration file or `surge.conf` for an inline rule.
    pub file: String,
    /// One-based line number inside the file.
    pub line: usize,
    /// Rule text.
    pub rule: String,
    /// Policy from the referencing rule-set or inline rule.
    pub policy: String,
}

/// Outcome of a local lookup. Unavailable external or built-in rules are reported rather than guessed.
#[derive(Debug, Default)]
pub struct Lookup {
    /// Matching local rules in profile order.
    pub matches: Vec<Match>,
    /// Rule sets not available locally and database-backed rules that cannot be evaluated.
    pub unknown: Vec<String>,
}

fn suffix_covers(suffix: &str, host: &str) -> bool {
    host == suffix
        || host
            .strip_suffix(suffix)
            .is_some_and(|prefix| prefix.ends_with('.'))
}

fn matches_rule(rule: &str, input: &str, address: Option<IpAddr>, keyword: bool) -> bool {
    let Some((kind, rest)) = rule.split_once(',') else {
        return false;
    };
    let value = rest.split(',').next().unwrap_or_default().trim();
    let value_lower = value.to_ascii_lowercase();
    if keyword {
        return matches!(kind, "DOMAIN" | "DOMAIN-SUFFIX" | "DOMAIN-KEYWORD")
            && value_lower.contains(input);
    }
    if let Some(ip) = address {
        return matches!(kind, "IP-CIDR" | "IP-CIDR6")
            && parse_network(rule)
                .is_some_and(|net| network_covers(net, (ip, if ip.is_ipv4() { 32 } else { 128 })));
    }
    match kind {
        "DOMAIN" => input == value_lower,
        "DOMAIN-SUFFIX" => suffix_covers(&value_lower, input),
        "DOMAIN-KEYWORD" => input.contains(&value_lower),
        _ => false,
    }
}

struct Query<'a> {
    input: &'a str,
    address: Option<IpAddr>,
    keyword: bool,
}

fn scan_set(
    root: &Path,
    path: &Path,
    domain_set: bool,
    query: &Query<'_>,
    policy: &str,
    lookup: &mut Lookup,
) -> Result<()> {
    let content =
        fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let display = path
        .strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string();
    for (index, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        let matched = if domain_set {
            domain_set_entry(trimmed).is_some_and(|(kind, value)| {
                let value = value.to_ascii_lowercase();
                if query.keyword {
                    value.contains(query.input)
                } else if query.address.is_some() {
                    false
                } else if kind == "DOMAIN" {
                    query.input == value
                } else {
                    suffix_covers(&value, query.input)
                }
            })
        } else {
            matches_rule(trimmed, query.input, query.address, query.keyword)
        };
        if matched {
            lookup.matches.push(Match {
                file: display.clone(),
                line: index + 1,
                rule: trimmed.to_string(),
                policy: policy.to_string(),
            });
        }
    }
    Ok(())
}

/// Look up a hostname, a substring (`keyword=true`), or an IPv4/IPv6 literal in local rule sets.
/// Matches are in `surge.conf` order; the first local match is effective only if no earlier
/// unavailable built-in/remote rule or DNS-dependent rule matches. Hostname lookups do not
/// resolve DNS, so IP-based policies for hostnames cannot be established by this command.
///
/// # Errors
/// Returns an error if `surge.conf` or any referenced local file cannot be read.
pub fn lookup(root: &Path, query: &str, keyword: bool) -> Result<Lookup> {
    let raw = query.trim().trim_end_matches('.');
    anyhow::ensure!(!raw.is_empty(), "input cannot be empty");
    let input = raw.to_ascii_lowercase();
    let address = if keyword {
        None
    } else {
        input.parse::<IpAddr>().ok()
    };
    let query = Query {
        input: &input,
        address,
        keyword,
    };
    let config = fs::read_to_string(root.join("surge.conf")).context("reading surge.conf")?;
    let mut result = Lookup::default();
    let mut in_rules = false;
    for (index, line) in config.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed == "[Rule]" {
            in_rules = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_rules = false;
        }
        if !in_rules || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with("RULE-SET,") || trimmed.starts_with("DOMAIN-SET,") {
            let target = trimmed.split(',').nth(1).unwrap_or_default();
            let policy = trimmed.split(',').nth(2).unwrap_or_default().trim();
            if let Some(path) = local_set_path(root, trimmed) {
                scan_set(
                    root,
                    &path,
                    trimmed.starts_with("DOMAIN-SET,"),
                    &query,
                    policy,
                    &mut result,
                )?;
            } else if target == "LAN" && address.is_some() {
                // Surge's LAN set is built in, and may vary between Surge versions.
                result
                    .unknown
                    .push(format!("surge.conf:{} RULE-SET,LAN (built-in)", index + 1));
            } else if target != "LAN" {
                result.unknown.push(format!(
                    "surge.conf:{} {} (not available locally)",
                    index + 1,
                    trimmed
                ));
            }
        } else if matches_rule(trimmed, &input, address, keyword) {
            let policy = trimmed.split(',').nth(2).unwrap_or_default().trim();
            result.matches.push(Match {
                file: "surge.conf".to_string(),
                line: index + 1,
                rule: trimmed.to_string(),
                policy: policy.to_string(),
            });
        } else if !keyword && (trimmed.starts_with("GEOIP,") || trimmed.starts_with("IP-ASN,")) {
            result.unknown.push(format!(
                "surge.conf:{} {} (requires Surge database)",
                index + 1,
                trimmed
            ));
        }
    }
    if result.matches.is_empty() && !keyword {
        result.unknown.push(
            "No local rule matched; Surge may use FINAL after built-in, remote and database rules"
                .to_string(),
        );
    }
    if !keyword && address.is_none() {
        result
            .unknown
            .push("DNS-dependent IP rules are not evaluated for hostnames".to_string());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_domain_keyword_suffix_and_ip_families() {
        assert!(matches_rule(
            "DOMAIN,example.com",
            "example.com",
            None,
            false
        ));
        assert!(!matches_rule(
            "DOMAIN,example.com",
            "a.example.com",
            None,
            false
        ));
        assert!(matches_rule(
            "DOMAIN-SUFFIX,example.com",
            "a.example.com",
            None,
            false
        ));
        assert!(!matches_rule(
            "DOMAIN-SUFFIX,example.com",
            "badexample.com",
            None,
            false
        ));
        assert!(matches_rule(
            "DOMAIN-KEYWORD,ample",
            "a.example.com",
            None,
            false
        ));
        assert!(matches_rule(
            "DOMAIN-KEYWORD,example",
            "example",
            None,
            true
        ));
        assert!(matches_rule(
            "IP-CIDR,10.0.0.0/8,no-resolve",
            "10.2.3.4",
            "10.2.3.4".parse().ok(),
            false
        ));
        assert!(matches_rule(
            "IP-CIDR6,2607:6bc0::/32,no-resolve",
            "2607:6bc0::1",
            "2607:6bc0::1".parse().ok(),
            false
        ));
        assert!(!matches_rule(
            "IP-CIDR,10.0.0.0/8",
            "11.0.0.1",
            "11.0.0.1".parse().ok(),
            false
        ));
    }
}
