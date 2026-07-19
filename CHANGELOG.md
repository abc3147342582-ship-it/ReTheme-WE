# Changelog

All notable ReTheme desktop changes are documented here.

## [0.1.1] - 2026-07-19

### Fixed

- Recover installed themes after reinstalling ReTheme on Windows when an old device-encrypted cache can no longer be opened.
- Show a clear result after manually checking for updates, including when ReTheme is already up to date.
- Build tagged releases with the tag version across application metadata and updater manifests.

## [0.1.0] - 2026-07-19

### Added

- Manage installed themes, community downloads, local theme development, account state, cloud favorites, and device history.
- Apply signed themes through the local Rust runtime with remote ChatGPT compatibility data.
- Support Simplified Chinese and English, tray behavior, deep links, light and dark appearance, and signed automatic updates.
- Ship signed installers for macOS Apple Silicon, macOS Intel, and Windows x64.

### Security

- Verify theme packages and compatibility data before use and serve theme assets only through a bounded loopback session.
- Restore the official ChatGPT interface before ReTheme exits or installs an application update.
