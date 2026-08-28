# Changelog

All notable ReTheme desktop changes are documented here.

## [0.4.7] - 2026-08-28

### Fixed

- Replace GDI `PrintWindow`, which cannot capture Wallpaper Engine's DirectX renderer, with Windows Graphics Capture so a normally playing Scene no longer appears black in ChatGPT.
- Capture a clean, native-resolution off-screen Wallpaper Engine window instead of `Program Manager` with desktop icons or an enlarged low-resolution Workshop preview.
- Reuse or normally start Wallpaper Engine without forcibly terminating it, preventing ReTheme from causing Wallpaper Engine safe-start recovery on the next launch.

### Optimized

- Compress BGRA frames directly with libjpeg-turbo, remove the per-frame full BGRA-to-RGB copy, and re-encode only newly captured frames.
- Pace capture at about 10 FPS; measured 2561×1601 Scene capture plus JPEG encoding dropped from about 93.7 ms to 37.1 ms per frame.
- Remove the old GDI canvas, DIB, and XPS dependencies while keeping direct Video/Web playback independent of Wallpaper Engine.

### Notes

- Scene selection still synchronizes the primary-monitor desktop to the same project. Public capture APIs cannot cleanly target Wallpaper Engine's DirectX desktop child window, so ChatGPT uses a hidden off-screen capture window; this build does not hide desktop icons, inject into another process, or disable security features.

## [0.4.6] - 2026-08-27

### Fixed

- Do not treat a present but persistently black Wallpaper Engine desktop window as a successful Scene start; require a real rendered first frame before applying the theme.
- Reuse an already-running Wallpaper Engine instance and explicitly resume playback after switching the Scene, avoiding duplicate startup and stopped-state black screens.
- Preserve the last valid frame during transient black captures instead of flashing the ChatGPT background to black.
- Report actionable safe-start recovery guidance without disabling Wallpaper Engine's crash-protection option.

### Verified

- Added black-frame detection coverage; a safe-start renderer no longer creates a black theme session, while normal Scenes continue to use one synchronized desktop renderer.

## [0.4.5] - 2026-08-27

### Changed

- Original Scene selection now also switches the primary Wallpaper Engine desktop to the same project and captures that existing desktop renderer instead of launching a second `playInWindow` copy.
- Keep the independent off-screen renderer only as an automatic compatibility fallback when the Wallpaper Engine desktop window cannot be discovered.
- Explain the desktop synchronization behavior directly in the Scene group before the user applies a wallpaper.

### Verified

- Two Workshop Scene projects used exactly one `WPEDesktop… / WPELiveWallpaper` renderer, created no `ReThemeWEScene` window, played at 14.3–14.5 FPS with zero transport errors, and passed pause/resume checks.

## [0.4.4] - 2026-08-27

### Optimized

- Reuse the Scene capture GDI surface and RGB buffer instead of allocating a full 1920x1080 capture pipeline for every frame; measured raw capture time dropped from 36.9 ms to 28.7 ms on the tested Workshop Scene.
- Keep one authorized DevTools connection for the in-memory JPEG frame stream, removing per-frame temporary-file writes, file-input injection, Blob URL replacement, and theme DOM updates.
- Decode frames on a canvas and retain only the newest pending frame when the page is busy, preventing an outdated-frame backlog and improving tested Scene playback from about 10 FPS to 12.8-15.0 FPS.

### Verified

- Two original Wallpaper Engine Scene projects rendered without transport errors; pause held the frame sequence unchanged and resume advanced it again.

## [0.4.3] - 2026-08-27

### Fixed

- Install the theme runtime as soon as the ChatGPT application shell is ready instead of waiting for the composer to finish loading. This avoids false `view:home|home-compact|conversation|other` timeouts and the follow-up `os error 10060` retry failure.
- Keep the DOM observer active so the composer, sidebar, and conversation slots are filled when those regions appear later, without changing existing transparency behavior on normal home and conversation views.

## [0.4.2] - 2026-08-26

### Fixed

- Restore original Wallpaper Engine rendering of `scene.pkg` projects instead of stretching 256×256 Workshop previews across ChatGPT.
- Output Scene frames at 1920×1080 and JPEG quality 90, then bind them through browser local-file handles and revocable Blob URLs. This avoids both ChatGPT's blocked loopback-image policy and the older per-frame Base64 DevTools overhead.

### Changed

- Put every Scene project back under “Wallpaper Engine required”; Video and sandboxed Web projects remain under “Wallpaper Engine not required.”
- Show the play/pause control for Scene projects too; pausing freezes capture and encoding, while playback replaces only the background image instead of reapplying the whole theme every frame.

## [0.4.1] - 2026-08-26

### Fixed

- Bind Scene GIF/JPG previews through a browser local-file handle and revocable Blob URL so they decode under ChatGPT's image content security policy.
- Reattach the local Scene file after page reloads or lease recovery instead of falling back to a loopback URL blocked by the page policy.

## [0.4.0] - 2026-08-26

### Added

- Play Scene projects from their Workshop GIF/JPG preview without launching Wallpaper Engine; GIF previews animate directly and static previews use a low-cost slow pan and zoom.
- Group Workshop projects by the engine requirement reported by the backend, placing lightweight Scene previews under “Wallpaper Engine not required.”

### Optimized

- Remove the Scene off-screen window, GDI capture, per-frame JPEG encoding, Base64 transport, and DevTools frame bridge to reduce CPU, GPU, memory use, and stutter.
- Lightweight mode omits original Scene particles, audio, mouse interaction, and scripts; Video and sandboxed Web playback are unchanged.

## [0.3.9] - 2026-08-25

### Fixed

- Allow the new live-wallpaper playback button classes through the theme CSS safety validator so 0.3.8 themes can apply successfully.

## [0.3.8] - 2026-08-25

### Added

- Add a play/pause button to the ReTheme live-wallpaper controls inside ChatGPT.
- Make an explicit pause override the playback recovery guard so the video stays paused until resumed.

## [0.3.7] - 2026-08-25

### Fixed

- Fix the injected-script variable scope used by the 0.3.6 bottom-gradient scan so live wallpaper preview starts and applies the dark-band fix correctly.

## [0.3.6] - 2026-08-25

### Fixed

- Detect the current ChatGPT bottom `from-surface` gradient nested inside the conversation scroller instead of checking only composer siblings.
- Remove both the conversation-bottom gradient and the smaller pre-composer gradient so the rectangular dark band is fully cleared.

## [0.3.5] - 2026-08-25

### Fixed

- Remove the main content area's dark background only in existing conversation views without changing home, settings, sidebar, or composer transparency.
- Recognize the current ChatGPT `_MainContentFrame_*` container and clear its top separator so the frame cannot return around older conversations.

## [0.3.4] - 2026-08-25

### Fixed

- Recognize the updated ChatGPT composer DOM across bottom-aligned `relative`, `absolute`, `fixed`, and `sticky` containers.
- Clear the bottom composer region's background, gradients, blur, shadow, and pseudo-elements so the large dark band cannot return below the input.

## [0.3.3] - 2026-08-25

### Fixed

- Give the two Wallpaper Engine category selectors a dedicated full-width row so their headings are not truncated.

## [0.3.2] - 2026-08-25

### Fixed

- Replace categories hidden at the bottom of one long native dropdown with two always-visible selectors showing their engine requirement and project totals.

## [0.3.1] - 2026-08-25

### Changed

- Group Wallpaper Engine projects into those that do not require the engine process (Video / Web) and those that do (Scene).

### Fixed

- Remove the large downward shadow below the ChatGPT composer while preserving composer transparency and other card shadows.

## [0.3.0] - 2026-08-25

### Added

- Support Wallpaper Engine Scene and Web projects instead of reporting those Workshop wallpapers as not runnable.
- Render Scene projects with the locally installed official Wallpaper Engine executable and deliver their frames through ReTheme's loopback theme channel.
- Run Web projects in script-only sandboxed iframes while blocking network connections, popups, forms, downloads, cross-origin access, and object embedding.

### Changed

- Report Video, Scene, and Web totals separately; all 24 Video, 33 Scene, and 2 Web projects in the current Workshop library are selectable.
- Stop only the dedicated render window created by ReTheme when a Scene wallpaper is stopped or replaced, leaving the main Wallpaper Engine process running.

## [0.2.3] - 2026-08-24

### Added

- Add a collapsible bottom-right panel inside ChatGPT with live wallpaper-brightness and interface-transparency sliders.
- Persist both slider values locally and restore them when ReTheme is reopened.

### Fixed

- Remove the complete Wallpaper Engine home hero so the blank dark banner no longer remains after its copy is hidden.
- Replace the old preview-only video-opacity behavior with separate live wallpaper-brightness and interface-overlay controls.
- Reinject the theme, remount the local video, and restore live control values when ChatGPT replaces its page target after startup.

## [0.2.2] - 2026-08-24

### Added

- Add a persistent Wallpaper Engine background-opacity control and remember the last selected Workshop project after the management window or app is reopened.

### Changed

- Hide the generated Wallpaper Engine source, title, and streaming-description copy from the ChatGPT home hero while preserving its layout.

## [0.2.1] - 2026-08-24

### Fixed

- Accommodate the ChatGPT 26.818.5229.0 startup sequence by waiting for the app document to finish navigating and remain briefly stable before connecting the local theme channel, avoiding dynamic-wallpaper timeouts.
- Retry one clean isolated instance when ChatGPT cold-starts into an incomplete shell or its local DevTools channel times out; compatibility checks still report persistent selector mismatches.
- Bind Wallpaper Engine videos through a browser-native local file handle and keep them playing while backgrounded, without unsafe web-security flags or loading the whole video into memory.
- Keep the diagnostic action visible when a long compatibility message is displayed.

## [0.2.0] - 2026-08-22

### Added

- Scan and stream Wallpaper Engine Workshop video wallpapers without the former 8 MB image-import limit.
- Run and cache compatibility self-checks per ChatGPT version, including four-part Windows package versions.
- Check configurable GitHub Releases for updates and accept only installers signed by the independent ReTheme WE key.
- Enable a low-memory mode by default that destroys the management WebView after applying a theme while keeping the tray backend available to recreate it.

### Cleanup

- Remove the unused frontend updater package and permission plus publishing workflows unrelated to this Windows fork.

## [0.1.4] - 2026-07-20

### Added

- Add a GitHub Star support entry to the overview for users who want to support the open-source project.

### Fixed

- Fix theme activation for the Microsoft Store/MSIX ChatGPT package installed under `WindowsApps`.
- Launch the isolated ChatGPT instance through its system AppUserModelId and reclaim only the process returned by that activation.

## [0.1.3] - 2026-07-19

### Added

- Allow controlled theme assets to declare `lightAsset` and `darkAsset` variants that switch with the ChatGPT appearance and fall back to `asset`.
- Share the appearance-aware asset schema across desktop loading, the theme-development Skill, and server-side community validation.

## [0.1.2] - 2026-07-19

### Added

- Add one shared Rust theme protocol and CLI validator for desktop loading, local authoring, AI workflows, and server-side community review.
- Document the complete v1 Manifest schema, 164 stable slots, annotated examples, Banner asset specifications, generation prompts, and a reusable theme-development Skill.

### Fixed

- Refresh only the affected home slots while editing the composer, avoiding full runtime reinjection and related layout flicker.
- Fully release theme observers and animation frames when replacing or restoring a runtime.

### Security

- Strictly reject unknown Manifest fields, unsafe archive paths, unscoped or structural CSS, external asset references, invalid image content, and unsafe SVG markup through the shared protocol gate.

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
