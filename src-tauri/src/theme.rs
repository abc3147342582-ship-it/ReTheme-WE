use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};
use ed25519_dalek::{Signature, VerifyingKey};
use rand_core::{OsRng, RngCore};
#[cfg(test)]
use retheme_theme_protocol::{
    ALLOWED_ASSET_SLOTS, ALLOWED_SLOTS, is_valid_author_id, is_valid_theme_id, validate_css,
    validate_manifest, validate_svg,
};
use retheme_theme_protocol::{
    MAX_ARCHIVE_SIZE, ThemeError, ThemeManifest, ThemePreview, read_archive,
    read_development_directory, validate_image, validate_package_files,
};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use crate::{security_config, wallpaper_engine::SceneWallpaperCapture};

const ONLINE_CACHE_MAGIC: &[u8; 4] = b"RTC1";
const MAX_WALLPAPER_PROJECT_JSON_SIZE: u64 = 1024 * 1024;
const MAX_LOCAL_WALLPAPER_VIDEO_SIZE: u64 = 16 * 1024 * 1024 * 1024;
const MAX_WALLPAPER_WEB_FILE_COUNT: usize = 4096;
const MAX_WALLPAPER_WEB_TOTAL_SIZE: u64 = 512 * 1024 * 1024;
pub(crate) const WALLPAPER_ASSET_PATH: &str = "wallpaper-engine/background-video";

#[derive(Debug, Clone)]
pub struct ThemePackage {
    manifest: ThemeManifest,
    css: String,
    runtime_assets: HashMap<String, ThemeRuntimeAsset>,
    wallpaper_asset: Option<String>,
    wallpaper_kind: Option<String>,
    wallpaper_file: Option<String>,
    wallpaper_scene: Option<Arc<SceneWallpaperCapture>>,
    wallpaper_controls: Option<WallpaperControls>,
}

#[derive(Debug, Clone)]
pub(crate) struct ThemeRuntimeAsset {
    pub path: String,
    pub mime: String,
    pub source: ThemeRuntimeAssetSource,
    pub route_path: Option<String>,
    pub policy: ThemeAssetPolicy,
}

#[derive(Debug, Clone)]
pub(crate) enum ThemeRuntimeAssetSource {
    Memory(Arc<[u8]>),
    File(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThemeAssetPolicy {
    Media,
    SandboxedWeb,
}

impl ThemeRuntimeAsset {
    pub(crate) fn from_memory(path: String, mime: String, source: Vec<u8>) -> Self {
        Self {
            path,
            mime,
            source: ThemeRuntimeAssetSource::Memory(Arc::from(source)),
            route_path: None,
            policy: ThemeAssetPolicy::Media,
        }
    }

    pub(crate) fn from_file(path: String, mime: String, source: PathBuf) -> Self {
        Self {
            path,
            mime,
            source: ThemeRuntimeAssetSource::File(source),
            route_path: None,
            policy: ThemeAssetPolicy::Media,
        }
    }

    pub(crate) fn from_web_file(
        path: String,
        mime: String,
        source: PathBuf,
        route_path: String,
    ) -> Self {
        Self {
            path,
            mime,
            source: ThemeRuntimeAssetSource::File(source),
            route_path: Some(route_path),
            policy: ThemeAssetPolicy::SandboxedWeb,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WallpaperEngineCatalog {
    pub root: PathBuf,
    pub projects: Vec<WallpaperEngineProject>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WallpaperControls {
    pub wallpaper_brightness: u8,
    pub interface_transparency: u8,
}

impl WallpaperControls {
    pub fn new(wallpaper_brightness: u8, interface_transparency: u8) -> Result<Self, ThemeError> {
        if wallpaper_brightness > 100 {
            return Err(ThemeError("壁纸亮度必须在 0 到 100 之间".into()));
        }
        if interface_transparency > 100 {
            return Err(ThemeError("界面透明度必须在 0 到 100 之间".into()));
        }
        Ok(Self {
            wallpaper_brightness,
            interface_transparency,
        })
    }
}

impl Default for WallpaperControls {
    fn default() -> Self {
        Self {
            wallpaper_brightness: 68,
            interface_transparency: 32,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WallpaperEngineProject {
    pub id: String,
    pub title: String,
    pub project_type: String,
    pub project_path: PathBuf,
    pub media_path: Option<PathBuf>,
    pub media_size_bytes: Option<u64>,
    pub requires_wallpaper_engine: bool,
    pub supported: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WallpaperEngineManifest {
    #[serde(default)]
    title: String,
    #[serde(default, rename = "type")]
    project_type: String,
    #[serde(default)]
    file: String,
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

    pub(crate) fn wallpaper_file_path(&self) -> Option<&Path> {
        let asset_path = self.wallpaper_file.as_ref()?;
        match &self.runtime_assets.get(asset_path)?.source {
            ThemeRuntimeAssetSource::File(path) => Some(path),
            ThemeRuntimeAssetSource::Memory(_) => None,
        }
    }

    pub(crate) fn wallpaper_scene(&self) -> Option<Arc<SceneWallpaperCapture>> {
        self.wallpaper_scene.clone()
    }

    pub(crate) fn wallpaper_controls(&self) -> Option<WallpaperControls> {
        self.wallpaper_controls
    }

    pub(crate) fn runtime_config_with_asset_urls(
        &self,
        asset_urls: &HashMap<String, String>,
    ) -> Result<Value, ThemeError> {
        let mut config = build_runtime_config_with(&self.manifest, |path| {
            asset_urls
                .get(path)
                .cloned()
                .ok_or_else(|| ThemeError(format!("主题资源 URL 缺失：{path}")))
        })?;
        if let (Some(path), Some(kind)) = (&self.wallpaper_asset, &self.wallpaper_kind) {
            let asset_url = asset_urls
                .get(path)
                .cloned()
                .ok_or_else(|| ThemeError(format!("动态壁纸资源 URL 缺失：{path}")))?;
            config["wallpaper"] = json!({
                "kind": kind,
                "assetUrl": asset_url,
                "fit": "cover",
                "position": "center",
                "brightness": self.wallpaper_controls.unwrap_or_default().wallpaper_brightness,
                "interfaceTransparency": self.wallpaper_controls.unwrap_or_default().interface_transparency,
            });
        }
        Ok(config)
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

pub fn wallpaper_engine_catalog() -> Result<WallpaperEngineCatalog, ThemeError> {
    scan_wallpaper_engine_root(&default_wallpaper_engine_root())
}

fn default_wallpaper_engine_root() -> PathBuf {
    let program_files = std::env::var_os("ProgramFiles(x86)")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"));
    program_files
        .join("Steam")
        .join("steamapps")
        .join("workshop")
        .join("content")
        .join("431960")
}

fn scan_wallpaper_engine_root(root: &Path) -> Result<WallpaperEngineCatalog, ThemeError> {
    let root = root.canonicalize().map_err(|error| {
        ThemeError(format!(
            "找不到 Wallpaper Engine Workshop 目录 {}：{error}",
            root.display()
        ))
    })?;
    if root.file_name().and_then(|name| name.to_str()) != Some("431960") {
        return Err(ThemeError(
            "Wallpaper Engine Workshop 目录必须以 431960 结尾".into(),
        ));
    }
    let entries = fs::read_dir(&root)
        .map_err(|error| ThemeError(format!("无法读取 {}：{error}", root.display())))?;
    let mut projects = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let project_path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        if !id.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let manifest = match read_wallpaper_engine_manifest(&project_path) {
            Ok(manifest) => manifest,
            Err(_) => continue,
        };
        projects.push(describe_wallpaper_engine_project(
            &project_path,
            id,
            manifest,
        ));
    }
    projects.sort_by(|left, right| {
        left.title
            .to_lowercase()
            .cmp(&right.title.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(WallpaperEngineCatalog { root, projects })
}

fn read_wallpaper_engine_manifest(
    project_path: &Path,
) -> Result<WallpaperEngineManifest, ThemeError> {
    let manifest_path = project_path.join("project.json");
    let metadata = fs::metadata(&manifest_path)
        .map_err(|error| ThemeError(format!("无法读取 {}：{error}", manifest_path.display())))?;
    if metadata.len() > MAX_WALLPAPER_PROJECT_JSON_SIZE {
        return Err(ThemeError(format!(
            "{} 超过 1 MiB，拒绝解析",
            manifest_path.display()
        )));
    }
    let source = fs::read(&manifest_path)
        .map_err(|error| ThemeError(format!("无法读取 {}：{error}", manifest_path.display())))?;
    serde_json::from_slice(&source)
        .map_err(|error| ThemeError(format!("{} 无效：{error}", manifest_path.display())))
}

fn describe_wallpaper_engine_project(
    project_path: &Path,
    id: String,
    manifest: WallpaperEngineManifest,
) -> WallpaperEngineProject {
    let project_type = manifest.project_type.to_ascii_lowercase();
    let requires_wallpaper_engine = project_type == "scene";
    let title = if manifest.title.trim().is_empty() {
        id.clone()
    } else {
        manifest.title.trim().to_owned()
    };
    if !matches!(project_type.as_str(), "video" | "scene" | "web") {
        return WallpaperEngineProject {
            id,
            title,
            project_type: project_type.clone(),
            project_path: project_path.to_path_buf(),
            media_path: None,
            media_size_bytes: None,
            requires_wallpaper_engine,
            supported: false,
            reason: Some("不支持的 Wallpaper Engine 项目类型".into()),
        };
    }

    let declared_file = if project_type == "scene" {
        "scene.pkg"
    } else {
        manifest.file.as_str()
    };
    let media_path = match safe_project_file(project_path, declared_file) {
        Ok(path) => path,
        Err(error) => {
            return WallpaperEngineProject {
                id,
                title,
                project_type,
                project_path: project_path.to_path_buf(),
                media_path: None,
                media_size_bytes: None,
                requires_wallpaper_engine,
                supported: false,
                reason: Some(error.to_string()),
            };
        }
    };
    let extension = media_path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let expected_extension = match project_type.as_str() {
        "video" => matches!(extension.as_str(), "mp4" | "m4v" | "webm"),
        "web" => matches!(extension.as_str(), "html" | "htm"),
        "scene" => extension == "pkg",
        _ => false,
    };
    if !expected_extension {
        return WallpaperEngineProject {
            id,
            title,
            project_type: project_type.clone(),
            project_path: project_path.to_path_buf(),
            media_path: Some(media_path),
            media_size_bytes: None,
            requires_wallpaper_engine,
            supported: false,
            reason: Some(format!(
                "{project_type} 项目的主文件格式不受支持：.{extension}"
            )),
        };
    }
    let media_size_bytes = fs::metadata(&media_path)
        .ok()
        .map(|metadata| metadata.len());
    let reason = match media_size_bytes {
        None => Some("无法读取壁纸资源大小".into()),
        Some(0) => Some("壁纸资源为空".into()),
        Some(size) if project_type == "video" && size > MAX_LOCAL_WALLPAPER_VIDEO_SIZE => {
            Some("视频超过本地动态壁纸 16 GiB 安全上限".into())
        }
        Some(_) => None,
    };
    WallpaperEngineProject {
        id,
        title,
        project_type,
        project_path: project_path.to_path_buf(),
        media_path: Some(media_path),
        media_size_bytes,
        requires_wallpaper_engine,
        supported: reason.is_none(),
        reason,
    }
}

fn safe_project_file(project_path: &Path, relative: &str) -> Result<PathBuf, ThemeError> {
    if relative.trim().is_empty() {
        return Err(ThemeError("project.json 未声明主文件".into()));
    }
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(ThemeError("project.json 的主文件路径不安全".into()));
    }
    let root = project_path
        .canonicalize()
        .map_err(|error| ThemeError(format!("无法解析项目目录：{error}")))?;
    let candidate = root.join(relative_path).canonicalize().map_err(|error| {
        ThemeError(format!(
            "找不到 Wallpaper Engine 主文件 {relative}：{error}"
        ))
    })?;
    if !candidate.starts_with(&root) || !candidate.is_file() {
        return Err(ThemeError(
            "Wallpaper Engine 主文件不在所选项目目录内".into(),
        ));
    }
    Ok(candidate)
}

pub fn load_wallpaper_engine_project(
    project_path: &Path,
    controls: WallpaperControls,
) -> Result<ThemePackage, ThemeError> {
    load_wallpaper_engine_project_from_root(
        &default_wallpaper_engine_root(),
        project_path,
        controls,
    )
}

fn load_wallpaper_engine_project_from_root(
    workshop_root: &Path,
    project_path: &Path,
    controls: WallpaperControls,
) -> Result<ThemePackage, ThemeError> {
    let controls = WallpaperControls::new(
        controls.wallpaper_brightness,
        controls.interface_transparency,
    )?;
    let workshop_root = workshop_root
        .canonicalize()
        .map_err(|error| ThemeError(format!("无法解析 Wallpaper Engine Workshop 目录：{error}")))?;
    let project_path = project_path
        .canonicalize()
        .map_err(|error| ThemeError(format!("无法解析 Wallpaper Engine 项目目录：{error}")))?;
    let id = project_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()))
        .ok_or_else(|| ThemeError("所选目录不是有效的 Wallpaper Engine Workshop 项目".into()))?;
    if project_path.parent() != Some(workshop_root.as_path()) {
        return Err(ThemeError(
            "只允许导入已扫描的 Wallpaper Engine Workshop 项目目录".into(),
        ));
    }
    let manifest = read_wallpaper_engine_manifest(&project_path)?;
    let project = describe_wallpaper_engine_project(&project_path, id.to_owned(), manifest);
    if !project.supported {
        return Err(ThemeError(
            project
                .reason
                .unwrap_or_else(|| "当前 Wallpaper Engine 项目不受支持".into()),
        ));
    }
    let media_path = project
        .media_path
        .clone()
        .ok_or_else(|| ThemeError("Wallpaper Engine 主文件路径缺失".into()))?;
    let extension = media_path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let playback_description = match project.project_type.as_str() {
        "video" => "本地视频流式播放，不复制 Workshop 原文件",
        "web" => "本地 Web 壁纸沙箱运行，已禁止联网和外部跳转",
        "scene" => "Wallpaper Engine 官方引擎原版渲染，高清离屏传输到 ChatGPT",
        _ => unreachable!("unsupported project types were rejected above"),
    };
    let theme_id = format!("local.wallpaper-engine.w{id}");
    let display_title = project.title.chars().take(80).collect::<String>();
    let manifest = json!({
        "schemaVersion": 1,
        "id": theme_id,
        "name": display_title,
        "description": format!("Wallpaper Engine 本地动态壁纸 · Workshop {id}"),
        "version": "1.0.0",
        "author": { "id": "local.wallpaper-engine", "name": "Wallpaper Engine" },
        "testedCodexVersions": [],
        "styles": ["styles/wallpaper.css"],
        "slots": [
            "app.shell", "app.background", "titlebar", "sidebar", "main", "page",
            "home.hero", "home.hero.media", "home.hero.media.asset", "home.card",
            "conversation.banner", "composer", "settings.canvas", "settings.card"
        ],
        "permissions": [],
        "preview": { "background": "#080b12", "surface": "#161b26", "accent": "#7aa2ff" },
        "experience": {
            "homeHero": {
                "eyebrow": "WALLPAPER ENGINE",
                "title": display_title,
                "description": playback_description,
                "asset": "assets/transparent.svg",
                "fit": "cover",
                "position": "center"
            },
            "homePrompt": { "title": "今天想聊些什么？" },
            "assets": [],
            "decorations": []
        },
        "locales": {}
    });
    let wallpaper_brightness = f32::from(controls.wallpaper_brightness) / 100.0;
    let interface_opacity = f32::from(100 - controls.interface_transparency) / 100.0;
    let css = format!(
        r#":root[data-ct-theme="{theme_id}"] {{ --ct-wallpaper-brightness: {wallpaper_brightness:.2}; --ct-interface-opacity: {interface_opacity:.2}; }}
:root[data-ct-theme="{theme_id}"] [data-ct-slot="app.shell"] {{ background: transparent !important; color: #f4f7ff; }}
:root[data-ct-theme="{theme_id}"] [data-ct-mount="app.background"] > :where(img, video, iframe, canvas) {{ display: block !important; width: 100% !important; height: 100% !important; border: 0 !important; object-fit: cover !important; object-position: center !important; opacity: 1 !important; filter: brightness(var(--ct-wallpaper-brightness)) saturate(0.9); }}
:root[data-ct-theme="{theme_id}"] [data-ct-mount="home.hero"] {{ display: none !important; }}
:root[data-ct-theme="{theme_id}"] :where([data-ct-slot="main"], [data-ct-slot="page"], [data-ct-slot="settings.canvas"]) {{ background: rgb(7 10 17 / calc(var(--ct-interface-opacity) * 0.66)) !important; }}
:root[data-ct-theme="{theme_id}"][data-ct-view="conversation"] :where([data-ct-slot="main"], [data-ct-slot="page"], [data-ct-slot="main.content.frame"]) {{ background: transparent !important; background-image: none !important; border-color: transparent !important; box-shadow: none !important; }}
:root[data-ct-theme="{theme_id}"] [data-ct-slot="sidebar"] {{ background: rgb(8 11 18 / var(--ct-interface-opacity)) !important; }}
:root[data-ct-theme="{theme_id}"] :where([data-ct-slot="home.card"], [data-ct-slot="composer"], [data-ct-slot="settings.card"], [data-ct-slot="conversation.banner"]) {{ background: rgb(16 21 31 / calc(var(--ct-interface-opacity) * 0.94)) !important; border-color: rgb(255 255 255 / calc(var(--ct-interface-opacity) * 0.22)) !important; }}
:root[data-ct-theme="{theme_id}"] :where([data-ct-slot="home.card"], [data-ct-slot="settings.card"], [data-ct-slot="conversation.banner"]) {{ box-shadow: 0 14px 36px rgb(0 0 0 / calc(var(--ct-interface-opacity) * 0.34)) !important; }}
:root[data-ct-theme="{theme_id}"] [data-ct-slot="composer"] {{ box-shadow: none !important; }}
:root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls {{ position: fixed !important; right: 20px !important; bottom: 104px !important; z-index: 2147483000 !important; width: 236px !important; color: #f7f8fc !important; font-size: 13px !important; line-height: 1.35 !important; pointer-events: auto !important; }}
:root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls * {{ box-sizing: border-box !important; }}
:root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls__panel {{ display: grid !important; gap: 11px !important; padding: 13px 14px !important; border: 1px solid rgb(255 255 255 / 16%) !important; border-radius: 14px !important; background: rgb(12 16 24 / 82%) !important; box-shadow: 0 14px 38px rgb(0 0 0 / 34%) !important; backdrop-filter: blur(18px) !important; -webkit-backdrop-filter: blur(18px) !important; }}
:root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls__header {{ display: flex !important; align-items: center !important; justify-content: space-between !important; gap: 8px !important; }}
:root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls__header strong {{ font-size: 13px !important; letter-spacing: .02em !important; }}
:root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls__collapse {{ width: 26px !important; height: 26px !important; padding: 0 !important; border: 0 !important; border-radius: 8px !important; color: #f7f8fc !important; background: rgb(255 255 255 / 9%) !important; cursor: pointer !important; }}
:root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls__field {{ display: grid !important; gap: 5px !important; }}
:root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls__label {{ display: flex !important; justify-content: space-between !important; gap: 8px !important; color: rgb(247 248 252 / 86%) !important; }}
:root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls__label output {{ color: #ffb340 !important; font-weight: 700 !important; }}
:root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls__range {{ width: 100% !important; height: 18px !important; margin: 0 !important; padding: 0 !important; accent-color: #f2a93b !important; cursor: pointer !important; }}
:root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls__playback {{ display: flex !important; align-items: center !important; justify-content: center !important; gap: 7px !important; width: 100% !important; min-height: 34px !important; padding: 7px 12px !important; border: 1px solid rgb(255 179 64 / 46%) !important; border-radius: 9px !important; color: #ffd28c !important; background: rgb(242 169 59 / 14%) !important; font-weight: 700 !important; cursor: pointer !important; transition: background-color 120ms ease, border-color 120ms ease !important; }}
:root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls__playback:hover {{ border-color: rgb(255 179 64 / 72%) !important; background: rgb(242 169 59 / 24%) !important; }}
:root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls__playback:focus-visible {{ outline: 2px solid #ffb340 !important; outline-offset: 2px !important; }}
:root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls__playback-icon {{ display: inline-flex !important; align-items: center !important; justify-content: center !important; width: 16px !important; color: #ffb340 !important; font-size: 12px !important; line-height: 1 !important; }}
:root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls__open {{ display: none !important; width: 48px !important; height: 48px !important; margin-left: auto !important; padding: 0 !important; border: 1px solid rgb(255 255 255 / 18%) !important; border-radius: 14px !important; color: #11151d !important; background: #f2a93b !important; box-shadow: 0 10px 28px rgb(0 0 0 / 30%) !important; font-weight: 800 !important; cursor: pointer !important; }}
:root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls.ct-wallpaper-controls--collapsed {{ width: 48px !important; }}
:root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls.ct-wallpaper-controls--collapsed .ct-wallpaper-controls__panel {{ display: none !important; }}
:root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls.ct-wallpaper-controls--collapsed .ct-wallpaper-controls__open {{ display: block !important; }}
@media (max-width: 900px), (max-height: 700px) {{ :root[data-ct-theme="{theme_id}"] .ct-wallpaper-controls {{ right: 12px !important; bottom: 84px !important; }} }}"#
    );
    let transparent_svg = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"></svg>"#;
    let files = HashMap::from([
        (
            "manifest.json".to_owned(),
            serde_json::to_vec_pretty(&manifest)
                .map_err(|error| ThemeError(format!("无法创建动态壁纸主题：{error}")))?,
        ),
        ("styles/wallpaper.css".to_owned(), css.into_bytes()),
        (
            "assets/transparent.svg".to_owned(),
            transparent_svg.to_vec(),
        ),
    ]);
    let mut package = parse_package_files(&files, false)?;
    match project.project_type.as_str() {
        "video" => {
            let mime = if extension == "webm" {
                "video/webm"
            } else {
                "video/mp4"
            };
            package.runtime_assets.insert(
                WALLPAPER_ASSET_PATH.into(),
                ThemeRuntimeAsset::from_file(WALLPAPER_ASSET_PATH.into(), mime.into(), media_path),
            );
            package.wallpaper_file = Some(WALLPAPER_ASSET_PATH.into());
        }
        "web" => {
            for asset in collect_wallpaper_web_assets(&project_path, &media_path)? {
                package.runtime_assets.insert(asset.path.clone(), asset);
            }
        }
        "scene" => {
            let scene =
                SceneWallpaperCapture::start(&workshop_root, &project_path.join("project.json"))
                    .map_err(ThemeError)?;
            package.runtime_assets.insert(
                WALLPAPER_ASSET_PATH.into(),
                ThemeRuntimeAsset::from_memory(
                    WALLPAPER_ASSET_PATH.into(),
                    "image/jpeg".into(),
                    Vec::new(),
                ),
            );
            package.wallpaper_scene = Some(Arc::new(scene));
        }
        _ => unreachable!("unsupported project types were rejected above"),
    }
    package.wallpaper_asset = Some(WALLPAPER_ASSET_PATH.into());
    package.wallpaper_kind = Some(project.project_type);
    package.wallpaper_controls = Some(controls);
    Ok(package)
}

fn collect_wallpaper_web_assets(
    project_root: &Path,
    entry_path: &Path,
) -> Result<Vec<ThemeRuntimeAsset>, ThemeError> {
    let project_root = project_root
        .canonicalize()
        .map_err(|error| ThemeError(format!("无法解析 Web 壁纸目录：{error}")))?;
    let entry_path = entry_path
        .canonicalize()
        .map_err(|error| ThemeError(format!("无法解析 Web 壁纸入口：{error}")))?;
    let mut pending = vec![project_root.clone()];
    let mut files = Vec::new();
    let mut total_size = 0_u64;
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)
            .map_err(|error| ThemeError(format!("无法读取 Web 壁纸目录：{error}")))?
        {
            let entry =
                entry.map_err(|error| ThemeError(format!("无法读取 Web 壁纸文件：{error}")))?;
            let file_type = entry
                .file_type()
                .map_err(|error| ThemeError(format!("无法检查 Web 壁纸文件：{error}")))?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                pending.push(entry.path());
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let path = entry
                .path()
                .canonicalize()
                .map_err(|error| ThemeError(format!("无法解析 Web 壁纸文件：{error}")))?;
            if !path.starts_with(&project_root) {
                return Err(ThemeError("Web 壁纸文件越出了项目目录".into()));
            }
            let size = fs::metadata(&path)
                .map_err(|error| ThemeError(format!("无法读取 Web 壁纸文件大小：{error}")))?
                .len();
            total_size = total_size
                .checked_add(size)
                .ok_or_else(|| ThemeError("Web 壁纸总大小溢出".into()))?;
            if total_size > MAX_WALLPAPER_WEB_TOTAL_SIZE {
                return Err(ThemeError("Web 壁纸超过 512 MiB 安全上限".into()));
            }
            files.push(path);
            if files.len() > MAX_WALLPAPER_WEB_FILE_COUNT {
                return Err(ThemeError("Web 壁纸文件数量超过 4096 个安全上限".into()));
            }
        }
    }
    let mut assets = Vec::with_capacity(files.len());
    for path in files {
        let relative = path
            .strip_prefix(&project_root)
            .map_err(|_| ThemeError("Web 壁纸文件越出了项目目录".into()))?;
        let route_relative = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        let logical_path = if path == entry_path {
            WALLPAPER_ASSET_PATH.to_owned()
        } else {
            format!("wallpaper-engine/web/{route_relative}")
        };
        assets.push(ThemeRuntimeAsset::from_web_file(
            logical_path,
            wallpaper_web_mime(&path).into(),
            path,
            format!("web/{route_relative}"),
        ));
    }
    if !assets
        .iter()
        .any(|asset| asset.path == WALLPAPER_ASSET_PATH)
    {
        return Err(ThemeError("Web 壁纸入口文件不在项目目录中".into()));
    }
    Ok(assets)
}

fn wallpaper_web_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        _ => "application/octet-stream",
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
        let mut registry = self.read_registry()?;
        let mut themes = Vec::new();
        let mut invalid_theme_ids = Vec::new();
        for (theme_id, installed) in &registry.themes {
            match self.load_installed(theme_id, installed) {
                Ok(package) => themes.push(package.summary(false, Some(&installed.online))),
                Err(_) => invalid_theme_ids.push(theme_id.clone()),
            }
        }
        if !invalid_theme_ids.is_empty() {
            let removed = invalid_theme_ids
                .into_iter()
                .filter_map(|theme_id| registry.themes.remove(&theme_id))
                .collect::<Vec<_>>();
            let registry_bytes = serde_json::to_vec_pretty(&registry)
                .map_err(|error| ThemeError(format!("无法保存主题注册表：{error}")))?;
            write_atomic(&self.registry_path(), &registry_bytes)?;
            for installed in removed {
                if !registry
                    .themes
                    .values()
                    .any(|item| item.digest == installed.digest)
                {
                    let _ = fs::remove_file(self.store_path(&installed.digest));
                }
            }
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
        let files = read_archive(archive_bytes, true)?;
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
        let stored = self.encrypt_online_cache(&digest, archive_bytes)?;
        write_atomic(&store_path, &stored)?;
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
        let files = read_archive(&archive_bytes, true)?;
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
    let validated = validate_package_files(files, signed_package)?;
    let manifest = validated.manifest;
    let runtime_config = build_runtime_config(&manifest, files)?;
    let runtime_assets = collect_runtime_assets(files, &runtime_config)?;

    Ok(ThemePackage {
        manifest,
        css: validated.css,
        runtime_assets,
        wallpaper_asset: None,
        wallpaper_kind: None,
        wallpaper_file: None,
        wallpaper_scene: None,
        wallpaper_controls: None,
    })
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
                "lightAssetUrl": asset
                    .light_asset
                    .as_deref()
                    .map(&load_asset_url)
                    .transpose()?,
                "darkAssetUrl": asset
                    .dark_asset
                    .as_deref()
                    .map(&load_asset_url)
                    .transpose()?,
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
                ThemeRuntimeAsset::from_memory(path.to_owned(), mime, source.clone()),
            ))
        })
        .collect()
}

fn collect_runtime_asset_paths<'a>(value: &'a Value, paths: &mut HashSet<&'a str>) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if matches!(
                    key.as_str(),
                    "assetUrl" | "foregroundAssetUrl" | "lightAssetUrl" | "darkAssetUrl"
                ) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use std::io::Cursor;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    const TEST_SECRET: [u8; 32] = [7; 32];

    #[test]
    fn scans_and_loads_wallpaper_engine_video_without_copying_it() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let root = directory.path().join("431960");
        let project_path = root.join("1234567890");
        fs::create_dir_all(&project_path).expect("workshop project directory");
        fs::write(
            project_path.join("project.json"),
            br#"{"title":"Test video","type":"video","file":"wallpaper.mp4"}"#,
        )
        .expect("project manifest");
        fs::write(project_path.join("wallpaper.mp4"), b"video-bytes").expect("test video");

        let catalog = scan_wallpaper_engine_root(&root).expect("wallpaper catalog");
        assert_eq!(catalog.projects.len(), 1);
        assert!(catalog.projects[0].supported);
        assert_eq!(catalog.projects[0].media_size_bytes, Some(11));

        let controls = WallpaperControls::new(42, 73).expect("wallpaper controls");
        let package = load_wallpaper_engine_project_from_root(&root, &project_path, controls)
            .expect("video package");
        let assets = package.runtime_assets();
        let video = assets
            .iter()
            .find(|asset| asset.path == WALLPAPER_ASSET_PATH)
            .expect("streamed video asset");
        assert!(matches!(&video.source, ThemeRuntimeAssetSource::File(_)));
        let urls = assets
            .iter()
            .map(|asset| {
                (
                    asset.path.clone(),
                    format!("http://127.0.0.1/{}", asset.path),
                )
            })
            .collect();
        let config = package
            .runtime_config_with_asset_urls(&urls)
            .expect("runtime config");
        assert_eq!(config["wallpaper"]["kind"], "video");
        assert_eq!(
            config["wallpaper"]["assetUrl"].as_str(),
            Some(urls[WALLPAPER_ASSET_PATH].as_str())
        );
        assert_eq!(config["wallpaper"]["brightness"], 42);
        assert_eq!(config["wallpaper"]["interfaceTransparency"], 73);
        assert!(package.css.contains("--ct-wallpaper-brightness: 0.42"));
        assert!(package.css.contains("--ct-interface-opacity: 0.27"));
        assert!(package.css.contains("[data-ct-mount=\"home.hero\"]"));
        assert!(package.css.contains("display: none !important"));
        assert!(package.css.contains(".ct-wallpaper-controls"));
        assert!(
            package
                .css
                .contains("[data-ct-view=\"conversation\"] :where([data-ct-slot=\"main\"]")
        );
        assert!(package.css.contains("background: transparent !important"));
        assert!(
            package
                .css
                .contains("[data-ct-slot=\"composer\"] { box-shadow: none !important; }")
        );
    }

    #[test]
    fn rejects_wallpaper_controls_above_one_hundred() {
        let error = WallpaperControls::new(101, 32).expect_err("brightness above 100 must fail");
        assert!(error.to_string().contains("0 到 100"));
        let error = WallpaperControls::new(68, 101)
            .expect_err("interface transparency above 100 must fail");
        assert!(error.to_string().contains("0 到 100"));
    }

    #[test]
    fn scans_scene_and_loads_sandboxed_web_projects() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let root = directory.path().join("431960");
        for (id, project_type, file) in
            [("100", "scene", "scene.json"), ("200", "web", "index.html")]
        {
            let project_path = root.join(id);
            fs::create_dir_all(&project_path).expect("workshop project directory");
            fs::write(
                project_path.join("project.json"),
                serde_json::to_vec(&json!({
                    "title": id,
                    "type": project_type,
                    "file": file,
                    "preview": if project_type == "scene" { "preview.gif" } else { "" },
                }))
                .expect("project manifest JSON"),
            )
            .expect("project manifest");
        }
        fs::write(root.join("100").join("scene.pkg"), b"scene-package").expect("scene package");
        fs::write(
            root.join("200").join("index.html"),
            br#"<html><script src="js/app.js"></script></html>"#,
        )
        .expect("web entry");
        fs::create_dir_all(root.join("200").join("js")).expect("web script directory");
        fs::write(
            root.join("200").join("js").join("app.js"),
            b"window.ready=true",
        )
        .expect("web script");

        let catalog = scan_wallpaper_engine_root(&root).expect("wallpaper catalog");
        assert_eq!(catalog.projects.len(), 2);
        assert!(catalog.projects.iter().all(|project| project.supported));
        let scene = catalog
            .projects
            .iter()
            .find(|project| project.project_type == "scene")
            .expect("scene project");
        assert!(scene.requires_wallpaper_engine);
        assert_eq!(
            scene.media_path.as_deref(),
            Some(
                root.join("100")
                    .join("scene.pkg")
                    .canonicalize()
                    .unwrap()
                    .as_path()
            )
        );
        let web = catalog
            .projects
            .iter()
            .find(|project| project.project_type == "web")
            .expect("web project");
        assert!(!web.requires_wallpaper_engine);

        let package = load_wallpaper_engine_project_from_root(
            &root,
            &root.join("200"),
            WallpaperControls::default(),
        )
        .expect("sandboxed web package");
        let assets = package.runtime_assets();
        assert_eq!(
            assets
                .iter()
                .filter(|asset| asset.policy == ThemeAssetPolicy::SandboxedWeb)
                .count(),
            3
        );
        let entry = assets
            .iter()
            .find(|asset| asset.path == WALLPAPER_ASSET_PATH)
            .expect("web entry asset");
        assert_eq!(entry.route_path.as_deref(), Some("web/index.html"));
        assert_eq!(entry.policy, ThemeAssetPolicy::SandboxedWeb);
        let urls = assets
            .iter()
            .map(|asset| {
                (
                    asset.path.clone(),
                    format!("http://127.0.0.1/{}", asset.path),
                )
            })
            .collect();
        let config = package
            .runtime_config_with_asset_urls(&urls)
            .expect("web runtime config");
        assert_eq!(config["wallpaper"]["kind"], "web");
    }

    #[test]
    fn classifies_static_scene_previews_as_original_engine_scenes() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let root = directory.path().join("431960");
        let project_path = root.join("300");
        fs::create_dir_all(&project_path).expect("workshop project directory");
        fs::write(
            project_path.join("project.json"),
            br#"{"title":"Static scene","type":"scene","file":"scene.json","preview":"preview.jpg"}"#,
        )
        .expect("project manifest");
        fs::write(project_path.join("scene.pkg"), b"scene-package").expect("scene package");
        fs::write(project_path.join("preview.jpg"), b"jpeg-preview").expect("scene preview");

        let catalog = scan_wallpaper_engine_root(&root).expect("wallpaper catalog");
        let project = catalog.projects.first().expect("scene project");
        assert!(project.supported);
        assert!(project.requires_wallpaper_engine);
        assert!(
            project
                .media_path
                .as_ref()
                .is_some_and(|path| path.ends_with("scene.pkg"))
        );
    }

    #[test]
    fn codex_tested_versions_are_optional_and_accept_legacy_field() {
        let mut manifest: Value = serde_json::from_str(TEST_THEME_MANIFEST).expect("test manifest");
        manifest
            .as_object_mut()
            .expect("manifest object")
            .remove("testedCodexVersions");
        let without_versions: ThemeManifest =
            serde_json::from_value(manifest.clone()).expect("versions are optional");
        assert!(without_versions.tested_codex_versions.is_empty());

        manifest["supportedCodexVersions"] = json!(["26.707.91948"]);
        let legacy: ThemeManifest =
            serde_json::from_value(manifest).expect("legacy version field remains readable");
        assert_eq!(legacy.tested_codex_versions, ["26.707.91948"]);
    }

    #[test]
    fn rejects_unknown_manifest_fields_in_nested_protocol_objects() {
        let mut manifest: Value = serde_json::from_str(TEST_THEME_MANIFEST).expect("test manifest");
        manifest["experience"]["conversationSummaryDecoraton"] = json!({
            "asset": "assets/ornament.svg"
        });
        let error = serde_json::from_value::<ThemeManifest>(manifest)
            .expect_err("misspelled protocol fields must fail")
            .to_string();
        assert!(error.contains("unknown field `conversationSummaryDecoraton`"));
    }

    #[test]
    fn decorations_default_to_empty_when_omitted() {
        let mut manifest: Value = serde_json::from_str(TEST_THEME_MANIFEST).expect("test manifest");
        manifest["experience"]
            .as_object_mut()
            .expect("experience object")
            .remove("decorations");
        let manifest: ThemeManifest =
            serde_json::from_value(manifest).expect("decorations should be optional");
        assert!(manifest.experience.decorations.is_empty());
        validate_manifest(&manifest).expect("manifest without decorations should remain valid");
    }

    #[test]
    fn rejects_duplicate_styles_and_slots() {
        let mut manifest: Value = serde_json::from_str(TEST_THEME_MANIFEST).expect("test manifest");
        manifest["styles"] = json!(["styles/theme.css", "styles/theme.css"]);
        let duplicate_styles: ThemeManifest =
            serde_json::from_value(manifest.clone()).expect("manifest");
        assert!(
            validate_manifest(&duplicate_styles)
                .expect_err("duplicate styles must fail")
                .to_string()
                .contains("样式文件不能重复")
        );

        manifest["styles"] = json!(["styles/theme.css"]);
        manifest["slots"] = json!(["app.shell", "app.shell"]);
        let duplicate_slots: ThemeManifest = serde_json::from_value(manifest).expect("manifest");
        assert!(
            validate_manifest(&duplicate_slots)
                .expect_err("duplicate slots must fail")
                .to_string()
                .contains("逻辑插槽不能重复")
        );
    }

    #[test]
    fn rejects_overlong_base_manifest_text() {
        for (field, length, expected) in [
            ("name", 121, "Theme name"),
            ("description", 241, "Theme description"),
        ] {
            let mut manifest: Value =
                serde_json::from_str(TEST_THEME_MANIFEST).expect("test manifest");
            manifest[field] = json!("x".repeat(length));
            let manifest: ThemeManifest = serde_json::from_value(manifest).expect("manifest");
            assert!(
                validate_manifest(&manifest)
                    .expect_err("overlong text must fail")
                    .to_string()
                    .contains(expected)
            );
        }

        let mut manifest: Value = serde_json::from_str(TEST_THEME_MANIFEST).expect("test manifest");
        manifest["author"]["name"] = json!("x".repeat(101));
        let manifest: ThemeManifest = serde_json::from_value(manifest).expect("manifest");
        assert!(
            validate_manifest(&manifest)
                .expect_err("overlong author name must fail")
                .to_string()
                .contains("Author name")
        );
    }

    #[test]
    fn selector_lists_preserve_nested_function_commas() {
        let source = r#":root[data-ct-theme="studio.example.test-theme"] :is([data-ct-slot="home.card"], [data-ct-slot="settings.card"]) { color: white; }"#;
        validate_css(source, "studio.example.test-theme")
            .expect("nested selector commas must not split the root scope");
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
            {
                "slot": "app.background",
                "asset": "assets/hero.svg",
                "lightAsset": "assets/hero.svg",
                "darkAsset": "assets/ornament.svg"
            },
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
        assert_eq!(config["assets"][0]["lightAssetUrl"], "assets/hero.svg");
        assert_eq!(config["assets"][0]["darkAssetUrl"], "assets/ornament.svg");
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
                    if matches!(
                        key.as_str(),
                        "assetUrl" | "foregroundAssetUrl" | "lightAssetUrl" | "darkAssetUrl"
                    ) {
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
    fn removes_online_cache_encrypted_for_another_device() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let root = directory.path().join("repository");
        let signing_key = SigningKey::from_bytes(&TEST_SECRET);
        let first_device = ThemeRepository {
            root: root.clone(),
            verifying_key: signing_key.verifying_key(),
            cache_key: Some([9; 32]),
        };
        first_device
            .install_online(&signed_online_test_package(), "test-theme".into())
            .expect("first device install");

        let current_device = ThemeRepository {
            root,
            verifying_key: signing_key.verifying_key(),
            cache_key: Some([10; 32]),
        };
        assert!(
            current_device
                .list()
                .expect("stale cache cleanup")
                .is_empty()
        );
        assert!(
            current_device
                .read_registry()
                .expect("cleaned registry")
                .themes
                .is_empty()
        );
    }

    #[test]
    fn reinstall_reencrypts_online_cache_for_current_device() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let root = directory.path().join("repository");
        let package = signed_online_test_package();
        let signing_key = SigningKey::from_bytes(&TEST_SECRET);
        let first_device = ThemeRepository {
            root: root.clone(),
            verifying_key: signing_key.verifying_key(),
            cache_key: Some([9; 32]),
        };
        first_device
            .install_online(&package, "test-theme".into())
            .expect("first device install");

        let current_device = ThemeRepository {
            root,
            verifying_key: signing_key.verifying_key(),
            cache_key: Some([10; 32]),
        };
        current_device
            .install_online(&package, "test-theme".into())
            .expect("current device reinstall");
        assert_eq!(
            current_device
                .load("studio.example.test-theme")
                .expect("current device cache")
                .id(),
            "studio.example.test-theme"
        );
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
        let theme_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/theme-example/package");

        let package = repository
            .load_development(&theme_path)
            .expect("documented example theme should remain loadable");

        assert_eq!(package.id(), "studio.example.protocol-preview");
        assert_eq!(package.version(), "1.0.0");
        assert_eq!(package.runtime_assets().len(), 1);
    }

    #[test]
    fn documented_schema_and_slot_catalog_match_the_runtime_protocol() {
        let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs");
        let schema: Value = serde_json::from_str(
            &fs::read_to_string(docs.join("theme.schema.json")).expect("theme schema"),
        )
        .expect("valid theme schema JSON");
        let schema_slots = schema["$defs"]["slot"]["enum"]
            .as_array()
            .expect("schema slot enum")
            .iter()
            .map(|slot| slot.as_str().expect("string slot"))
            .collect::<Vec<_>>();
        assert_eq!(schema_slots, ALLOWED_SLOTS);

        let catalog = fs::read_to_string(docs.join("theme-slots.md")).expect("theme slot catalog");
        for slot in ALLOWED_SLOTS {
            assert!(
                catalog.contains(&format!("`{slot}`")),
                "slot catalog is missing {slot}"
            );
        }
    }

    #[test]
    fn accepts_platform_and_account_author_ids_without_weakening_theme_ids() {
        assert!(is_valid_author_id("retheme"));
        assert!(is_valid_author_id("123456789"));
        assert!(is_valid_author_id("studio.example"));
        assert!(is_valid_theme_id("studio.example.community-theme"));
        assert!(!is_valid_theme_id("community-theme"));
        assert!(!is_valid_author_id("Invalid Author"));
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
        let files = read_archive(&signed_test_package(true), true).expect("read archive");
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
        let error = read_archive(&cursor.into_inner(), true).expect_err("unsafe path must fail");
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
        let files = read_archive(&package, true).expect("read signed package");
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
