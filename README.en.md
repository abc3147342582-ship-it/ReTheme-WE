# ReTheme WE

ReTheme WE is an unofficial Windows fork of [duxweb/ReTheme](https://github.com/duxweb/ReTheme) that applies local Wallpaper Engine Workshop wallpapers to the ChatGPT Windows app.

This project is not affiliated with or endorsed by the upstream ReTheme project, OpenAI, Valve, or Wallpaper Engine. The upstream copyright notice and BSD 3-Clause license are retained in [LICENSE](LICENSE).

## Highlights

- Scans the local Wallpaper Engine Workshop library and groups runnable projects by engine requirement.
- Streams Video wallpapers with the browser's native video pipeline without the former 8 MB theme-image limit.
- Runs supported Web wallpapers in a restricted offline sandbox.
- Renders Scene wallpapers with the locally installed Wallpaper Engine and synchronizes the selected project with the desktop.
- Persists brightness, interface transparency, playback state, and the last selected wallpaper.
- Removes opaque frames and composer shadows across current and older ChatGPT views.
- Supports GitHub-based compatibility checks and signed application updates.

Scene wallpapers currently use Windows Graphics Capture followed by a JPEG frame bridge into ChatGPT. Version 0.4.7 intentionally targets about 10 FPS to bound CPU, GPU, and transport cost. Video and Web wallpapers do not use this Scene capture limit.

## Requirements

- Windows 10/11 x64.
- The Microsoft Store ChatGPT Windows app.
- A locally installed licensed copy of Wallpaper Engine for original Scene projects. Video and supported Web projects do not need Wallpaper Engine to remain running.
- Default Workshop path: `C:\Program Files (x86)\Steam\steamapps\workshop\content\431960`.

## Download

Download the Windows x64 installer from this repository's [Releases](https://github.com/abc3147342582-ship-it/ReTheme-WE/releases) page.

The installer uses current-user mode and does not write to WindowsApps. A locally built installer may report Windows Authenticode as `NotSigned`; verify the Release SHA-256 and Tauri updater signature, and do not use third-party download mirrors.

## Local build

Install Node.js 22, pnpm 10, Rust stable, Visual Studio Build Tools 2022, and the Windows SDK.

```powershell
pnpm install --frozen-lockfile
node scripts/write-retheme-we-build-config.mjs
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri build -- --bundles nsis
```

`src-tauri/config/api.toml`, `src-tauri/config/security.toml`, the Tauri updater private key, `node_modules`, `dist`, and Rust `target` directories are ignored and must not be committed.

## Release and security

- Windows release instructions: [docs/releasing.md](docs/releasing.md)
- Full change history: [CHANGELOG.md](CHANGELOG.md)
- Security boundaries: [docs/security.md](docs/security.md)
- Never publish the Tauri updater private key, GitHub tokens, account credentials, or local logs.

## Upstream and license

This fork retains the upstream Git history and BSD 3-Clause license. Upstream project: [duxweb/ReTheme](https://github.com/duxweb/ReTheme).
