<p align="center">
  <img src="src-tauri/icons/icon.png" width="112" height="112" alt="ReTheme">
</p>

<h1 align="center">ReTheme</h1>

<p align="center">
  A safe, updateable, community-driven theme engine for the ChatGPT desktop app.<br>
  Tauri 2 + Rust · macOS and Windows · English and 简体中文
</p>

<p align="center">
  <a href="https://retheme.app">Website</a> ·
  <a href="https://github.com/duxweb/ReTheme/releases">Download</a> ·
  <a href="docs/theme-development.en.md">Theme Development</a> ·
  <a href="https://github.com/duxweb/ReTheme/issues">Feedback</a>
</p>

<p align="center">
  <a href="README.md">简体中文</a> | English
</p>

<p align="center">
  <img src="docs/images/retheme-light-dark.png" alt="ReTheme dark and light interface preview">
</p>

---

ReTheme separates the stable logical slots used by theme authors from version-specific ChatGPT adaptation. A theme declares colors, CSS, images, and localized copy, while signed compatibility data can update independently when ChatGPT changes.

## Theme Preview

A theme can consistently cover ChatGPT backgrounds, home banners, suggestion cards, the composer, sidebar, menus, icons, settings, and conversations while preserving native interactions and responsive layout.

### Qitian · Dark Mythic Style

<p align="center">
  <img src="docs/images/theme-qitian.png" alt="Qitian dark mythic theme preview">
</p>

### Berry · Light Journal Style

<p align="center">
  <img src="docs/images/theme-berry.png" alt="Berry light journal theme preview">
</p>

## Highlights

| Capability | Description |
|:--|:--|
| Theme runtime | Covers home, conversations, composer, sidebar, menus, settings, cards, and decoration slots |
| Community themes | Downloads through `retheme://` deep links and installs only after platform signature verification |
| Local development | Loads an unpacked theme directory directly for fast authoring and iteration |
| Safe restore | Restores the official ChatGPT interface when a theme stops, ReTheme exits, or an update installs |
| Remote compatibility | Selects signed adaptation data by ChatGPT version so old and new releases can coexist |
| Desktop experience | Tray behavior, single instance, bilingual UI, light/dark appearance, and signed auto-updates |

No community theme ships inside this repository or the desktop installer. Themes are published to the community and downloaded on demand.

## Download

Get the latest build from [GitHub Releases](https://github.com/duxweb/ReTheme/releases):

- macOS Apple Silicon (`aarch64`)
- macOS Intel (`x86_64`)
- Windows x64 bilingual installer (English / Simplified Chinese)

ReTheme detects the installed ChatGPT app on first launch. Closing the main window hides it to the tray; use “Quit ReTheme” from the tray menu to terminate it completely.

## Build a Theme

A development theme is an unpacked directory and does not need `.ctheme` decryption or a platform signature. Read the authoring kit in this order:

1. [Theme Development Guide](docs/theme-development.en.md): package structure, Manifest, CSS, security boundaries, localization, and QA.
2. [Loadable minimal example](docs/theme-example/package) and [annotated Manifest](docs/theme-example/manifest.annotated.jsonc): start from working code.
3. [164 stable slots](docs/theme-slots.md) and [Manifest JSON Schema](docs/theme.schema.json): find supported surfaces and field constraints.
4. [Banner and image specifications with generation prompts](docs/theme-banner-assets.md): create the home Hero, compact conversation Banner, and transparent foreground assets.
5. [Deterministic AI workflow](docs/theme-ai-workflow.md): make an AI agent create, validate, and review a theme consistently.

### Manual workflow

Copy the minimal theme, then edit `manifest.json`, `styles/`, and `assets/`:

```bash
cp -R docs/theme-example/package /absolute/path/to/my-theme
```

```text
my-theme/
├── manifest.json
├── styles/
└── assets/
```

Validate a source directory with the same Rust protocol checker used by the desktop app and server:

```bash
cargo run --manifest-path crates/theme-validator/Cargo.toml -- \
  --directory /absolute/path/to/theme
```

Validate the final ZIP as well. Its root must directly contain `manifest.json`:

```bash
cargo run --manifest-path crates/theme-validator/Cargo.toml -- \
  --source /absolute/path/to/theme.zip
```

Choose “Load local theme” in ReTheme and select the theme directory for real-app testing. For community publication, upload the source ZIP; the platform reviews, normalizes, signs, and produces the downloadable `.ctheme` package.

### Use the AI Skill

The bundled [`retheme-theme-development`](skills/retheme-theme-development/SKILL.md) Skill contains the protocol, slot catalog, QA matrix, Banner generation rules, validation script, and starter template. It helps Codex and other Skill-capable agents create themes, complete existing themes, or determine whether a defect belongs to the theme or engine.

Ask Codex to install the Skill directly from GitHub:

```text
Install the Skill at skills/retheme-theme-development from the duxweb/ReTheme GitHub repository.
```

Or install it from a local checkout. macOS / Linux:

```bash
mkdir -p "${CODEX_HOME:-$HOME/.codex}/skills"
cp -R skills/retheme-theme-development "${CODEX_HOME:-$HOME/.codex}/skills/"
```

Windows PowerShell:

```powershell
$codexHome = if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $HOME ".codex" }
New-Item -ItemType Directory -Force (Join-Path $codexHome "skills") | Out-Null
Copy-Item -Recurse -Force skills/retheme-theme-development (Join-Path $codexHome "skills/retheme-theme-development")
```

Start a new Codex session after installation and describe the task directly:

```text
Use the retheme-theme-development Skill to create a bilingual light/dark theme in /path/to/my-theme. Validate it, but do not package it as .ctheme.
```

The AI must treat the shared validator as authoritative. It must not weaken the protocol for a theme, target private ChatGPT class names, or change native layout behavior.

## Development

Install Node.js 22, pnpm 10, Rust stable, and the platform requirements for Tauri.

```bash
pnpm install
cp src-tauri/config/api.example.toml src-tauri/config/api.toml
cp src-tauri/config/security.example.toml src-tauri/config/security.toml
pnpm tauri dev
```

Run the checks:

```bash
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Configuration templates contain no production secrets. Maintainers should read the [release guide](docs/releasing.md) and [security model](docs/security.md).

## Links

- [Theme Development Guide](docs/theme-development.en.md)
- [Stable Slot Catalog](docs/theme-slots.md)
- [Banner and Image Specifications](docs/theme-banner-assets.md)
- [AI Theme Workflow](docs/theme-ai-workflow.md)
- [Theme Development Skill](skills/retheme-theme-development/SKILL.md)
- [中文主题开发规范](docs/theme-development.md)
- [Security Model](docs/security.md)
- [Release Guide](docs/releasing.md)
- [License](LICENSE)
- [ReTheme Website](https://retheme.app)
- [Issue Tracker](https://github.com/duxweb/ReTheme/issues)
