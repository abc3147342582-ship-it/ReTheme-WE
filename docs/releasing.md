# ReTheme WE Windows 发布指南

`.github/workflows/release.yml` 只构建 Windows x64 NSIS 安装包，并生成 Tauri 自动更新所需的签名与 `latest.json`。

## 首次配置

1. 在自己的 GitHub 账号或组织下创建 ReTheme WE 仓库并推送本分支。
2. 在仓库 `Settings > Secrets and variables > Actions` 新建 Secret：`TAURI_SIGNING_PRIVATE_KEY`。
3. Secret 的值取自本机 `%LOCALAPPDATA%\ReTheme WE Updater\retheme-we.key`。私钥不得提交、粘贴到 Issue 或写入日志。
4. 客户端“设置”页的“GitHub 更新仓库”填写同一个 `owner/repo`。

客户端内置独立的 ReTheme WE 公钥；即使仓库设置被改动，也只会安装该私钥签名的更新包。当前密钥没有密码，工作流把 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 留空。

## 发布

可以推送 SemVer 标签：

```text
git tag v0.4.9
git push origin main
git push origin v0.4.9
```

也可以从 GitHub Actions 手动运行 `Release ReTheme WE` 并填写版本号。工作流会同步所有版本文件、运行前端与 Rust 测试、构建签名 NSIS 安装包并发布公开 Release。
Windows 工作流只从 NASM 官方发布目录下载 3.02 Win64 压缩包，并在解压前校验固定 SHA-256；该工具用于 libjpeg-turbo 的 x64 SIMD 构建。

## 本机预检

项目根目录的 `.cargo/config.toml` 已将所有 Rust/Tauri 构建统一到
`src-tauri/target`。本机调试、测试与 Release 应复用这一个缓存目录，
不要再为单次测试设置新的 `CARGO_TARGET_DIR`，否则会重复保存数 GiB 的依赖和调试符号。

```text
pnpm install --frozen-lockfile
pnpm test
pnpm test:release
pnpm build
pnpm version:check
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
```

发布完成后确认 Release 含 Windows 安装包、对应 `.sig` 以及 `latest.json`，然后从已安装客户端执行一次“检查更新”。每次修复都发布新版本，不要覆盖已有标签。
