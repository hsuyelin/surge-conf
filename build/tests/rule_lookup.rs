use std::fs;

use surge_sync::{lookup::lookup, rules::normalize_rules};

#[test]
fn local_profile_matches_in_order_and_normalizes_domain_sets() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!(
        "surge-lookup-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let rules = root.join("rules/ai");
    fs::create_dir_all(&rules)?;
    let outcome = (|| -> anyhow::Result<()> {
        fs::write(root.join("surge.conf"), "[Rule]\nDOMAIN-KEYWORD,first,US\nDOMAIN-SET,rules/ai/domains.conf,AI\nRULE-SET,rules/ai/mixed.conf,Proxy\nFINAL,Final\n")?;
        fs::write(rules.join("ai.conf"), "DOMAIN-SUFFIX,unrelated.ai\n")?;
        fs::write(
            rules.join("domains.conf"),
            "# Domain set\n.first.example.com\n.example.com\nwww.example.com\nnotexample.com\n",
        )?;
        fs::write(rules.join("mixed.conf"), "DOMAIN-KEYWORD,example\nIP-CIDR,10.0.0.0/8,no-resolve\nIP-CIDR6,2001:db8::/32,no-resolve\n")?;
        assert_eq!(normalize_rules(&root)?, 1);
        assert_eq!(
            fs::read_to_string(rules.join("domains.conf"))?,
            "# Domain set\n.example.com\nnotexample.com\n"
        );
        let domain = lookup(&root, "WWW.Example.COM.", false)?;
        assert_eq!(domain.matches.len(), 2);
        assert_eq!(domain.matches[0].file, "rules/ai/domains.conf");
        assert_eq!(domain.matches[0].policy, "AI");
        assert_eq!(domain.matches[1].file, "rules/ai/mixed.conf");
        let keyword = lookup(&root, "example", true)?;
        assert_eq!(keyword.matches.len(), 3);
        assert_eq!(keyword.matches[0].file, "rules/ai/domains.conf");
        let ipv4 = lookup(&root, "10.2.3.4", false)?;
        assert_eq!(ipv4.matches[0].rule, "IP-CIDR,10.0.0.0/8,no-resolve");
        let ipv6 = lookup(&root, "2001:db8::1", false)?;
        assert_eq!(ipv6.matches[0].rule, "IP-CIDR6,2001:db8::/32,no-resolve");
        assert_eq!(normalize_rules(&root)?, 0);
        Ok(())
    })();
    fs::remove_dir_all(&root)?;
    outcome
}

#[test]
fn configured_overrides_keep_existing_policies() -> anyhow::Result<()> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow::anyhow!("build has no parent"))?;
    for (host, expected) in [
        ("wonderegg.de", "Emby"),
        ("push.pilix.app", "DIRECT"),
        ("meta.ai", "AI"),
        ("netflix.com", "Streaming"),
        ("www.biliblili.uk", "HK"),
        ("typesafe.ai", "US"),
        ("209.33.172.62", "DIRECT"),
    ] {
        let routing = lookup(root, host, false)?;
        assert_eq!(
            routing.matches.first().map(|entry| entry.policy.as_str()),
            Some(expected),
            "{host}"
        );
    }
    Ok(())
}
