<div align="center">

# Surge Configuration

![Top Language](https://img.shields.io/github/languages/top/hsuyelin/surge-conf?style=flat-square)
![CI](https://img.shields.io/github/actions/workflow/status/hsuyelin/surge-conf/ci.yml?style=flat-square&label=CI)
![License](https://img.shields.io/github/license/hsuyelin/surge-conf?style=flat-square)

**English | [中文](README_CN.md)**

</div>

---

Auto-syncing Surge configuration with icons and rule sets from upstream sources.

## 📁 Directory Structure

```
surge-conf/
├── icons/              # Policy group icons
│   ├── apps/           # Application icons
│   ├── country/        # Country/region icons
│   ├── policy/         # Policy icons
│   └── private/        # Custom icons
├── rules/              # Rule sets
│   ├── adblock/        # Ad blocking
│   ├── ai/             # AI services
│   ├── apple/          # Apple services
│   ├── media/          # Streaming media
│   ├── social/         # Social & messaging
│   ├── gaming/         # Gaming platforms
│   ├── proxy/          # Proxy rules
│   └── private/        # Private rules
├── modules/            # Surge modules
│   ├── enhance/        # Enhancement modules
│   ├── adblock/        # Ad blocking modules
│   ├── utility/        # Utility modules
│   └── subtitle/       # Subtitle modules
├── build/              # Rust sync tools
├── surge.conf          # Template configuration
└── sync.sh             # Manual sync script
```

## 🚀 Usage

### Option 1: Fork and Auto-Sync

1. Fork this repository
2. GitHub Actions will auto-sync daily at 08:00 (UTC+8)
3. Use the raw URL of `surge.conf` in Surge

### Option 2: Clone and Manual Sync

```bash
git clone https://github.com/hsuyelin/surge-conf.git
cd surge-conf
./sync.sh
```

Requirements: [Rust](https://rustup.rs/)

## ⚙️ Configuration

Edit `surge.conf` and replace:
- `<YOUR_SUBSCRIPTION_URL>` with your proxy subscription link
- Add your proxies in `[Proxy]` section
- Add MITM certificate in `[MITM]` section

Rule sync deduplicates local rule sets in `surge.conf` `[Rule]` order, including `DOMAIN-KEYWORD` / `DOMAIN-SUFFIX` / `DOMAIN` and IP network containment. Earlier specific rules and later broad rules both remain when removing either would change routing. Keep custom AI entries only inside `BEGIN/END LOCAL AI RULES` in `rules/ai/ai.conf`; upstream refresh preserves that section. `private-ai.conf` is no longer used. To normalize local rules without downloading: `cd build && cargo run --bin sync_rules -- --normalize-only`.

Regional and direct overrides live under `rules/private/` and are loaded with `RULE-SET` entries. Existing Emby, AI, Streaming, and DIRECT decisions take precedence; broad `co.uk` would shadow service rules and is omitted. `wonderegg.de` remains Emby, so no empty TW rule set is created. GitHub Actions runs `sync_rules` after checkout: it downloads upstream sets, normalizes the complete local set, then commits the resulting files.

`modules/utility/appleCertBlock.sgmodule` blocks certificate status-check endpoints and appends hosts with `[MITM] hostname = %APPEND%`. Enabling it weakens certificate revocation checks; use only when the risk is understood.

## Inspect local routing rules

```bash
cd build
cargo run --bin check_rule -- chatgpt.com       # Hostname, IPv4 or IPv6
cargo run --bin check_rule -- --keyword cursor  # Search domain rule operands
cargo run --bin check_rule                     # Interactive input, one per line
```

The tool reports matching local rules in `surge.conf` order with their conf, line number and policy. Supported matchers: DOMAIN, DOMAIN-SUFFIX, DOMAIN-KEYWORD, DOMAIN-SET (plain domain lists), IP-CIDR and IP-CIDR6. Keyword search lists rules containing the term; it is not a traffic match. Built-in SYSTEM/LAN sets, external URLs, GeoIP/ASN and DNS-dependent IP rules for hostnames cannot be resolved offline. The first local match may therefore differ from Surge's effective route.
