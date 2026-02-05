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
