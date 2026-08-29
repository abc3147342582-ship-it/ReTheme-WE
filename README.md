# ReTheme WE

ReTheme WE 是基于 [duxweb/ReTheme](https://github.com/duxweb/ReTheme) 的非官方 Windows 分支，用于把本机 Wallpaper Engine Workshop 壁纸应用到 ChatGPT Windows 客户端。

本项目与 ReTheme 官方项目、OpenAI、Valve 或 Wallpaper Engine 均无隶属或背书关系。原项目版权和 BSD 3-Clause 许可证见 [LICENSE](LICENSE)。

## 主要功能

- 扫描本机 Wallpaper Engine Workshop 目录并按类型分类。
- Video 壁纸由浏览器原生视频能力直接播放，不受主题图片 8 MB 限制。
- 受限 Web 壁纸运行在隔离沙箱中并禁止联网、弹窗、表单和下载。
- Scene 壁纸由本机已安装的 Wallpaper Engine 原版渲染，并同步到桌面与 ChatGPT 背景。
- 支持壁纸亮度、界面透明度、播放/暂停和上次选择恢复。
- 针对新版与旧版 ChatGPT 页面清理背景黑框、输入框阴影和不透明容器。
- 支持指定 GitHub 仓库进行版本兼容自检和签名自动更新。

Scene 壁纸会先同步为 Wallpaper Engine 的桌面壁纸，再通过 Windows Graphics Capture 抓取同一项目的纯净离屏画面并传入 ChatGPT。0.4.8 保留 2561×1601 原始尺寸和 JPEG 80，本机原版 Scene 实测 15.0 FPS；Video 与 Web 壁纸不受此 Scene 抓帧限制。为避免同一壁纸同时渲染两次，建议在 Wallpaper Engine 中配置 `ChatGPT.exe`「最大化时暂停」应用规则。

## 系统要求

- Windows 10/11 x64。
- Microsoft Store 版 ChatGPT Windows 客户端。
- 导入 Scene 壁纸时需要已安装并拥有 Wallpaper Engine；Video/Web 壁纸无需保持 Wallpaper Engine 运行。
- 默认 Workshop 路径：`C:\Program Files (x86)\Steam\steamapps\workshop\content\431960`。

## 下载与安装

从本仓库的 [Releases](https://github.com/abc3147342582-ship-it/ReTheme-WE/releases) 下载 Windows x64 安装包。

安装模式为当前用户，不需要写入 WindowsApps。若安装包的 Windows Authenticode 状态显示为 `NotSigned`，请结合 Release 的 SHA-256 和 Tauri 更新签名核验来源；不要从第三方下载站获取安装包。

## 本地构建

需要 Node.js 22、pnpm 10、Rust stable、Visual Studio Build Tools 2022 与 Windows SDK。

```powershell
pnpm install --frozen-lockfile
node scripts/write-retheme-we-build-config.mjs
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri build -- --bundles nsis
```

`src-tauri/config/api.toml`、`src-tauri/config/security.toml`、Tauri 更新私钥、`node_modules`、`dist` 和 Rust `target` 均被忽略，不应提交到仓库。

## 发布与安全

- Windows 发布流程见 [docs/releasing.md](docs/releasing.md)。
- 完整版本记录见 [CHANGELOG.zh-CN.md](CHANGELOG.zh-CN.md)。
- 安全边界见 [docs/security.md](docs/security.md)。
- 请勿将 Tauri 更新私钥、GitHub Token、账号凭据或本机日志提交到 Issue 或仓库。

## 上游与许可证

本分支保留上游 Git 历史和 BSD 3-Clause 许可证。上游项目：[duxweb/ReTheme](https://github.com/duxweb/ReTheme)。
