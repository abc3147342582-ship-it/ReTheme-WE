use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};
use ed25519_dalek::{Signature, VerifyingKey};
use lightningcss::properties::PropertyId;
use lightningcss::rules::{CssRule, CssRuleList};
use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};
use lightningcss::traits::ToCss;
use rand_core::{OsRng, RngCore};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use zip::ZipArchive;

use crate::security_config;

const MAX_ARCHIVE_SIZE: u64 = 30 * 1024 * 1024;
const MAX_EXTRACTED_SIZE: u64 = 60 * 1024 * 1024;
const MAX_IMAGE_SIZE: u64 = 8 * 1024 * 1024;
const MAX_FILE_COUNT: usize = 256;
const ONLINE_CACHE_MAGIC: &[u8; 4] = b"RTC1";
const ALLOWED_SLOTS: [&str; 164] = [
    "app.shell",
    "app.background",
    "titlebar",
    "sidebar",
    "sidebar.scroll",
    "sidebar.resize",
    "sidebar.resize.indicator",
    "sidebar.header",
    "sidebar.header.icon",
    "sidebar.header.label",
    "sidebar.header.background",
    "sidebar.header.decoration",
    "sidebar.brand",
    "sidebar.brand.icon",
    "sidebar.brand.badge",
    "sidebar.frame",
    "sidebar.section",
    "sidebar.section.projects",
    "sidebar.section.header",
    "sidebar.section.toggle",
    "sidebar.section.label",
    "sidebar.section.actions",
    "sidebar.section.action",
    "sidebar.section.action.icon",
    "sidebar.section.decoration",
    "sidebar.item",
    "sidebar.item.icon",
    "sidebar.item.label",
    "sidebar.item.active",
    "sidebar.item.active.icon",
    "sidebar.item.active.label",
    "sidebar.footer",
    "sidebar.footer.item",
    "sidebar.footer.icon",
    "sidebar.footer.label",
    "sidebar.footer.brand",
    "sidebar.footer.brand.label",
    "sidebar.footer.brand.timer",
    "sidebar.footer.brand.pro",
    "sidebar.footer.brand.version",
    "main",
    "main.fade",
    "main.content.frame",
    "main.background",
    "main.overlay",
    "main.frame",
    "page",
    "page.surface",
    "page.header",
    "page.content",
    "menu",
    "menu.item",
    "menu.item.active",
    "menu.item.checked",
    "menu.icon",
    "menu.label",
    "menu.shortcut",
    "menu.separator",
    "composer",
    "composer.backdrop",
    "composer.context",
    "composer.context.item",
    "composer.context.item.icon",
    "composer.context.item.label",
    "composer.editor",
    "composer.action",
    "composer.action.icon",
    "composer.action.label",
    "composer.permission",
    "composer.permission.icon",
    "composer.permission.label",
    "composer.submit",
    "composer.submit.icon",
    "composer.submit.decoration",
    "composer.decoration",
    "composer.panel",
    "composer.panel.item",
    "composer.panel.icon",
    "composer.panel.separator",
    "home.hero",
    "home.hero.viewport",
    "home.hero.copy",
    "home.hero.eyebrow",
    "home.hero.title",
    "home.hero.description",
    "home.hero.media",
    "home.hero.media.asset",
    "home.hero.foreground",
    "home.hero.foreground.asset",
    "home.hero.divider",
    "home.hero.divider.icon",
    "home.hero.divider.label",
    "home.hero.divider.line",
    "home.layout",
    "home.content.region",
    "home.stage",
    "home.brand",
    "home.prompt",
    "home.prompt.title",
    "home.cards",
    "home.cards.layout",
    "home.cards.grid",
    "home.card",
    "home.card.background",
    "home.card.content",
    "home.card.icon",
    "home.card.icon.glyph",
    "home.card.label",
    "home.card.arrow",
    "home.card.arrow.glyph",
    "home.card.arrow.asset",
    "composer.region",
    "conversation.stage",
    "conversation.header",
    "conversation.header.content",
    "conversation.viewport",
    "conversation.summary.region",
    "conversation.summary",
    "conversation.summary.decoration",
    "conversation",
    "conversation.user",
    "conversation.assistant",
    "conversation.banner",
    "conversation.banner.copy",
    "conversation.banner.eyebrow",
    "conversation.banner.title",
    "conversation.banner.description",
    "conversation.banner.media",
    "conversation.banner.media.asset",
    "conversation.banner.foreground",
    "conversation.banner.foreground.asset",
    "code",
    "code.inline",
    "diff",
    "terminal",
    "terminal.viewport",
    "settings",
    "settings.header",
    "settings.sidebar",
    "settings.nav",
    "settings.nav.item",
    "settings.nav.item.active",
    "settings.content",
    "settings.surface",
    "settings.frame",
    "settings.canvas",
    "settings.toolbar",
    "settings.body",
    "settings.section",
    "settings.section.title",
    "settings.card",
    "settings.row",
    "settings.row.title",
    "settings.row.description",
    "settings.row.separator",
    "settings.control",
    "settings.control.checked",
    "settings.switch",
    "settings.switch.checked",
    "settings.switch.track",
    "settings.switch.track.checked",
    "settings.switch.thumb",
    "decoration.top-right",
    "decoration.bottom-right",
];
const ALLOWED_ASSET_SLOTS: [&str; 11] = [
    "app.background",
    "main.background",
    "main.overlay",
    "main.frame",
    "sidebar.brand.icon",
    "sidebar.brand.badge",
    "sidebar.header.background",
    "sidebar.header.decoration",
    "sidebar.frame",
    "home.card.background",
    "home.card.arrow.asset",
];
const ALLOWED_MOUNTS: [&str; 19] = [
    "home.hero",
    "app.background",
    "main.background",
    "main.overlay",
    "main.frame",
    "sidebar.brand.icon",
    "sidebar.brand.badge",
    "sidebar.header.background",
    "sidebar.header.decoration",
    "sidebar.frame",
    "sidebar.footer.brand",
    "home.card.background",
    "home.card.arrow.asset",
    "composer.submit.decoration",
    "composer.decoration",
    "conversation.summary.decoration",
    "sidebar.section.decoration",
    "decoration.top-right",
    "decoration.bottom-right",
];

#[derive(Debug)]
pub struct ThemeError(String);

impl fmt::Display for ThemeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ThemeError {}

impl From<std::io::Error> for ThemeError {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeManifest {
    schema_version: u32,
    id: String,
    name: String,
    description: String,
    version: String,
    author: ThemeAuthor,
    #[serde(
        default,
        rename = "testedCodexVersions",
        alias = "supportedCodexVersions"
    )]
    _tested_codex_versions: Vec<String>,
    styles: Vec<String>,
    slots: Vec<String>,
    permissions: Vec<String>,
    preview: ThemePreview,
    experience: ThemeExperience,
    #[serde(default)]
    locales: BTreeMap<String, ThemeLocalization>,
    #[serde(default)]
    access: Option<ThemePackageAccess>,
    #[serde(default)]
    integrity: Option<String>,
    #[serde(default)]
    signature: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ThemeLocalization {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    experience: Option<ThemeExperienceLocalization>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ThemeExperienceLocalization {
    #[serde(default)]
    home_hero: Option<ThemeHeroLocalization>,
    #[serde(default)]
    home_prompt: Option<ThemeHomePrompt>,
    #[serde(default)]
    conversation_banner: Option<ThemeBannerLocalization>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ThemeHeroLocalization {
    #[serde(default)]
    eyebrow: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    divider: Option<ThemeDividerLocalization>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ThemeBannerLocalization {
    #[serde(default)]
    eyebrow: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ThemeDividerLocalization {
    #[serde(default)]
    label: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThemePackageAccess {
    delivery: String,
    slug: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ThemeAuthor {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThemePreview {
    background: String,
    surface: String,
    accent: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThemeExperience {
    home_hero: ThemeHero,
    #[serde(default)]
    home_prompt: Option<ThemeHomePrompt>,
    #[serde(default)]
    conversation_banner: Option<ThemeConversationBanner>,
    #[serde(default)]
    composer_submit: Option<ThemeAsset>,
    #[serde(default)]
    composer_decoration: Option<ThemeAsset>,
    #[serde(default)]
    conversation_summary_decoration: Option<ThemeAsset>,
    #[serde(default)]
    sidebar_section_decoration: Option<ThemeAsset>,
    #[serde(default)]
    assets: Vec<ThemeDecoration>,
    decorations: Vec<ThemeDecoration>,
}

#[derive(Debug, Clone, Deserialize)]
struct ThemeAsset {
    asset: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ThemeHero {
    eyebrow: String,
    title: String,
    description: String,
    asset: String,
    fit: String,
    position: String,
    #[serde(default)]
    foreground: Option<String>,
    #[serde(default)]
    divider: Option<ThemeHeroDivider>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ThemeHomePrompt {
    title: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ThemeConversationBanner {
    #[serde(default)]
    eyebrow: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    asset: String,
    fit: String,
    position: String,
    #[serde(default)]
    foreground: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ThemeHeroDivider {
    label: String,
    #[serde(default)]
    asset: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ThemeDecoration {
    slot: String,
    asset: String,
}

#[derive(Debug, Clone)]
pub struct ThemePackage {
    manifest: ThemeManifest,
    css: String,
    runtime_assets: HashMap<String, ThemeRuntimeAsset>,
}

#[derive(Debug, Clone)]
pub(crate) struct ThemeRuntimeAsset {
    pub path: String,
    pub mime: String,
    pub source: Arc<[u8]>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeSummary {
    id: String,
    name: String,
    description: String,
    version: String,
    author: String,
    preview: ThemePreview,
    built_in: bool,
    online_slug: Option<String>,
    requires_account: bool,
    locales: BTreeMap<String, ThemeSummaryLocalization>,
}

#[derive(Debug, Clone, Serialize)]
struct ThemeSummaryLocalization {
    name: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeInstallReport {
    theme: ThemeSummary,
    replaced: bool,
    package_digest: String,
    signature_verified: bool,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct ThemeRegistry {
    themes: BTreeMap<String, InstalledTheme>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct InstalledTheme {
    version: String,
    digest: String,
    online: OnlineThemeSource,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct OnlineThemeSource {
    slug: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct IntegrityIndex {
    algorithm: String,
    files: BTreeMap<String, String>,
}

#[derive(Clone)]
pub struct ThemeRepository {
    root: PathBuf,
    verifying_key: VerifyingKey,
    cache_key: Option<[u8; 32]>,
}

impl ThemePackage {
    pub fn id(&self) -> &str {
        &self.manifest.id
    }

    pub fn css(&self) -> &str {
        &self.css
    }

    pub(crate) fn version(&self) -> &str {
        &self.manifest.version
    }

    pub(crate) fn runtime_assets(&self) -> Vec<ThemeRuntimeAsset> {
        self.runtime_assets.values().cloned().collect()
    }

    pub(crate) fn runtime_config_with_asset_urls(
        &self,
        asset_urls: &HashMap<String, String>,
    ) -> Result<Value, ThemeError> {
        build_runtime_config_with(&self.manifest, |path| {
            asset_urls
                .get(path)
                .cloned()
                .ok_or_else(|| ThemeError(format!("主题资源 URL 缺失：{path}")))
        })
    }

    pub fn preview_summary(&self) -> ThemeSummary {
        self.summary(false, None)
    }

    fn summary(&self, built_in: bool, online: Option<&OnlineThemeSource>) -> ThemeSummary {
        ThemeSummary {
            id: self.manifest.id.clone(),
            name: self.manifest.name.clone(),
            description: self.manifest.description.clone(),
            version: self.manifest.version.clone(),
            author: self.manifest.author.name.clone(),
            preview: self.manifest.preview.clone(),
            built_in,
            online_slug: online.map(|source| source.slug.clone()),
            requires_account: false,
            locales: self
                .manifest
                .locales
                .iter()
                .map(|(locale, translation)| {
                    (
                        locale.clone(),
                        ThemeSummaryLocalization {
                            name: translation.name.clone(),
                            description: translation.description.clone(),
                        },
                    )
                })
                .collect(),
        }
    }
}

impl ThemeRepository {
    #[cfg(test)]
    pub fn new(root: PathBuf) -> Result<Self, ThemeError> {
        Ok(Self {
            root,
            verifying_key: platform_verifying_key()?,
            cache_key: None,
        })
    }

    pub fn new_with_cache_key(root: PathBuf, cache_key: [u8; 32]) -> Result<Self, ThemeError> {
        Ok(Self {
            root,
            verifying_key: platform_verifying_key()?,
            cache_key: Some(cache_key),
        })
    }

    pub fn list(&self) -> Result<Vec<ThemeSummary>, ThemeError> {
        let mut themes = Vec::new();
        for (theme_id, installed) in self.read_registry()?.themes {
            let package = self.load_installed(&theme_id, &installed)?;
            themes.push(package.summary(false, Some(&installed.online)));
        }
        Ok(themes)
    }

    pub fn load(&self, theme_id: &str) -> Result<ThemePackage, ThemeError> {
        let registry = self.read_registry()?;
        let installed = registry
            .themes
            .get(theme_id)
            .ok_or_else(|| ThemeError(format!("未知主题：{theme_id}")))?;
        self.load_installed(theme_id, installed)
    }

    pub fn online_slug(&self, theme_id: &str) -> Result<String, ThemeError> {
        let registry = self.read_registry()?;
        let installed = registry
            .themes
            .get(theme_id)
            .ok_or_else(|| ThemeError(format!("未知主题：{theme_id}")))?;
        Ok(installed.online.slug.clone())
    }

    pub fn load_development(&self, theme_path: &Path) -> Result<ThemePackage, ThemeError> {
        let files = read_development_directory(theme_path)?;
        parse_package_files(&files, false)
    }

    pub fn install_online(
        &self,
        archive_bytes: &[u8],
        slug: String,
    ) -> Result<ThemeInstallReport, ThemeError> {
        self.install_bytes(archive_bytes, OnlineThemeSource { slug })
    }

    pub fn uninstall(&self, theme_id: &str) -> Result<bool, ThemeError> {
        let mut registry = self.read_registry()?;
        let installed = registry
            .themes
            .remove(theme_id)
            .ok_or_else(|| ThemeError(format!("主题未安装：{theme_id}")))?;
        let cache_is_shared = registry
            .themes
            .values()
            .any(|item| item.digest == installed.digest);
        let registry_bytes = serde_json::to_vec_pretty(&registry)
            .map_err(|error| ThemeError(format!("无法保存主题注册表：{error}")))?;
        fs::create_dir_all(&self.root)?;
        write_atomic(&self.registry_path(), &registry_bytes)?;
        if !cache_is_shared {
            let cache_path = self.store_path(&installed.digest);
            if cache_path.exists() {
                fs::remove_file(&cache_path).map_err(|error| {
                    ThemeError(format!(
                        "主题已卸载，但无法删除缓存 {}：{error}",
                        cache_path.display()
                    ))
                })?;
            }
        }
        Ok(true)
    }

    fn install_bytes(
        &self,
        archive_bytes: &[u8],
        online: OnlineThemeSource,
    ) -> Result<ThemeInstallReport, ThemeError> {
        if archive_bytes.len() as u64 > MAX_ARCHIVE_SIZE {
            return Err(ThemeError("主题包超过 30 MB".into()));
        }
        let files = read_archive(archive_bytes)?;
        verify_integrity_and_signature(&files, &self.verifying_key)?;
        let package = parse_package_files(&files, true)?;
        validate_package_access(&package.manifest, &online)?;

        let digest = sha256_hex(archive_bytes);
        let mut registry = self.read_registry()?;
        let replaced = registry.themes.contains_key(package.id());
        if let Some(current) = registry.themes.get(package.id()) {
            let current_version = Version::parse(&current.version)
                .map_err(|error| ThemeError(format!("已安装主题版本损坏：{error}")))?;
            let next_version = Version::parse(&package.manifest.version)
                .map_err(|error| ThemeError(format!("主题版本无效：{error}")))?;
            if next_version < current_version {
                return Err(ThemeError("不能用较旧版本覆盖已安装主题".into()));
            }
            if next_version == current_version && digest != current.digest {
                return Err(ThemeError("同一主题版本发布后不可覆盖内容".into()));
            }
        }

        fs::create_dir_all(self.store_dir())?;
        let store_path = self.store_path(&digest);
        if !store_path.exists() {
            let stored = self.encrypt_online_cache(&digest, archive_bytes)?;
            write_atomic(&store_path, &stored)?;
        }
        registry.themes.insert(
            package.id().to_owned(),
            InstalledTheme {
                version: package.manifest.version.clone(),
                digest: digest.clone(),
                online,
            },
        );
        fs::create_dir_all(&self.root)?;
        let registry_bytes = serde_json::to_vec_pretty(&registry)
            .map_err(|error| ThemeError(format!("无法保存主题注册表：{error}")))?;
        write_atomic(&self.registry_path(), &registry_bytes)?;

        Ok(ThemeInstallReport {
            theme: package.summary(
                false,
                registry.themes.get(package.id()).map(|item| &item.online),
            ),
            replaced,
            package_digest: digest,
            signature_verified: true,
        })
    }

    fn load_installed(
        &self,
        theme_id: &str,
        installed: &InstalledTheme,
    ) -> Result<ThemePackage, ThemeError> {
        if !is_sha256(&installed.digest) {
            return Err(ThemeError(format!("主题 {theme_id} 的注册表摘要无效")));
        }
        let stored = fs::read(self.store_path(&installed.digest))
            .map_err(|error| ThemeError(format!("无法读取已安装主题 {theme_id}：{error}")))?;
        let archive_bytes = self.decrypt_online_cache(&installed.digest, &stored)?;
        if sha256_hex(&archive_bytes) != installed.digest {
            return Err(ThemeError(format!("已安装主题 {theme_id} 的缓存已被篡改")));
        }
        let files = read_archive(&archive_bytes)?;
        verify_integrity_and_signature(&files, &self.verifying_key)?;
        let package = parse_package_files(&files, true)?;
        if package.id() != theme_id || package.manifest.version != installed.version {
            return Err(ThemeError(format!("主题 {theme_id} 与注册表不一致")));
        }
        validate_package_access(&package.manifest, &installed.online)?;
        Ok(package)
    }

    fn read_registry(&self) -> Result<ThemeRegistry, ThemeError> {
        let path = self.registry_path();
        if !path.exists() {
            return Ok(ThemeRegistry::default());
        }
        let source = fs::read(&path)?;
        serde_json::from_slice(&source)
            .map_err(|error| ThemeError(format!("主题注册表损坏：{error}")))
    }

    fn registry_path(&self) -> PathBuf {
        self.root.join("registry.json")
    }

    fn store_dir(&self) -> PathBuf {
        self.root.join("store")
    }

    fn store_path(&self, digest: &str) -> PathBuf {
        self.store_dir().join(format!("{digest}.cache"))
    }

    fn encrypt_online_cache(&self, digest: &str, source: &[u8]) -> Result<Vec<u8>, ThemeError> {
        let key = self
            .cache_key
            .ok_or_else(|| ThemeError("当前设备无法保护在线主题缓存".into()))?;
        let cipher = XChaCha20Poly1305::new_from_slice(&key)
            .map_err(|_| ThemeError("在线主题缓存密钥无效".into()))?;
        let mut nonce = [0_u8; 24];
        OsRng.fill_bytes(&mut nonce);
        let ciphertext = cipher
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: source,
                    aad: digest.as_bytes(),
                },
            )
            .map_err(|_| ThemeError("无法加密在线主题缓存".into()))?;
        let mut stored =
            Vec::with_capacity(ONLINE_CACHE_MAGIC.len() + nonce.len() + ciphertext.len());
        stored.extend_from_slice(ONLINE_CACHE_MAGIC);
        stored.extend_from_slice(&nonce);
        stored.extend_from_slice(&ciphertext);
        Ok(stored)
    }

    fn decrypt_online_cache(&self, digest: &str, stored: &[u8]) -> Result<Vec<u8>, ThemeError> {
        if stored.len() <= ONLINE_CACHE_MAGIC.len() + 24
            || &stored[..ONLINE_CACHE_MAGIC.len()] != ONLINE_CACHE_MAGIC
        {
            return Err(ThemeError("在线主题缓存格式已过期，请重新下载".into()));
        }
        let key = self
            .cache_key
            .ok_or_else(|| ThemeError("当前设备无法读取在线主题缓存".into()))?;
        let cipher = XChaCha20Poly1305::new_from_slice(&key)
            .map_err(|_| ThemeError("在线主题缓存密钥无效".into()))?;
        let nonce_end = ONLINE_CACHE_MAGIC.len() + 24;
        cipher
            .decrypt(
                XNonce::from_slice(&stored[ONLINE_CACHE_MAGIC.len()..nonce_end]),
                Payload {
                    msg: &stored[nonce_end..],
                    aad: digest.as_bytes(),
                },
            )
            .map_err(|_| ThemeError("在线主题缓存与当前设备不匹配或已损坏".into()))
    }
}

fn validate_package_access(
    manifest: &ThemeManifest,
    online: &OnlineThemeSource,
) -> Result<(), ThemeError> {
    match manifest.access.as_ref() {
        Some(access) if access.delivery == "online" && access.slug == online.slug => Ok(()),
        None => Err(ThemeError(
            "在线主题包缺少签名授权策略，请重新下载最新版本".into(),
        )),
        Some(_) => Err(ThemeError("在线主题包授权策略与服务端不一致".into())),
    }
}

#[cfg(test)]
const TEST_THEME_MANIFEST: &str = r##"{
  "schemaVersion": 1,
  "id": "studio.example.test-theme",
  "name": "Test Theme",
  "description": "A protocol test theme.",
  "version": "1.0.0",
  "author": { "id": "studio.example", "name": "Example Studio" },
  "testedCodexVersions": ["26.707.91948"],
  "styles": ["styles/theme.css"],
  "slots": [
    "app.shell",
    "home.hero",
    "conversation.banner",
    "composer.decoration",
    "conversation.summary.decoration"
  ],
  "permissions": [],
  "preview": { "background": "#111111", "surface": "#222222", "accent": "#aaff44" },
  "experience": {
    "homeHero": {
      "eyebrow": "TEST",
      "title": "Protocol theme",
      "description": "Theme engine verification",
      "asset": "assets/hero.svg",
      "fit": "cover",
      "position": "center"
    },
    "homePrompt": { "title": "What should we test?" },
    "conversationBanner": {
      "eyebrow": "SESSION",
      "title": "Test conversation",
      "description": "Narrow banner verification",
      "asset": "assets/banner-chat.svg",
      "fit": "contain",
      "position": "right"
    },
    "composerDecoration": { "asset": "assets/ornament.svg" },
    "conversationSummaryDecoration": { "asset": "assets/ornament.svg" },
    "decorations": []
  },
  "locales": {
    "zh-CN": {
      "name": "测试主题",
      "description": "用于验证主题协议。",
      "experience": {
        "homePrompt": { "title": "今天测试什么？" }
      }
    }
  }
}"##;

#[cfg(test)]
const TEST_THEME_CSS: &str = r#":root[data-ct-theme="studio.example.test-theme"] [data-ct-slot="app.shell"] { color: white; }"#;

#[cfg(test)]
const TEST_THEME_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0 0h10v10H0z"/></svg>"#;

#[cfg(test)]
fn test_theme_files() -> HashMap<String, Vec<u8>> {
    HashMap::from([
        (
            "manifest.json".to_owned(),
            TEST_THEME_MANIFEST.as_bytes().to_vec(),
        ),
        (
            "styles/theme.css".to_owned(),
            TEST_THEME_CSS.as_bytes().to_vec(),
        ),
        (
            "assets/hero.svg".to_owned(),
            TEST_THEME_SVG.as_bytes().to_vec(),
        ),
        (
            "assets/banner-chat.svg".to_owned(),
            TEST_THEME_SVG.as_bytes().to_vec(),
        ),
        (
            "assets/ornament.svg".to_owned(),
            TEST_THEME_SVG.as_bytes().to_vec(),
        ),
    ])
}

#[cfg(test)]
pub(crate) fn test_theme_summary() -> ThemeSummary {
    test_theme_package()
        .expect("test theme should be valid")
        .preview_summary()
}

#[cfg(test)]
fn test_theme_package() -> Result<ThemePackage, ThemeError> {
    parse_package_files(&test_theme_files(), false)
}

fn parse_package_files(
    files: &HashMap<String, Vec<u8>>,
    signed_package: bool,
) -> Result<ThemePackage, ThemeError> {
    let manifest_source = utf8_file(files, "manifest.json")?;
    let manifest: ThemeManifest = serde_json::from_str(manifest_source)
        .map_err(|error| ThemeError(format!("主题 Manifest 无效：{error}")))?;
    validate_manifest(&manifest)?;
    if signed_package
        && (manifest.integrity.as_deref() != Some("integrity.json")
            || manifest.signature.as_deref() != Some("signature.ed25519"))
    {
        return Err(ThemeError(
            "正式主题必须声明 integrity.json 与 signature.ed25519".into(),
        ));
    }
    if let Some(source) = files.get("compatibility.json") {
        serde_json::from_slice::<Value>(source)
            .map_err(|error| ThemeError(format!("compatibility.json 无效：{error}")))?;
    }

    let mut css = String::new();
    for style_path in &manifest.styles {
        validate_style_path(style_path)?;
        let source = utf8_file(files, style_path)?;
        validate_css(source, &manifest.id)?;
        css.push_str(source);
        css.push('\n');
    }
    let runtime_config = build_runtime_config(&manifest, files)?;
    let runtime_assets = collect_runtime_assets(files, &runtime_config)?;

    Ok(ThemePackage {
        manifest,
        css,
        runtime_assets,
    })
}

fn validate_manifest(manifest: &ThemeManifest) -> Result<(), ThemeError> {
    if manifest.schema_version != 1 {
        return Err(ThemeError(format!(
            "不支持主题规范版本：{}",
            manifest.schema_version
        )));
    }
    if !is_valid_id(&manifest.id) || !is_valid_id(&manifest.author.id) {
        return Err(ThemeError("主题或作者 ID 格式无效".into()));
    }
    Version::parse(&manifest.version)
        .map_err(|error| ThemeError(format!("主题版本无效：{error}")))?;
    if manifest.name.trim().is_empty()
        || manifest.description.trim().is_empty()
        || manifest.author.name.trim().is_empty()
    {
        return Err(ThemeError("主题名称、说明和作者不能为空".into()));
    }
    if !manifest.permissions.is_empty() {
        return Err(ThemeError("第一阶段主题 permissions 必须为空".into()));
    }
    for (locale, translation) in &manifest.locales {
        if !is_valid_locale(locale) {
            return Err(ThemeError(format!("主题语言标签无效：{locale}")));
        }
        validate_optional_text("Localized theme name", translation.name.as_deref(), 120)?;
        validate_optional_text(
            "Localized theme description",
            translation.description.as_deref(),
            240,
        )?;
        if let Some(experience) = &translation.experience {
            if let Some(hero) = &experience.home_hero {
                validate_optional_text("Localized Banner eyebrow", hero.eyebrow.as_deref(), 80)?;
                validate_optional_text("Localized Banner title", hero.title.as_deref(), 120)?;
                validate_optional_text(
                    "Localized Banner description",
                    hero.description.as_deref(),
                    240,
                )?;
                if let Some(divider) = &hero.divider {
                    validate_optional_text(
                        "Localized divider label",
                        divider.label.as_deref(),
                        80,
                    )?;
                }
            }
            if let Some(prompt) = &experience.home_prompt {
                validate_text("Localized home prompt title", &prompt.title, 120)?;
            }
            if let Some(banner) = &experience.conversation_banner {
                validate_optional_text(
                    "Localized conversation eyebrow",
                    banner.eyebrow.as_deref(),
                    80,
                )?;
                validate_optional_text(
                    "Localized conversation title",
                    banner.title.as_deref(),
                    120,
                )?;
                validate_optional_text(
                    "Localized conversation description",
                    banner.description.as_deref(),
                    240,
                )?;
            }
        }
    }
    if manifest.styles.is_empty() {
        return Err(ThemeError("主题至少需要一个样式文件".into()));
    }
    validate_text("Banner eyebrow", &manifest.experience.home_hero.eyebrow, 80)?;
    validate_text("Banner title", &manifest.experience.home_hero.title, 120)?;
    validate_text(
        "Banner description",
        &manifest.experience.home_hero.description,
        240,
    )?;
    if !matches!(
        manifest.experience.home_hero.fit.as_str(),
        "cover" | "contain"
    ) {
        return Err(ThemeError("Banner fit 只允许 cover 或 contain".into()));
    }
    if !matches!(
        manifest.experience.home_hero.position.as_str(),
        "center" | "top" | "bottom" | "left" | "right"
    ) {
        return Err(ThemeError("Banner position 不在允许范围".into()));
    }
    validate_asset_path(&manifest.experience.home_hero.asset)?;
    if let Some(foreground) = &manifest.experience.home_hero.foreground {
        validate_asset_path(foreground)?;
    }
    if let Some(prompt) = &manifest.experience.home_prompt {
        validate_text("Home prompt title", &prompt.title, 120)?;
    }
    if let Some(banner) = &manifest.experience.conversation_banner {
        validate_text("Conversation Banner eyebrow", &banner.eyebrow, 80)?;
        validate_text("Conversation Banner title", &banner.title, 120)?;
        validate_text("Conversation Banner description", &banner.description, 240)?;
        if !matches!(banner.fit.as_str(), "cover" | "contain") {
            return Err(ThemeError(
                "Conversation Banner fit 只允许 cover 或 contain".into(),
            ));
        }
        if !matches!(
            banner.position.as_str(),
            "center" | "top" | "bottom" | "left" | "right"
        ) {
            return Err(ThemeError(
                "Conversation Banner position 不在允许范围".into(),
            ));
        }
        validate_asset_path(&banner.asset)?;
        if let Some(foreground) = &banner.foreground {
            validate_asset_path(foreground)?;
        }
    }
    if let Some(divider) = &manifest.experience.home_hero.divider {
        validate_text("Banner divider label", &divider.label, 80)?;
        if let Some(asset) = &divider.asset {
            validate_asset_path(asset)?;
        }
    }
    for asset in [
        &manifest.experience.composer_submit,
        &manifest.experience.composer_decoration,
        &manifest.experience.conversation_summary_decoration,
        &manifest.experience.sidebar_section_decoration,
    ]
    .into_iter()
    .flatten()
    {
        validate_asset_path(&asset.asset)?;
    }
    let mut asset_slots = HashSet::new();
    for asset in &manifest.experience.assets {
        if !ALLOWED_ASSET_SLOTS.contains(&asset.slot.as_str()) {
            return Err(ThemeError("主题资源声明了未开放的插槽".into()));
        }
        if !manifest.slots.iter().any(|slot| slot == &asset.slot) {
            return Err(ThemeError(format!(
                "主题资源插槽未在 slots 中声明：{}",
                asset.slot
            )));
        }
        if !asset_slots.insert(asset.slot.as_str()) {
            return Err(ThemeError(format!("主题资源插槽重复：{}", asset.slot)));
        }
        validate_asset_path(&asset.asset)?;
    }
    for decoration in &manifest.experience.decorations {
        if !matches!(
            decoration.slot.as_str(),
            "decoration.top-right" | "decoration.bottom-right"
        ) {
            return Err(ThemeError("主题装饰声明了未开放的插槽".into()));
        }
        validate_asset_path(&decoration.asset)?;
    }
    let allowed_slots: HashSet<&str> = ALLOWED_SLOTS.into_iter().collect();
    if manifest
        .slots
        .iter()
        .any(|slot| !allowed_slots.contains(slot.as_str()))
    {
        return Err(ThemeError("主题声明了未开放的逻辑插槽".into()));
    }
    for color in [
        &manifest.preview.background,
        &manifest.preview.surface,
        &manifest.preview.accent,
    ] {
        if !is_hex_color(color) {
            return Err(ThemeError(format!("主题预览色无效：{color}")));
        }
    }
    Ok(())
}

fn build_runtime_config(
    manifest: &ThemeManifest,
    files: &HashMap<String, Vec<u8>>,
) -> Result<Value, ThemeError> {
    build_runtime_config_with(manifest, |path| {
        let source = files
            .get(path)
            .ok_or_else(|| ThemeError(format!("主题缺少资源文件：{path}")))?;
        validate_image(path, source)?;
        Ok(path.to_owned())
    })
}

fn build_runtime_config_with(
    manifest: &ThemeManifest,
    load_asset_url: impl Fn(&str) -> Result<String, ThemeError>,
) -> Result<Value, ThemeError> {
    let hero = &manifest.experience.home_hero;
    let hero_asset = load_asset_url(&hero.asset)?;
    let hero_foreground_asset = hero
        .foreground
        .as_deref()
        .map(&load_asset_url)
        .transpose()?;
    let home_prompt = manifest
        .experience
        .home_prompt
        .as_ref()
        .map(|prompt| json!({ "title": prompt.title }));
    let conversation_banner = manifest
        .experience
        .conversation_banner
        .as_ref()
        .map(|banner| -> Result<Value, ThemeError> {
            Ok(json!({
                "eyebrow": banner.eyebrow,
                "title": banner.title,
                "description": banner.description,
                "assetUrl": load_asset_url(&banner.asset)?,
                "foregroundAssetUrl": banner
                    .foreground
                    .as_deref()
                    .map(&load_asset_url)
                    .transpose()?,
                "fit": banner.fit,
                "position": banner.position,
            }))
        })
        .transpose()?;
    let hero_divider = hero
        .divider
        .as_ref()
        .map(|divider| -> Result<Value, ThemeError> {
            Ok(json!({
                "label": divider.label,
                "assetUrl": divider
                    .asset
                    .as_deref()
                    .map(&load_asset_url)
                    .transpose()?,
            }))
        })
        .transpose()?;
    let decorations = manifest
        .experience
        .decorations
        .iter()
        .map(|decoration| {
            Ok(json!({
                "slot": decoration.slot,
                "assetUrl": load_asset_url(&decoration.asset)?,
            }))
        })
        .collect::<Result<Vec<_>, ThemeError>>()?;
    let composer_submit = manifest
        .experience
        .composer_submit
        .as_ref()
        .map(|visual| load_asset_url(&visual.asset))
        .transpose()?;
    let composer_decoration = manifest
        .experience
        .composer_decoration
        .as_ref()
        .map(|visual| load_asset_url(&visual.asset))
        .transpose()?;
    let conversation_summary_decoration = manifest
        .experience
        .conversation_summary_decoration
        .as_ref()
        .map(|visual| load_asset_url(&visual.asset))
        .transpose()?;
    let sidebar_section_decoration = manifest
        .experience
        .sidebar_section_decoration
        .as_ref()
        .map(|visual| load_asset_url(&visual.asset))
        .transpose()?;
    let assets = manifest
        .experience
        .assets
        .iter()
        .map(|asset| {
            Ok(json!({
                "slot": asset.slot,
                "assetUrl": load_asset_url(&asset.asset)?,
            }))
        })
        .collect::<Result<Vec<_>, ThemeError>>()?;

    Ok(json!({
        "hero": {
            "eyebrow": hero.eyebrow,
            "title": hero.title,
            "description": hero.description,
            "assetUrl": hero_asset,
            "foregroundAssetUrl": hero_foreground_asset,
            "fit": hero.fit,
            "position": hero.position,
            "divider": hero_divider,
        },
        "homePrompt": home_prompt,
        "conversationBanner": conversation_banner,
        "composerSubmit": composer_submit.map(|asset_url| json!({ "assetUrl": asset_url })),
        "composerDecoration": composer_decoration.map(|asset_url| json!({ "assetUrl": asset_url })),
        "conversationSummaryDecoration": conversation_summary_decoration
            .map(|asset_url| json!({ "assetUrl": asset_url })),
        "sidebarSectionDecoration": sidebar_section_decoration
            .map(|asset_url| json!({ "assetUrl": asset_url })),
        "assets": assets,
        "decorations": decorations,
        "locales": manifest.locales,
    }))
}

fn collect_runtime_assets(
    files: &HashMap<String, Vec<u8>>,
    runtime_config: &Value,
) -> Result<HashMap<String, ThemeRuntimeAsset>, ThemeError> {
    let mut paths = HashSet::new();
    collect_runtime_asset_paths(runtime_config, &mut paths);
    paths
        .into_iter()
        .map(|path| {
            let source = files
                .get(path)
                .ok_or_else(|| ThemeError(format!("主题缺少资源文件：{path}")))?;
            let mime = validate_image(path, source)?.to_owned();
            Ok((
                path.to_owned(),
                ThemeRuntimeAsset {
                    path: path.to_owned(),
                    mime,
                    source: Arc::from(source.clone()),
                },
            ))
        })
        .collect()
}

fn collect_runtime_asset_paths<'a>(value: &'a Value, paths: &mut HashSet<&'a str>) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if matches!(key.as_str(), "assetUrl" | "foregroundAssetUrl") {
                    if let Some(path) = value.as_str() {
                        paths.insert(path);
                    }
                } else {
                    collect_runtime_asset_paths(value, paths);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_runtime_asset_paths(value, paths);
            }
        }
        _ => {}
    }
}

fn read_archive(archive_bytes: &[u8]) -> Result<HashMap<String, Vec<u8>>, ThemeError> {
    let mut archive = ZipArchive::new(Cursor::new(archive_bytes))
        .map_err(|error| ThemeError(format!(".ctheme 不是有效 ZIP：{error}")))?;
    if archive.len() > MAX_FILE_COUNT {
        return Err(ThemeError("主题包文件数量超过 256".into()));
    }
    let mut files = HashMap::new();
    let mut extracted_size = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| ThemeError(format!("无法读取主题包：{error}")))?;
        let path = entry
            .enclosed_name()
            .ok_or_else(|| ThemeError("主题包包含越界路径".into()))?;
        if entry.is_dir() {
            continue;
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(ThemeError("主题包不能包含符号链接".into()));
        }
        let path = normalize_package_path(&path)?;
        validate_package_file(&path, entry.size())?;
        extracted_size = extracted_size
            .checked_add(entry.size())
            .ok_or_else(|| ThemeError("主题包解压大小溢出".into()))?;
        if extracted_size > MAX_EXTRACTED_SIZE {
            return Err(ThemeError("主题包解压后超过 60 MB".into()));
        }
        let declared_size = entry.size();
        let mut source = Vec::with_capacity(declared_size as usize);
        (&mut entry)
            .take(declared_size + 1)
            .read_to_end(&mut source)
            .map_err(|error| ThemeError(format!("无法解压 {path}：{error}")))?;
        if source.len() as u64 != declared_size {
            return Err(ThemeError(format!("主题文件长度不一致：{path}")));
        }
        if files.insert(path.clone(), source).is_some() {
            return Err(ThemeError(format!("主题包包含重复文件：{path}")));
        }
    }
    for required in ["manifest.json", "integrity.json", "signature.ed25519"] {
        if !files.contains_key(required) {
            return Err(ThemeError(format!("主题包缺少 {required}")));
        }
    }
    Ok(files)
}

fn read_development_directory(root: &Path) -> Result<HashMap<String, Vec<u8>>, ThemeError> {
    let metadata = fs::symlink_metadata(root)
        .map_err(|error| ThemeError(format!("无法读取本地主题目录：{error}")))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ThemeError("请选择包含 manifest.json 的主题目录".into()));
    }

    let mut files = HashMap::new();
    let mut total_size = 0_u64;
    read_development_entries(root, root, 0, &mut files, &mut total_size)?;
    if !files.contains_key("manifest.json") {
        return Err(ThemeError("本地主题目录缺少 manifest.json".into()));
    }
    Ok(files)
}

fn read_development_entries(
    root: &Path,
    directory: &Path,
    depth: usize,
    files: &mut HashMap<String, Vec<u8>>,
    total_size: &mut u64,
) -> Result<(), ThemeError> {
    if depth > 16 {
        return Err(ThemeError("本地主题目录层级超过 16 层".into()));
    }
    for entry in fs::read_dir(directory)
        .map_err(|error| ThemeError(format!("无法读取本地主题目录：{error}")))?
    {
        let entry = entry.map_err(|error| ThemeError(format!("无法读取本地主题文件：{error}")))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| ThemeError(format!("无法读取本地主题文件：{error}")))?;
        if metadata.file_type().is_symlink() {
            return Err(ThemeError("本地主题目录不能包含符号链接".into()));
        }
        if metadata.is_dir() {
            read_development_entries(root, &path, depth + 1, files, total_size)?;
            continue;
        }
        if !metadata.is_file() {
            return Err(ThemeError("本地主题目录包含不支持的文件类型".into()));
        }
        if path.file_name().and_then(|name| name.to_str()) == Some(".DS_Store") {
            continue;
        }
        if files.len() >= MAX_FILE_COUNT {
            return Err(ThemeError("本地主题文件数量超过 256".into()));
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| ThemeError("本地主题文件越过所选目录".into()))?;
        let package_path = normalize_package_path(relative)?;
        validate_package_file(&package_path, metadata.len())?;
        *total_size = total_size
            .checked_add(metadata.len())
            .ok_or_else(|| ThemeError("本地主题大小溢出".into()))?;
        if *total_size > MAX_EXTRACTED_SIZE {
            return Err(ThemeError("本地主题总大小超过 60 MB".into()));
        }
        let source = fs::read(&path)
            .map_err(|error| ThemeError(format!("无法读取本地主题文件 {package_path}：{error}")))?;
        if source.len() as u64 != metadata.len() {
            return Err(ThemeError(format!(
                "本地主题文件读取期间发生变化：{package_path}"
            )));
        }
        files.insert(package_path, source);
    }
    Ok(())
}

fn verify_integrity_and_signature(
    files: &HashMap<String, Vec<u8>>,
    verifying_key: &VerifyingKey,
) -> Result<(), ThemeError> {
    let integrity_source = files
        .get("integrity.json")
        .ok_or_else(|| ThemeError("主题包缺少 integrity.json".into()))?;
    let integrity: IntegrityIndex = serde_json::from_slice(integrity_source)
        .map_err(|error| ThemeError(format!("integrity.json 无效：{error}")))?;
    if integrity.algorithm != "sha256" {
        return Err(ThemeError("完整性算法必须为 sha256".into()));
    }
    let expected_files: HashSet<&str> = files
        .keys()
        .filter(|path| *path != "integrity.json" && *path != "signature.ed25519")
        .map(String::as_str)
        .collect();
    let indexed_files: HashSet<&str> = integrity.files.keys().map(String::as_str).collect();
    if expected_files != indexed_files {
        return Err(ThemeError(
            "integrity.json 必须完整且仅覆盖主题内容文件".into(),
        ));
    }
    for (path, expected_digest) in &integrity.files {
        if !is_sha256(expected_digest) {
            return Err(ThemeError(format!("完整性摘要格式无效：{path}")));
        }
        if sha256_hex(&files[path]) != *expected_digest {
            return Err(ThemeError(format!("主题文件完整性校验失败：{path}")));
        }
    }
    let signature_source = files
        .get("signature.ed25519")
        .ok_or_else(|| ThemeError("主题包缺少 signature.ed25519".into()))?;
    let signature = Signature::from_slice(signature_source)
        .map_err(|_| ThemeError("Ed25519 签名必须是 64 字节".into()))?;
    verifying_key
        .verify_strict(integrity_source, &signature)
        .map_err(|_| ThemeError("主题平台签名验证失败".into()))
}

fn validate_css(source: &str, theme_id: &str) -> Result<(), ThemeError> {
    let normalized = source.to_ascii_lowercase();
    if normalized.contains("--ct-font") {
        return Err(ThemeError("主题必须使用 Codex 默认字体".into()));
    }
    for forbidden in [
        "@import",
        "url(",
        "http://",
        "https://",
        "javascript:",
        "expression(",
        "</style",
        "data-app-",
    ] {
        if normalized.contains(forbidden) {
            return Err(ThemeError(format!("主题 CSS 包含禁止内容：{forbidden}")));
        }
    }
    let sheet = StyleSheet::parse(source, ParserOptions::default())
        .map_err(|error| ThemeError(format!("主题 CSS 语法无效：{error}")))?;
    validate_css_rules(&sheet.rules, theme_id)
}

fn validate_css_rules(rules: &CssRuleList<'_>, theme_id: &str) -> Result<(), ThemeError> {
    for rule in &rules.0 {
        match rule {
            CssRule::Style(style) => {
                let selectors = style
                    .selectors
                    .to_css_string(PrinterOptions::default())
                    .map_err(|error| ThemeError(format!("主题选择器无效：{error}")))?;
                validate_selectors(&selectors, theme_id)?;
                if style
                    .declarations
                    .declarations
                    .iter()
                    .chain(&style.declarations.important_declarations)
                    .any(|property| match property.property_id() {
                        PropertyId::Font | PropertyId::FontFamily => true,
                        PropertyId::Custom(name) => name.as_ref().contains("font"),
                        _ => false,
                    })
                {
                    return Err(ThemeError("主题必须使用 Codex 默认字体".into()));
                }
                if !style.rules.0.is_empty() {
                    return Err(ThemeError("主题 CSS 暂不允许嵌套规则".into()));
                }
            }
            CssRule::Media(media) => validate_css_rules(&media.rules, theme_id)?,
            CssRule::Keyframes(keyframes) => {
                let name = keyframes
                    .name
                    .to_css_string(PrinterOptions::default())
                    .map_err(|error| ThemeError(format!("主题动画名称无效：{error}")))?;
                if !name.starts_with("ct-") {
                    return Err(ThemeError("主题动画名称必须使用 ct- 前缀".into()));
                }
                if keyframes.keyframes.iter().any(|keyframe| {
                    keyframe
                        .declarations
                        .declarations
                        .iter()
                        .chain(&keyframe.declarations.important_declarations)
                        .any(|property| match property.property_id() {
                            PropertyId::Font | PropertyId::FontFamily => true,
                            PropertyId::Custom(name) => name.as_ref().contains("font"),
                            _ => false,
                        })
                }) {
                    return Err(ThemeError("主题必须使用 Codex 默认字体".into()));
                }
            }
            CssRule::Ignored => {}
            _ => {
                return Err(ThemeError(
                    "主题 CSS 只允许样式规则、@media 和 @keyframes".into(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_selectors(selectors: &str, theme_id: &str) -> Result<(), ThemeError> {
    let expected_root = format!(r#":root[data-ct-theme="{theme_id}"]"#);
    for selector in selectors.split(',').map(str::trim) {
        let Some(scoped_selector) = selector.strip_prefix(&expected_root) else {
            return Err(ThemeError(format!(
                "主题选择器未限定到自身作用域：{selector}"
            )));
        };
        if scoped_selector.contains('#') {
            return Err(ThemeError(format!("主题选择器禁止使用 ID：{selector}")));
        }
        let selector_without_attributes = strip_selector_attributes(scoped_selector);
        for class in selector_without_attributes.split('.').skip(1).map(|part| {
            part.split(|character: char| {
                !character.is_ascii_alphanumeric() && character != '-' && character != '_'
            })
            .next()
            .unwrap_or("")
        }) {
            if !matches!(
                class,
                "ct-home-hero__copy"
                    | "ct-home-hero__eyebrow"
                    | "ct-home-hero__title"
                    | "ct-home-hero__description"
                    | "ct-home-hero__image"
                    | "ct-decoration"
            ) {
                return Err(ThemeError(format!("主题选择器使用了非平台类名：.{class}")));
            }
        }
        validate_selector_attributes(selector, theme_id)?;
    }
    Ok(())
}

fn validate_selector_attributes(selector: &str, theme_id: &str) -> Result<(), ThemeError> {
    let mut remaining = selector;
    while let Some(start) = remaining.find('[') {
        let after_start = &remaining[start + 1..];
        let end = after_start
            .find(']')
            .ok_or_else(|| ThemeError("主题选择器属性未闭合".into()))?;
        let attribute = &after_start[..end];
        let valid = attribute == format!(r#"data-ct-theme="{theme_id}""#)
            || parse_attribute_value(attribute, "data-ct-color-scheme")
                .is_some_and(|value| matches!(value, "light" | "dark"))
            || parse_attribute_value(attribute, "data-ct-view").is_some_and(|value| {
                matches!(value, "home" | "home-compact" | "conversation" | "other")
            })
            || parse_attribute_value(attribute, "data-ct-slot")
                .is_some_and(|value| ALLOWED_SLOTS.contains(&value))
            || parse_attribute_value(attribute, "data-ct-mount")
                .is_some_and(|value| ALLOWED_MOUNTS.contains(&value))
            || matches!(attribute, r#"role="button""# | r#"role="switch""#);
        if !valid {
            return Err(ThemeError(format!(
                "主题选择器属性不在白名单：[{attribute}]"
            )));
        }
        remaining = &after_start[end + 1..];
    }
    Ok(())
}

fn parse_attribute_value<'a>(attribute: &'a str, name: &str) -> Option<&'a str> {
    attribute
        .strip_prefix(name)?
        .strip_prefix("=\"")?
        .strip_suffix('"')
}

fn strip_selector_attributes(selector: &str) -> String {
    let mut result = String::with_capacity(selector.len());
    let mut depth = 0_u32;
    for character in selector.chars() {
        match character {
            '[' => depth += 1,
            ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => result.push(character),
            _ => {}
        }
    }
    result
}

fn validate_text(label: &str, value: &str, max_length: usize) -> Result<(), ThemeError> {
    let length = value.chars().count();
    if value.trim().is_empty() || length > max_length {
        return Err(ThemeError(format!("{label} 必须为 1–{max_length} 个字符")));
    }
    Ok(())
}

fn validate_style_path(path: &str) -> Result<(), ThemeError> {
    if !path.starts_with("styles/") || !path.ends_with(".css") || !is_safe_relative_path(path) {
        return Err(ThemeError(format!("主题样式路径无效：{path}")));
    }
    Ok(())
}

fn validate_asset_path(path: &str) -> Result<(), ThemeError> {
    if !path.starts_with("assets/")
        || !is_safe_relative_path(path)
        || !matches!(
            extension(path),
            Some("svg" | "png" | "jpg" | "jpeg" | "webp")
        )
    {
        return Err(ThemeError(format!("主题资源路径无效：{path}")));
    }
    Ok(())
}

fn validate_image<'a>(path: &str, source: &[u8]) -> Result<&'a str, ThemeError> {
    match extension(path) {
        Some("svg") => {
            let source = std::str::from_utf8(source)
                .map_err(|error| ThemeError(format!("SVG 不是 UTF-8：{error}")))?;
            validate_svg(source)?;
            Ok("image/svg+xml")
        }
        Some("png") if source.starts_with(b"\x89PNG\r\n\x1a\n") => Ok("image/png"),
        Some("jpg" | "jpeg") if source.starts_with(b"\xff\xd8\xff") => Ok("image/jpeg"),
        Some("webp")
            if source.len() >= 12 && &source[..4] == b"RIFF" && &source[8..12] == b"WEBP" =>
        {
            Ok("image/webp")
        }
        _ => Err(ThemeError(format!("图片内容与扩展名不匹配：{path}"))),
    }
}

fn validate_svg(source: &str) -> Result<(), ThemeError> {
    let normalized = source.to_ascii_lowercase();
    if !normalized.contains("<svg") {
        return Err(ThemeError("SVG 资源缺少根节点".into()));
    }
    let content = normalized.replace("http://www.w3.org/2000/svg", "");
    for forbidden in [
        "<script",
        "<foreignobject",
        "javascript:",
        "http://",
        "https://",
        "onload=",
        "onclick=",
    ] {
        if content.contains(forbidden) {
            return Err(ThemeError(format!("SVG 资源包含禁止内容：{forbidden}")));
        }
    }
    Ok(())
}

fn validate_package_file(path: &str, size: u64) -> Result<(), ThemeError> {
    let allowed = matches!(
        path,
        "manifest.json" | "compatibility.json" | "integrity.json" | "signature.ed25519"
    ) || path.starts_with("styles/") && extension(path) == Some("css")
        || path.starts_with("assets/")
            && matches!(
                extension(path),
                Some("svg" | "png" | "jpg" | "jpeg" | "webp")
            );
    if !allowed {
        return Err(ThemeError(format!("主题包包含不允许的文件：{path}")));
    }
    let limit = if path.starts_with("assets/") {
        MAX_IMAGE_SIZE
    } else {
        1024 * 1024
    };
    if size > limit {
        return Err(ThemeError(format!("主题文件超过大小限制：{path}")));
    }
    Ok(())
}

fn normalize_package_path(path: &Path) -> Result<String, ThemeError> {
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ThemeError("主题包路径必须是规范相对路径".into()));
    }
    let value = path
        .to_str()
        .ok_or_else(|| ThemeError("主题包路径必须是 UTF-8".into()))?
        .replace('\\', "/");
    if !is_safe_relative_path(&value) {
        return Err(ThemeError(format!("主题包路径无效：{value}")));
    }
    Ok(value)
}

fn is_safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

fn utf8_file<'a>(files: &'a HashMap<String, Vec<u8>>, path: &str) -> Result<&'a str, ThemeError> {
    let source = files
        .get(path)
        .ok_or_else(|| ThemeError(format!("主题缺少文件：{path}")))?;
    std::str::from_utf8(source).map_err(|error| ThemeError(format!("{path} 不是 UTF-8：{error}")))
}

fn write_atomic(path: &Path, source: &[u8]) -> Result<(), ThemeError> {
    let parent = path
        .parent()
        .ok_or_else(|| ThemeError("主题存储路径无效".into()))?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(source)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| ThemeError(format!("无法原子写入主题存储：{}", error.error)))?;
    Ok(())
}

fn platform_verifying_key() -> Result<VerifyingKey, ThemeError> {
    let mut bytes = [0_u8; 32];
    hex::decode_to_slice(security_config::theme_public_key(), &mut bytes)
        .map_err(|error| ThemeError(format!("内置平台公钥无效：{error}")))?;
    VerifyingKey::from_bytes(&bytes).map_err(|_| ThemeError("内置平台公钥无效".into()))
}

fn sha256_hex(source: &[u8]) -> String {
    hex::encode(Sha256::digest(source))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn extension(path: &str) -> Option<&str> {
    path.rsplit_once('.').map(|(_, extension)| extension)
}

fn is_valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.contains('.')
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
}

fn is_valid_locale(value: &str) -> bool {
    let mut parts = value.split('-');
    let language = parts.next().unwrap_or_default();
    (2..=3).contains(&language.len())
        && language.bytes().all(|byte| byte.is_ascii_lowercase())
        && parts.all(|part| {
            (part.len() == 2 && part.bytes().all(|byte| byte.is_ascii_uppercase()))
                || (part.len() == 4
                    && part.as_bytes()[0].is_ascii_uppercase()
                    && part.as_bytes()[1..]
                        .iter()
                        .all(|byte| byte.is_ascii_lowercase()))
        })
}

fn validate_optional_text(
    label: &str,
    value: Option<&str>,
    max_length: usize,
) -> Result<(), ThemeError> {
    if let Some(value) = value {
        validate_text(label, value, max_length)?;
    }
    Ok(())
}

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    const TEST_SECRET: [u8; 32] = [7; 32];

    #[test]
    fn codex_tested_versions_are_optional_and_accept_legacy_field() {
        let mut manifest: Value = serde_json::from_str(TEST_THEME_MANIFEST).expect("test manifest");
        manifest
            .as_object_mut()
            .expect("manifest object")
            .remove("testedCodexVersions");
        let without_versions: ThemeManifest =
            serde_json::from_value(manifest.clone()).expect("versions are optional");
        assert!(without_versions._tested_codex_versions.is_empty());

        manifest["supportedCodexVersions"] = json!(["26.707.91948"]);
        let legacy: ThemeManifest =
            serde_json::from_value(manifest).expect("legacy version field remains readable");
        assert_eq!(legacy._tested_codex_versions, ["26.707.91948"]);
    }

    #[test]
    fn home_prompt_and_conversation_banner_are_optional() {
        let mut manifest: Value = serde_json::from_str(TEST_THEME_MANIFEST).expect("test manifest");
        manifest["experience"]
            .as_object_mut()
            .expect("experience object")
            .retain(|key, _| {
                !matches!(key.as_str(), "homePrompt" | "conversationBanner" | "assets")
            });
        assert!(manifest["experience"].get("homePrompt").is_none());
        assert!(manifest["experience"].get("conversationBanner").is_none());
        let manifest: ThemeManifest =
            serde_json::from_value(manifest).expect("optional experience fields");
        validate_manifest(&manifest).expect("legacy theme remains valid");
        let files = test_theme_files();
        let config = build_runtime_config(&manifest, &files).expect("runtime config");
        assert!(config["homePrompt"].is_null());
        assert!(config["conversationBanner"].is_null());
    }

    #[test]
    fn builds_home_prompt_and_conversation_banner_runtime_config() {
        let mut manifest: Value = serde_json::from_str(TEST_THEME_MANIFEST).expect("test manifest");
        manifest["experience"]
            .as_object_mut()
            .expect("experience object")
            .remove("assets");
        manifest["experience"]["homePrompt"] = json!({
            "title": "今天想一起做什么？"
        });
        manifest["experience"]["conversationBanner"] = json!({
            "eyebrow": "SESSION",
            "title": "保持专注",
            "description": "会话页使用独立窄图。",
            "asset": "assets/banner-chat.svg",
            "fit": "contain",
            "position": "right"
        });
        let manifest: ThemeManifest = serde_json::from_value(manifest).expect("theme manifest");
        validate_manifest(&manifest).expect("experience fields should be valid");
        let files = test_theme_files();
        let config = build_runtime_config(&manifest, &files).expect("runtime config");
        assert_eq!(config["homePrompt"]["title"], "今天想一起做什么？");
        assert_eq!(config["conversationBanner"]["fit"], "contain");
        assert_eq!(config["conversationBanner"]["position"], "right");
        assert!(
            config["conversationBanner"]["assetUrl"]
                .as_str()
                .is_some_and(|value| value == "assets/banner-chat.svg")
        );
    }

    #[test]
    fn builds_controlled_asset_slot_runtime_config() {
        let mut manifest: Value = serde_json::from_str(TEST_THEME_MANIFEST).expect("test manifest");
        manifest["experience"]["assets"] = json!([
            { "slot": "app.background", "asset": "assets/hero.svg" },
            { "slot": "main.background", "asset": "assets/hero.svg" },
            { "slot": "main.overlay", "asset": "assets/ornament.svg" },
            { "slot": "main.frame", "asset": "assets/ornament.svg" },
            { "slot": "sidebar.brand.icon", "asset": "assets/ornament.svg" },
            { "slot": "sidebar.brand.badge", "asset": "assets/ornament.svg" },
            { "slot": "sidebar.header.decoration", "asset": "assets/ornament.svg" },
            { "slot": "sidebar.frame", "asset": "assets/ornament.svg" },
            { "slot": "home.card.arrow.asset", "asset": "assets/ornament.svg" }
        ]);
        manifest["slots"] = json!(
            manifest["slots"]
                .as_array()
                .expect("slots")
                .iter()
                .cloned()
                .chain(ALLOWED_ASSET_SLOTS.map(Value::from))
                .collect::<Vec<_>>()
        );
        let manifest: ThemeManifest = serde_json::from_value(manifest).expect("theme manifest");
        validate_manifest(&manifest).expect("controlled asset slots should be valid");
        let files = test_theme_files();
        let config = build_runtime_config(&manifest, &files).expect("runtime config");
        assert_eq!(config["assets"].as_array().map(Vec::len), Some(9));
        assert_eq!(config["assets"][0]["slot"], "app.background");
        assert_eq!(config["assets"][6]["slot"], "sidebar.header.decoration");
        assert_eq!(config["assets"][7]["slot"], "sidebar.frame");
        assert!(
            config["assets"][8]["assetUrl"]
                .as_str()
                .is_some_and(|value| value == "assets/ornament.svg")
        );
    }

    #[test]
    fn replaces_every_runtime_asset_path_with_session_urls() {
        let theme = test_theme_package().expect("test theme");
        let assets = theme.runtime_assets();
        assert!(!assets.is_empty());
        let urls = assets
            .iter()
            .map(|asset| {
                (
                    asset.path.clone(),
                    format!("http://127.0.0.1:49152/token/{}", asset.path),
                )
            })
            .collect::<HashMap<_, _>>();
        let config = theme
            .runtime_config_with_asset_urls(&urls)
            .expect("session runtime config");
        assert_runtime_urls_are_local(&config);
        assert_eq!(
            config["conversationBanner"]["assetUrl"],
            urls["assets/banner-chat.svg"]
        );
    }

    fn assert_runtime_urls_are_local(value: &Value) {
        match value {
            Value::Object(object) => {
                for (key, value) in object {
                    if matches!(key.as_str(), "assetUrl" | "foregroundAssetUrl") {
                        if let Some(url) = value.as_str() {
                            assert!(url.starts_with("http://127.0.0.1:"), "invalid URL: {url}");
                        }
                    } else {
                        assert_runtime_urls_are_local(value);
                    }
                }
            }
            Value::Array(values) => {
                for value in values {
                    assert_runtime_urls_are_local(value);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn rejects_unknown_or_duplicate_asset_slots() {
        let mut manifest: Value = serde_json::from_str(TEST_THEME_MANIFEST).expect("test manifest");
        manifest["experience"]["assets"] = json!([
            { "slot": "main.unknown", "asset": "assets/ornament.svg" }
        ]);
        let unknown: ThemeManifest = serde_json::from_value(manifest.clone()).expect("manifest");
        assert!(
            validate_manifest(&unknown)
                .expect_err("unknown asset slot must fail")
                .to_string()
                .contains("未开放")
        );

        manifest["experience"]["assets"] = json!([
            { "slot": "sidebar.frame", "asset": "assets/ornament.svg" }
        ]);
        let undeclared: ThemeManifest = serde_json::from_value(manifest.clone()).expect("manifest");
        assert!(
            validate_manifest(&undeclared)
                .expect_err("undeclared asset slot must fail")
                .to_string()
                .contains("slots")
        );

        manifest["slots"]
            .as_array_mut()
            .expect("slots")
            .push(json!("sidebar.frame"));
        manifest["experience"]["assets"] = json!([
            { "slot": "sidebar.frame", "asset": "assets/ornament.svg" },
            { "slot": "sidebar.frame", "asset": "assets/ornament.svg" }
        ]);
        let duplicate: ThemeManifest = serde_json::from_value(manifest).expect("manifest");
        assert!(
            validate_manifest(&duplicate)
                .expect_err("duplicate asset slot must fail")
                .to_string()
                .contains("重复")
        );
    }

    #[test]
    fn rejects_invalid_home_prompt_and_conversation_banner() {
        let mut manifest: Value = serde_json::from_str(TEST_THEME_MANIFEST).expect("test manifest");
        manifest["experience"]["homePrompt"] = json!({ "title": "" });
        let manifest_with_empty_prompt: ThemeManifest =
            serde_json::from_value(manifest.clone()).expect("theme manifest");
        assert!(
            validate_manifest(&manifest_with_empty_prompt)
                .expect_err("empty prompt must fail")
                .to_string()
                .contains("Home prompt title")
        );

        manifest["experience"]["homePrompt"] = json!({ "title": "Valid" });
        manifest["experience"]["conversationBanner"] = json!({
            "eyebrow": "SESSION",
            "title": "Conversation",
            "description": "Independent narrow banner",
            "asset": "assets/banner-chat.svg",
            "fit": "stretch",
            "position": "center"
        });
        let manifest_with_invalid_fit: ThemeManifest =
            serde_json::from_value(manifest.clone()).expect("theme manifest");
        assert!(
            validate_manifest(&manifest_with_invalid_fit)
                .expect_err("invalid fit must fail")
                .to_string()
                .contains("fit")
        );

        manifest["experience"]["conversationBanner"]["fit"] = json!("cover");
        manifest["experience"]["conversationBanner"]["position"] = json!("center 20%");
        let manifest_with_invalid_position: ThemeManifest =
            serde_json::from_value(manifest).expect("theme manifest");
        assert!(
            validate_manifest(&manifest_with_invalid_position)
                .expect_err("invalid position must fail")
                .to_string()
                .contains("position")
        );
    }

    #[test]
    fn installs_and_loads_signed_online_theme() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let package = signed_online_test_package();
        let signing_key = SigningKey::from_bytes(&TEST_SECRET);
        let repository = ThemeRepository {
            root: directory.path().join("repository"),
            verifying_key: signing_key.verifying_key(),
            cache_key: Some([9; 32]),
        };
        let installed_before = repository.list().expect("list themes before install").len();

        let report = repository
            .install_online(&package, "test-theme".into())
            .expect("install signed theme");

        assert!(report.signature_verified);
        assert_eq!(report.theme.id, "studio.example.test-theme");
        assert!(!report.theme.built_in);
        assert_eq!(
            repository.list().expect("list themes").len(),
            installed_before + 1
        );
        let installed = repository.read_registry().expect("registry");
        let digest = installed
            .themes
            .get("studio.example.test-theme")
            .expect("installed theme")
            .digest
            .clone();
        let cache_path = repository.store_path(&digest);
        assert!(cache_path.is_file());

        assert!(
            repository
                .uninstall("studio.example.test-theme")
                .expect("uninstall signed theme")
        );
        assert_eq!(
            repository.list().expect("list themes").len(),
            installed_before
        );
        assert!(!cache_path.exists());
    }

    #[test]
    fn online_theme_uses_device_encrypted_cache() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let package = signed_online_test_package();
        let signing_key = SigningKey::from_bytes(&TEST_SECRET);
        let repository = ThemeRepository {
            root: directory.path().join("repository"),
            verifying_key: signing_key.verifying_key(),
            cache_key: Some([9; 32]),
        };

        repository
            .install_online(&package, "test-theme".into())
            .expect("authorized online install");
        let registry = repository.read_registry().expect("registry");
        let installed = registry
            .themes
            .get("studio.example.test-theme")
            .expect("installed theme");
        let stored = fs::read(repository.store_path(&installed.digest)).expect("cache");
        assert!(stored.starts_with(ONLINE_CACHE_MAGIC));
        assert!(
            !stored
                .windows(package.len())
                .any(|window| window == package)
        );
        repository
            .load("studio.example.test-theme")
            .expect("device-bound cache loads");

        let mut registry = repository.read_registry().expect("registry");
        registry
            .themes
            .get_mut("studio.example.test-theme")
            .expect("installed theme")
            .online
            .slug = "different-theme".into();
        write_atomic(
            &repository.registry_path(),
            &serde_json::to_vec_pretty(&registry).expect("serialize registry"),
        )
        .expect("tamper registry");
        let error = repository
            .load("studio.example.test-theme")
            .expect_err("registry source tampering must fail");
        assert!(error.to_string().contains("授权策略与服务端不一致"));
    }

    #[test]
    fn loads_unsigned_development_directory_without_installing_it() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let theme_path = directory.path().join("development-theme");
        fs::create_dir_all(theme_path.join("styles")).expect("styles directory");
        fs::create_dir_all(theme_path.join("assets")).expect("assets directory");
        fs::write(theme_path.join("manifest.json"), TEST_THEME_MANIFEST)
            .expect("development manifest");
        fs::write(theme_path.join("styles/theme.css"), TEST_THEME_CSS).expect("development CSS");
        for asset in ["hero.svg", "banner-chat.svg", "ornament.svg"] {
            fs::write(theme_path.join("assets").join(asset), TEST_THEME_SVG)
                .expect("development asset");
        }
        let repository =
            ThemeRepository::new(directory.path().join("repository")).expect("theme repository");
        let installed_before = repository
            .list()
            .expect("installed themes before load")
            .len();

        let package = repository
            .load_development(&theme_path)
            .expect("development theme should load");

        assert_eq!(package.id(), "studio.example.test-theme");
        assert_eq!(
            repository.list().expect("installed themes").len(),
            installed_before
        );
        assert!(!directory.path().join("repository/registry.json").exists());
    }

    #[test]
    fn documented_example_theme_matches_the_runtime_protocol() {
        let repository_root = tempfile::tempdir().expect("temporary repository");
        let repository =
            ThemeRepository::new(repository_root.path().to_path_buf()).expect("theme repository");
        let theme_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/theme-example");

        let package = repository
            .load_development(&theme_path)
            .expect("documented example theme should remain loadable");

        assert_eq!(package.id(), "studio.example.protocol-preview");
        assert_eq!(package.version(), "1.0.0");
        assert_eq!(package.runtime_assets().len(), 1);
    }

    #[test]
    fn development_directory_keeps_package_file_restrictions() {
        let directory = tempfile::tempdir().expect("temporary directory");
        fs::write(directory.path().join("manifest.json"), TEST_THEME_MANIFEST)
            .expect("development manifest");
        fs::write(directory.path().join("unsafe.js"), "alert(1)").expect("unsafe file");
        let repository =
            ThemeRepository::new(directory.path().join("repository")).expect("theme repository");

        let error = repository
            .load_development(directory.path())
            .expect_err("unexpected files must fail");

        assert!(error.to_string().contains("不允许的文件"));
    }

    #[test]
    fn development_loader_rejects_ctheme_files() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let package_path = directory.path().join("theme.ctheme");
        fs::write(&package_path, signed_test_package(false)).expect("write package");
        let repository =
            ThemeRepository::new(directory.path().join("repository")).expect("theme repository");

        let error = repository
            .load_development(&package_path)
            .expect_err("packed themes are not development directories");

        assert!(
            error
                .to_string()
                .contains("请选择包含 manifest.json 的主题目录")
        );
    }

    #[test]
    fn rejects_tampered_package() {
        let files = read_archive(&signed_test_package(true)).expect("read archive");
        let signing_key = SigningKey::from_bytes(&TEST_SECRET);
        let error = verify_integrity_and_signature(&files, &signing_key.verifying_key())
            .expect_err("tampering must fail");
        assert!(error.to_string().contains("完整性校验失败"));
    }

    #[test]
    fn rejects_same_version_with_different_content() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let signing_key = SigningKey::from_bytes(&TEST_SECRET);
        let repository = ThemeRepository {
            root: directory.path().join("repository"),
            verifying_key: signing_key.verifying_key(),
            cache_key: Some([9; 32]),
        };
        repository
            .install_online(&signed_online_test_package(), "test-theme".into())
            .expect("first install");

        let changed_package =
            signed_online_test_package_with_description("Changed package content");
        let error = repository
            .install_online(&changed_package, "test-theme".into())
            .expect_err("same version replacement must fail");

        assert!(error.to_string().contains("不可覆盖"));
    }

    #[test]
    fn rejects_archive_path_traversal() {
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(&mut cursor);
        writer
            .start_file("../escape.css", SimpleFileOptions::default())
            .expect("start unsafe entry");
        writer.write_all(b"x").expect("write unsafe entry");
        writer.finish().expect("finish archive");
        let error = read_archive(&cursor.into_inner()).expect_err("unsafe path must fail");
        assert!(error.to_string().contains("路径"));
    }

    #[test]
    fn rejects_unscoped_css() {
        let error = validate_css("body { color: red; }", "studio.example.theme")
            .expect_err("unscoped CSS should fail");
        assert!(error.to_string().contains("作用域"));
    }

    #[test]
    fn rejects_theme_font_overrides() {
        for css in [
            r#":root[data-ct-theme="studio.example.theme"] { font-family: serif; }"#,
            r#":root[data-ct-theme="studio.example.theme"] { --theme-font: serif; }"#,
        ] {
            let error = validate_css(css, "studio.example.theme")
                .expect_err("theme font override should fail");
            assert!(error.to_string().contains("默认字体"));
        }
    }

    #[test]
    fn accepts_namespaced_keyframes_and_rejects_global_animation_names() {
        validate_css(
            r#"@keyframes ct-theme-float { from { opacity: 0; } to { opacity: 1; } }
               :root[data-ct-theme="studio.example.theme"] { animation: ct-theme-float 1s; }"#,
            "studio.example.theme",
        )
        .expect("namespaced theme keyframes should be allowed");
        let error = validate_css(
            "@keyframes float { from { opacity: 0; } to { opacity: 1; } }",
            "studio.example.theme",
        )
        .expect_err("global animation names should fail");
        assert!(error.to_string().contains("ct- 前缀"));
    }

    #[test]
    fn accepts_only_supported_color_schemes() {
        validate_css(
            r#":root[data-ct-theme="studio.example.theme"][data-ct-color-scheme="light"] { color: black; }"#,
            "studio.example.theme",
        )
        .expect("light scheme should be allowed");
        let error = validate_css(
            r#":root[data-ct-theme="studio.example.theme"][data-ct-color-scheme="system"] { color: black; }"#,
            "studio.example.theme",
        )
        .expect_err("unknown scheme should fail");
        assert!(error.to_string().contains("白名单"));
    }

    #[test]
    fn accepts_only_supported_runtime_views() {
        validate_css(
            r#":root[data-ct-theme="studio.example.theme"][data-ct-view="home-compact"] { color: black; }"#,
            "studio.example.theme",
        )
        .expect("compact home view should be allowed");
        let error = validate_css(
            r#":root[data-ct-theme="studio.example.theme"][data-ct-view="custom"] { color: black; }"#,
            "studio.example.theme",
        )
        .expect_err("unknown runtime view should fail");
        assert!(error.to_string().contains("白名单"));
    }

    #[test]
    fn accepts_page_menu_and_detailed_settings_slots() {
        for slot in [
            "page.surface",
            "page.header",
            "page.content",
            "menu.item.active",
            "menu.item.checked",
            "menu.separator",
            "settings.surface",
            "settings.body",
            "settings.section.title",
            "settings.card",
            "settings.row.title",
            "settings.row.description",
            "settings.row.separator",
            "settings.switch.checked",
            "settings.switch.track.checked",
            "settings.switch.thumb",
            "main.content.frame",
            "home.layout",
            "composer.region",
            "composer.backdrop",
            "conversation.stage",
            "conversation.header",
            "conversation.header.content",
            "conversation.viewport",
            "conversation.summary.region",
            "conversation.summary",
            "conversation.summary.decoration",
        ] {
            validate_css(
                &format!(
                    r#":root[data-ct-theme="studio.example.theme"] [data-ct-slot="{slot}"] {{ color: black; }}"#
                ),
                "studio.example.theme",
            )
            .unwrap_or_else(|error| panic!("{slot} should be allowed: {error}"));
        }
    }

    #[test]
    fn accepts_controlled_asset_mount_selectors() {
        for slot in ALLOWED_ASSET_SLOTS {
            validate_css(
                &format!(
                    r#":root[data-ct-theme="studio.example.theme"] [data-ct-mount="{slot}"] {{ pointer-events: none; }}"#
                ),
                "studio.example.theme",
            )
            .unwrap_or_else(|error| panic!("asset mount {slot} should be allowed: {error}"));
        }
        validate_css(
            r#":root[data-ct-theme="studio.example.theme"] [data-ct-mount="app.background"] > img { object-fit: cover; object-position: center; opacity: 0.8; }"#,
            "studio.example.theme",
        )
        .expect("app background image presentation should be themeable");
    }

    #[test]
    fn rejects_active_svg_content() {
        let error =
            validate_svg(r#"<svg xmlns="http://www.w3.org/2000/svg" onload="alert(1)"></svg>"#)
                .expect_err("active SVG content should be rejected");
        assert!(error.to_string().contains("onload="));
    }

    fn signed_test_package(tamper: bool) -> Vec<u8> {
        let package = signed_test_package_with_description("A signed test theme.");
        if !tamper {
            return package;
        }
        let files = read_archive(&package).expect("read signed package");
        build_test_archive(files.into_iter().map(|(path, source)| {
            if path == "styles/theme.css" {
                (path, b"tampered".to_vec())
            } else {
                (path, source)
            }
        }))
    }

    fn signed_test_package_with_description(description: &str) -> Vec<u8> {
        signed_test_package_with_access(description, None)
    }

    fn signed_online_test_package() -> Vec<u8> {
        signed_online_test_package_with_description("An online test theme.")
    }

    fn signed_online_test_package_with_description(description: &str) -> Vec<u8> {
        signed_test_package_with_access(
            description,
            Some(json!({
                "delivery": "online",
                "slug": "test-theme"
            })),
        )
    }

    fn signed_test_package_with_access(description: &str, access: Option<Value>) -> Vec<u8> {
        let manifest = format!(
            r##"{{
          "schemaVersion": 1,
          "id": "studio.example.test-theme",
          "name": "Test Theme",
          "description": "{description}",
          "version": "1.0.0",
          "author": {{ "id": "studio.example", "name": "Example Studio" }},
          "testedCodexVersions": ["26.707.91948"],
          "styles": ["styles/theme.css"],
          "slots": ["app.shell", "home.hero"],
          "permissions": [],{access}
          "preview": {{ "background": "#111111", "surface": "#222222", "accent": "#aaff44" }},
          "experience": {{
            "homeHero": {{
              "eyebrow": "TEST",
              "title": "Signed package",
              "description": "Installer verification",
              "asset": "assets/hero.svg",
              "fit": "cover",
              "position": "center"
            }},
            "decorations": []
          }},
          "integrity": "integrity.json",
          "signature": "signature.ed25519"
        }}"##,
            access = access
                .map(|value| format!("\n          \"access\": {value},"))
                .unwrap_or_default()
        );
        let css = r#":root[data-ct-theme="studio.example.test-theme"] [data-ct-slot="app.shell"] { color: white; }"#;
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0 0h10v10H0z"/></svg>"#;
        let mut files = BTreeMap::from([
            ("manifest.json".to_owned(), manifest.into_bytes()),
            ("styles/theme.css".to_owned(), css.as_bytes().to_vec()),
            ("assets/hero.svg".to_owned(), svg.as_bytes().to_vec()),
        ]);
        let integrity = IntegrityIndex {
            algorithm: "sha256".into(),
            files: files
                .iter()
                .map(|(path, source)| (path.clone(), sha256_hex(source)))
                .collect(),
        };
        let integrity_source = serde_json::to_vec(&integrity).expect("serialize integrity");
        let signing_key = SigningKey::from_bytes(&TEST_SECRET);
        let signature = signing_key.sign(&integrity_source).to_bytes().to_vec();
        files.insert("integrity.json".to_owned(), integrity_source);
        files.insert("signature.ed25519".to_owned(), signature);

        build_test_archive(files)
    }

    fn build_test_archive(files: impl IntoIterator<Item = (String, Vec<u8>)>) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(&mut cursor);
        for (path, source) in files {
            writer
                .start_file(path, SimpleFileOptions::default())
                .expect("start package file");
            writer.write_all(&source).expect("write package file");
        }
        writer.finish().expect("finish package");
        cursor.into_inner()
    }
}
