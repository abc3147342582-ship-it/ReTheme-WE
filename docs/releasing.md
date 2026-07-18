# Release Guide

ReTheme releases are built by `.github/workflows/release.yml` and published to `duxweb/ReTheme` GitHub Releases.

## Required repository secrets

- `RETHEME_API_SECRET_ID`
- `RETHEME_API_SECRET_KEY`
- `RETHEME_PACKAGE_KEY`
- `RETHEME_LICENSE_PUBLIC_KEY`
- `RETHEME_COMPATIBILITY_PUBLIC_KEY`
- `RETHEME_THEME_PUBLIC_KEY`
- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

Do not paste these values into source files, workflow logs, release notes, or issues. The private source repository is the authoritative encrypted/offline backup location.

## Preflight

```bash
pnpm install --frozen-lockfile
pnpm test
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Confirm that `src-tauri/tauri.conf.json`, `package.json`, and `src-tauri/Cargo.toml` use the same version. The release workflow generates `api.toml` and `security.toml` from Actions Secrets, then removes them in an `always()` cleanup step.

## Publish

```bash
git tag v0.1.0
git push origin main
git push origin v0.1.0
```

The matrix builds:

- macOS Apple Silicon on `macos-15` for `aarch64-apple-darwin` (`app` updater archive and DMG)
- macOS Intel on `macos-15-intel` for `x86_64-apple-darwin` (`app` updater archive and DMG)
- Windows x64 on `windows-latest` for `x86_64-pc-windows-msvc`

The Windows NSIS installer offers Simplified Chinese and English. Matrix builds update one draft release serially, then the publish job verifies all updater platform entries before marking it as the latest public release.

## Verify

1. All three matrix jobs pass.
2. The release contains both macOS DMGs, the Windows NSIS installer, updater archives/signatures, and `latest.json`.
3. `latest.json` points to the same release and contains all supported platforms.
4. Install each native artifact and verify launch, language, tray, deep links, theme apply/restore, and update checks.
5. Keep the tag immutable. Publish a new SemVer tag for every correction.

The ignored Rust integration tests require ChatGPT to be installed and may launch isolated windows. Run them explicitly on maintained macOS and Windows test machines before a production rollout.
