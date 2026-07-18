# ReTheme Theme Development Guide

A ReTheme theme is a declarative package containing `manifest.json`, CSS, and images. It cannot execute JavaScript and must not target ChatGPT's internal class names or selectors. The runtime maps stable slots to the active ChatGPT version.

## Structure

```text
my-theme/
├── manifest.json
├── styles/
│   ├── tokens.css
│   └── overrides.css
└── assets/
    └── hero.svg
```

Select this directory for local development. A community submission is a ZIP whose root directly contains `manifest.json`.

## Manifest

See the runnable [`docs/theme-example/manifest.json`](theme-example/manifest.json). Important fields:

| Field | Rule |
|:--|:--|
| `schemaVersion` | Must be `1` |
| `id` | Globally unique reverse-domain identifier; immutable after publication |
| `version` | Valid SemVer; a published version cannot be overwritten |
| `styles` | At least one package-relative CSS path |
| `slots` | Stable logical slots used by the CSS |
| `permissions` | Must currently be `[]` |
| `preview` | Three hexadecimal colors used by the store and desktop client |
| `experience` | Home hero, compact conversation banner, prompt copy, and controlled assets |
| `locales` | Optional localizations; provide a complete `en` entry when the base fields are Chinese |

`testedCodexVersions` records versions you actually tested. It does not lock runtime compatibility; signed platform compatibility data selects the correct mapping for each ChatGPT version.

## CSS Scope

Every selector must live under the theme root:

```css
:root[data-ct-theme="studio.example.protocol-preview"] [data-ct-slot="sidebar"] {
  background: var(--rt-sidebar-background) !important;
}
```

Do not depend on minified classes, DOM depth, or localized ChatGPT text. Do not override fonts. Put colors, radii, borders, shadows, and spacing in `tokens.css`, then write slot rules in `overrides.css`.

Use the runtime color scheme for dark variants:

```css
:root[data-ct-theme="studio.example.protocol-preview"][data-ct-color-scheme="dark"] {
  --rt-page-background: #101521;
}
```

## Common Slots

| Area | Common slots |
|:--|:--|
| App | `app.shell`, `app.background`, `titlebar` |
| Sidebar | `sidebar`, `sidebar.header`, `sidebar.item`, `sidebar.item.active`, `sidebar.footer` |
| Main | `main`, `main.background`, `page`, `page.surface`, `page.header` |
| Home | `home.hero`, `home.prompt.title`, `home.cards`, `home.card`, `home.card.background`, `home.card.label` |
| Conversation | `conversation.header`, `conversation.banner`, `conversation`, `conversation.user`, `conversation.assistant` |
| Composer | `composer`, `composer.context`, `composer.editor`, `composer.action`, `composer.submit` |
| Menus | `menu`, `menu.item`, `menu.item.active`, `menu.separator` |
| Settings | `settings`, `settings.canvas`, `settings.card`, `settings.row`, `settings.switch.track.checked` |

Declare only the slots you use. The authoritative allowlist is `ALLOWED_SLOTS` in `src-tauri/src/theme.rs`; a theme cannot create arbitrary `data-ct-slot` values.

## Banners and Assets

Declare a full home hero and a compact conversation banner separately. `asset` is the background and `foreground` is an optional transparent character or object layer. The engine owns stable mounting and layout.

PNG, JPEG, WebP, and sanitized static SVG are supported. An image may be up to 8 MiB; an archive up to 30 MiB; extracted contents up to 60 MiB and 256 files. Assets are served by ReTheme's loopback-only local asset service, so Base64 is unnecessary. External URLs, `@import`, and remote fonts are rejected.

Animate only `transform` and `opacity`, and support reduced motion:

```css
@media (prefers-reduced-motion: reduce) {
  :root[data-ct-theme="studio.example.protocol-preview"] * {
    animation: none !important;
  }
}
```

## Localization

Use the base manifest fields for Chinese and `locales.en` for the English name, description, home copy, prompt, and conversation banner. ReTheme synchronizes its interface locale into the running theme; never infer language from ChatGPT text.

## Develop and Publish

1. Copy [`docs/theme-example`](theme-example) and change its `id`.
2. Choose “Load local theme” in ReTheme. A local directory needs no signature, encryption, or `.ctheme` file.
3. Test Chinese/English, light/dark, home/conversation/settings, and different window sizes.
4. Remove unused assets, bump SemVer, and ZIP the source directory.
5. Submit the ZIP to the ReTheme community. Do not add `access`, `integrity`, or `signature` yourself.
6. After review, the platform creates the signed `.ctheme`; regular users install only through the online flow.

Package signing and encryption belong to the platform. Platform private keys and version-specific ChatGPT selectors never belong in theme source.
