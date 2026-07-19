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

<p align="center">
  <img src="docs/images/retheme-light-dark.png" alt="ReTheme 深色与浅色界面预览">
</p>

---

ReTheme 把主题作者面对的稳定逻辑插槽与 ChatGPT 的版本适配分开。主题只声明颜色、样式、图片和双语文案；匹配规则由签名兼容数据独立更新，因此 ChatGPT 升级时通常无需重新发布主题或桌面应用。

## 主题预览

主题可以统一覆盖 ChatGPT 的背景、首页横幅、建议卡片、输入框、侧栏、菜单、图标、设置页与会话界面，同时保留原生交互和自适应布局。

### 齐天 · 暗色国风

<p align="center">
  <img src="docs/images/theme-qitian.png" alt="齐天暗色国风主题预览">
</p>

### 莓果 · 浅色手帐

<p align="center">
  <img src="docs/images/theme-berry.png" alt="莓果浅色手帐主题预览">
</p>

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

从 [主题开发规范](docs/theme-development.md) 和 [可直接加载的最小参考主题](docs/theme-example/package) 开始。开发主题是普通目录，不需要 `.ctheme` 解密或平台签名：

```text
theme-example/
├── manifest.json
├── styles/
└── assets/
```

在 ReTheme 的“主题库”中选择“加载本地主题”，选中包含 `manifest.json` 的目录即可。正式发布时上传源码 ZIP，由平台审核、规范化、签名并生成 `.ctheme`。

完整开发资料还包括 [164 个稳定插槽](docs/theme-slots.md)、[Banner 与图片规格/生图提示词](docs/theme-banner-assets.md)、[Manifest JSON Schema](docs/theme.schema.json)、[带注释 Manifest](docs/theme-example/manifest.annotated.jsonc) 与 [AI 确定性工作流](docs/theme-ai-workflow.md)。仓库内置的 [`retheme-theme-development` Skill](skills/retheme-theme-development/SKILL.md) 可供 Codex 等 AI 直接创建、检查和评审主题。

使用与桌面端、服务端相同的 Rust 协议校验器验证源码目录：

```bash
cargo run --manifest-path crates/theme-validator/Cargo.toml -- \
  --directory /absolute/path/to/theme
```

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
- [稳定插槽目录](docs/theme-slots.md)
- [Banner 与图片规范](docs/theme-banner-assets.md)
- [AI 主题开发工作流](docs/theme-ai-workflow.md)
- [主题开发 Skill](skills/retheme-theme-development/SKILL.md)
- [Theme Development Guide](docs/theme-development.en.md)
- [安全设计](docs/security.md)
- [发版流程](docs/releasing.md)
- [开源协议](LICENSE)
- [ReTheme 官网](https://retheme.app)
- [LINUX DO 社区](https://linux.do/)
- [问题反馈](https://github.com/duxweb/ReTheme/issues)
