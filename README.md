<p align="center">
  <img src="src-tauri/icons/icon.png" width="112" height="112" alt="ReTheme">
</p>

<h1 align="center">ReTheme</h1>

<p align="center">
  为 ChatGPT 桌面端打造的安全、可更新、社区驱动主题引擎。<br>
  Tauri 2 + Rust · macOS 与 Windows · 中文与 English
</p>

<p align="center">
  <a href="https://retheme.app">官网</a> ·
  <a href="https://github.com/duxweb/ReTheme/releases">下载</a> ·
  <a href="docs/theme-development.md">主题开发</a> ·
  <a href="https://github.com/duxweb/ReTheme/issues">反馈</a>
</p>

<p align="center">
  简体中文 | <a href="README.en.md">English</a>
</p>

---

ReTheme 把主题作者面对的稳定逻辑插槽与 ChatGPT 的版本适配分开。主题只声明颜色、样式、图片和双语文案；匹配规则由签名兼容数据独立更新，因此 ChatGPT 升级时通常无需重新发布主题或桌面应用。

## 功能

| 能力 | 说明 |
|:--|:--|
| 主题运行时 | 覆盖首页、会话、输入框、侧栏、菜单、设置、卡片和装饰插槽 |
| 社区主题 | 通过 `retheme://` 深链下载，经平台签名验证后安装 |
| 本地开发 | 直接加载未打包主题目录，方便创作者实时调试 |
| 安全恢复 | 停止主题、退出 ReTheme 或安装更新前恢复 ChatGPT 官方界面 |
| 远程兼容 | 按 ChatGPT 版本获取签名适配数据，支持新旧版本并存 |
| 桌面体验 | 托盘、关闭隐藏、单实例、双语界面、深浅色与自动更新 |

ReTheme 桌面仓库不内置社区主题。主题在社区发布并按需下载，安装包保持精简。

## 下载

前往 [GitHub Releases](https://github.com/duxweb/ReTheme/releases) 下载：

- macOS Apple Silicon (`aarch64`)
- macOS Intel (`x86_64`)
- Windows x64 双语安装器（简体中文 / English）

首次启动后，ReTheme 会检测已安装的 ChatGPT。关闭主窗口会隐藏到托盘；请使用托盘菜单中的“退出 ReTheme”彻底退出。

## 主题开发

从 [主题开发规范](docs/theme-development.md) 和 [最小参考主题](docs/theme-example) 开始。开发主题是普通目录，不需要 `.ctheme` 解密或平台签名：

```text
theme-example/
├── manifest.json
├── styles/
└── assets/
```

在 ReTheme 的“主题库”中选择“加载本地主题”，选中包含 `manifest.json` 的目录即可。正式发布时上传源码 ZIP，由平台审核、规范化、签名并生成 `.ctheme`。

## 本地开发

需要 Node.js 22、pnpm 10、Rust stable 与 Tauri 对应的系统依赖。

```bash
pnpm install
cp src-tauri/config/api.example.toml src-tauri/config/api.toml
cp src-tauri/config/security.example.toml src-tauri/config/security.toml
pnpm tauri dev
```

验证：

```bash
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

配置模板不包含生产密钥。维护者发版请阅读 [发布流程](docs/releasing.md)，安全边界见 [安全设计](docs/security.md)。

## 相关链接

- [主题开发规范](docs/theme-development.md)
- [Theme Development Guide](docs/theme-development.en.md)
- [安全设计](docs/security.md)
- [发版流程](docs/releasing.md)
- [ReTheme 官网](https://retheme.app)
- [问题反馈](https://github.com/duxweb/ReTheme/issues)
