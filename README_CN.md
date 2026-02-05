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
