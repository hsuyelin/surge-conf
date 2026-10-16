//! Inspect local Surge rule matches without network requests.
use std::io::{self, BufRead};

use anyhow::{Context, Result};
use surge_sync::{lookup::lookup, project_root};

fn show(query: &str, keyword: bool) -> Result<()> {
    let result = lookup(&project_root()?, query, keyword)?;
    println!("Input: {query}");
    if let Some(first) = result.matches.first() {
        println!(
            "First local match: {}:{} [{}] {}",
            first.file, first.line, first.policy, first.rule
        );
        for matched in result.matches.iter().skip(1) {
            println!(
                "Also matches: {}:{} [{}] {}",
                matched.file, matched.line, matched.policy, matched.rule
            );
        }
    } else {
        println!("No local match");
    }
    for item in &result.unknown {
        println!("Not evaluated: {item}");
    }
    Ok(())
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let first = args.next();
    let (keyword, query) = match first.as_deref() {
        Some("--keyword") => (true, args.next()),
        _ => (false, first),
    };
    if let Some(query) = query {
        anyhow::ensure!(
            args.next().is_none(),
            "usage: check_rule [--keyword] <domain|ip|keyword>"
        );
        return show(&query, keyword);
    }
    let stdin = io::stdin();
    println!("Enter a domain or IP per line (Ctrl-D to exit):");
    for line in stdin.lock().lines() {
        let line = line.context("reading input")?;
        if !line.trim().is_empty() {
            show(&line, keyword)?;
        }
    }
    Ok(())
}
