<div align="center">

# Surge 配置

![Top Language](https://img.shields.io/github/languages/top/hsuyelin/surge-conf?style=flat-square)
![CI](https://img.shields.io/github/actions/workflow/status/hsuyelin/surge-conf/ci.yml?style=flat-square&label=CI)
![License](https://img.shields.io/github/license/hsuyelin/surge-conf?style=flat-square)

**[English](README.md) | 中文**

</div>

---

自动同步上游图标和规则集的 Surge 配置仓库。

## 📁 目录结构

```
surge-conf/
├── icons/              # 策略组图标
│   ├── apps/           # 应用图标
│   ├── country/        # 国家/地区图标
│   ├── policy/         # 策略图标
│   └── private/        # 自定义图标
├── rules/              # 规则集
│   ├── adblock/        # 广告拦截
│   ├── ai/             # AI 服务
│   ├── apple/          # 苹果服务
│   ├── media/          # 流媒体
│   ├── social/         # 社交通讯
│   ├── gaming/         # 游戏平台
│   ├── proxy/          # 代理规则
│   └── private/        # 私有规则
├── modules/            # Surge 模块
│   ├── enhance/        # 增强模块
│   ├── adblock/        # 去广告模块
│   ├── utility/        # 实用工具模块
│   └── subtitle/       # 字幕模块
├── build/              # Rust 同步工具
├── surge.conf          # 模板配置
└── sync.sh             # 手动同步脚本
```

## 🚀 使用方法

### 方式一：Fork 自动同步

1. Fork 本仓库
2. GitHub Actions 将在每天北京时间 08:00 自动同步
3. 在 Surge 中使用 `surge.conf` 的 raw URL

### 方式二：Clone 手动同步

```bash
git clone https://github.com/hsuyelin/surge-conf.git
cd surge-conf
./sync.sh
```

需要安装 [Rust](https://rustup.rs/)

## ⚙️ 配置说明

编辑 `surge.conf`，替换以下内容：
- 将 `<YOUR_SUBSCRIPTION_URL>` 替换为你的代理订阅链接
- 在 `[Proxy]` 区块添加你的代理
- 在 `[MITM]` 区块添加证书

规则同步会按 `surge.conf` 中 `[Rule]` 的顺序去重本地规则集；`DOMAIN-KEYWORD`、`DOMAIN-SUFFIX`、`DOMAIN` 及 IP 网段的包含关系也会处理。较早的具体规则与较晚的宽泛规则会同时保留，以免改变其他域名的匹配结果。AI 自定义规则只放在 `rules/ai/ai.conf` 的 `BEGIN/END LOCAL AI RULES` 区间，刷新上游时会保留该区间；不再使用 `private-ai.conf`。仅执行本地去重（不下载）：`cd build && cargo run --bin sync_rules -- --normalize-only`。

地区和直连定制规则保存在 `rules/private/` 并通过 `surge.conf` 的 `RULE-SET` 引用；已存在的 Emby、AI、Streaming、DIRECT 策略优先，因此未用 `co.uk` 等宽泛规则覆盖现有服务。`wonderegg.de` 保持 Emby，未创建空 TW 规则集。GitHub Actions 的 `sync_rules` 在下载上游规则后执行同一套去重，再提交最终规则文件。

`modules/utility/appleCertBlock.sgmodule` 会拦截证书状态检查端点，并通过 `[MITM] hostname = %APPEND%` 添加主机名；启用后会削弱证书吊销状态检查，仅在了解风险时使用。

## 查询分流规则

```bash
cd build
cargo run --bin check_rule -- chatgpt.com       # 域名；也支持 IPv4 / IPv6
cargo run --bin check_rule -- --keyword cursor  # 搜索规则中包含该关键词的域名
cargo run --bin check_rule                     # 逐行交互输入域名或 IP
```

按 `surge.conf` 加载顺序列出所有命中的本地规则、所在 conf、行号及策略。支持 DOMAIN、DOMAIN-SUFFIX、DOMAIN-KEYWORD、DOMAIN-SET（纯域名列表）与 IP-CIDR/IP-CIDR6。关键词模式搜索规则内容，不等于实际流量匹配。只查询本地文件；内置 SYSTEM/LAN、外部 URL、GeoIP/ASN 数据库及域名的 DNS→IP 匹配会注明无法离线判断，因此“首个本地命中”不一定是 Surge 最终策略。
