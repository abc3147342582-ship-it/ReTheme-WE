import { useEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";
import {
  applyTheme,
  chooseAndPreviewLocalTheme,
  checkForUpdate,
  completeOAuthLogin,
  detectCodex,
  downloadOnlineTheme,
  getAccountStatus,
  getAccountSync,
  getRuntimeEnvironment,
  getThemePreviewStatus,
  getWallpaperControlPreferences,
  getWallpaperEngineCatalog,
  isDesktopRuntime,
  listThemes,
  openGitHubRepository,
  openWebsite,
  previewWallpaperEngineProject,
  releaseUiMemory,
  restoreOfficialTheme,
  runSmokeTest,
  syncThemeLocale,
  syncTrayLocale,
  uninstallTheme,
  updateWallpaperControls,
  type DesktopUpdate,
} from "./desktop-api";
import AccountDialog from "./AccountDialog";
import { localizeAccountTheme, localizeTheme, useI18n, type TranslationKey } from "./i18n";
import type {
  AccountStatus,
  AccountSync,
  AccountThemeSummary,
  CodexInstallation,
  DesktopPreferences,
  PageId,
  RuntimeEnvironment,
  SmokeTestReport,
  ThemeInstallReport,
  ThemePreviewReport,
  ThemeSummary,
  WallpaperEngineCatalog,
  WallpaperEngineProject,
  WallpaperControls,
} from "./types";

const INITIAL_ACCOUNT: AccountStatus = {
  authenticated: false,
  pro: false,
  deviceName: "Local device",
  heartbeatState: "offline",
  entitlements: [],
  trials: [],
};

type Translate = (key: TranslationKey, values?: Record<string, string | number>) => string;

function formatFileSize(bytes?: number): string {
  if (bytes === undefined) return "—";
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
  return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
}

function formatWallpaperProject(project: WallpaperEngineProject): string {
  const kind = project.projectType === "web" ? "Web" : project.projectType === "scene" ? "Scene" : "Video";
  return `${project.title} · ${kind} · ${project.id} · ${formatFileSize(project.mediaSizeBytes)}`;
}

function heartbeatCopy(status: AccountStatus, t: Translate) {
  if (!status.authenticated) return { title: t("heartbeat.signedOut.title"), detail: t("heartbeat.signedOut.detail"), live: false };
  if (status.heartbeatState === "online") return { title: t("heartbeat.online.title"), detail: t("heartbeat.online.detail"), live: true };
  if (status.heartbeatState === "grace") return { title: t("heartbeat.grace.title"), detail: status.error || t("heartbeat.grace.detail"), live: true };
  if (status.heartbeatState === "replaced") return { title: t("heartbeat.replaced.title"), detail: status.error || t("heartbeat.replaced.detail"), live: false };
  return { title: t("heartbeat.offline.title"), detail: status.error || t("heartbeat.offline.detail"), live: false };
}

const INITIAL_PREFERENCES: DesktopPreferences = {
  launchAtLogin: false,
  hideToTray: true,
  autoUpdate: true,
  autoDetectCodex: true,
  lowMemoryMode: true,
};

type Appearance = "system" | "light" | "dark";

const APPEARANCE_KEY = "retheme.appearance";
const COMPATIBILITY_CACHE_KEY = "retheme.compatibility-check.v2";
const PREFERENCES_KEY = "retheme.preferences.v1";
const UPDATE_REPOSITORY_KEY = "retheme.update-repository.v1";
const WALLPAPER_PREFERENCES_KEY = "retheme.wallpaper-preferences.v1";
const AUTOMATIC_SELF_CHECK_DELAY_MS = 8_000;

type WallpaperPreferences = WallpaperControls & {
  projectPath: string;
};

const INITIAL_WALLPAPER_PREFERENCES: WallpaperPreferences = {
  projectPath: "",
  wallpaperBrightness: 68,
  interfaceTransparency: 32,
};

function loadPreferences(): DesktopPreferences {
  try {
    return {
      ...INITIAL_PREFERENCES,
      ...JSON.parse(window.localStorage.getItem(PREFERENCES_KEY) ?? "{}"),
    };
  } catch {
    return INITIAL_PREFERENCES;
  }
}

function loadUpdateRepository(): string {
  return window.localStorage.getItem(UPDATE_REPOSITORY_KEY)?.trim() ?? "";
}

function loadWallpaperPreferences(): WallpaperPreferences {
  try {
    const stored = JSON.parse(window.localStorage.getItem(WALLPAPER_PREFERENCES_KEY) ?? "{}") as Partial<WallpaperPreferences> & { backgroundOpacity?: number };
    const wallpaperBrightness = Number(stored.wallpaperBrightness ?? stored.backgroundOpacity);
    const interfaceTransparency = Number(stored.interfaceTransparency);
    return {
      projectPath: typeof stored.projectPath === "string" ? stored.projectPath : "",
      wallpaperBrightness: Number.isFinite(wallpaperBrightness)
        ? Math.min(100, Math.max(0, Math.round(wallpaperBrightness)))
        : INITIAL_WALLPAPER_PREFERENCES.wallpaperBrightness,
      interfaceTransparency: Number.isFinite(interfaceTransparency)
        ? Math.min(100, Math.max(0, Math.round(interfaceTransparency)))
        : INITIAL_WALLPAPER_PREFERENCES.interfaceTransparency,
    };
  } catch {
    return INITIAL_WALLPAPER_PREFERENCES;
  }
}

type CachedCompatibilityCheck = {
  appVersion: string;
  report?: SmokeTestReport;
  error?: string;
};

function loadCompatibilityCheck(appVersion: string): CachedCompatibilityCheck | null {
  try {
    const cached = JSON.parse(window.localStorage.getItem(COMPATIBILITY_CACHE_KEY) ?? "null") as CachedCompatibilityCheck | null;
    return cached?.appVersion === appVersion ? cached : null;
  } catch {
    return null;
  }
}

function loadAppearance(): Appearance {
  const stored = window.localStorage.getItem(APPEARANCE_KEY);
  return stored === "light" || stored === "dark" ? stored : "system";
}

function Icon({ name, size = 18 }: { name: string; size?: number }) {
  const paths: Record<string, ReactNode> = {
    overview: <><rect x="3" y="3" width="7" height="7" rx="2" /><rect x="14" y="3" width="7" height="7" rx="2" /><rect x="3" y="14" width="7" height="7" rx="2" /><rect x="14" y="14" width="7" height="7" rx="2" /></>,
    themes: <><path d="M12 3a9 9 0 1 0 0 18h1.4a1.6 1.6 0 0 0 .5-3.1 1.6 1.6 0 0 1 .5-3.1H17a4 4 0 0 0 4-4A7.8 7.8 0 0 0 12 3Z" /><circle cx="7.5" cy="10" r=".8" fill="currentColor" /><circle cx="10" cy="6.8" r=".8" fill="currentColor" /><circle cx="14" cy="6.6" r=".8" fill="currentColor" /></>,
    favorites: <path d="M20.8 5.7a5.5 5.5 0 0 0-7.8 0L12 6.8l-1.1-1.1a5.5 5.5 0 0 0-7.8 7.8L12 22l8.8-8.5a5.5 5.5 0 0 0 0-7.8Z" />,
    account: <><circle cx="12" cy="8" r="4" /><path d="M4 21a8 8 0 0 1 16 0" /></>,
    devices: <><rect x="3" y="4" width="18" height="13" rx="2" /><path d="M8 21h8M12 17v4" /></>,
    settings: <><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.8 2.8-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.6v.2h-4V21a1.7 1.7 0 0 0-1-1.6 1.7 1.7 0 0 0-1.9.3l-.1.1L4.2 17l.1-.1a1.7 1.7 0 0 0 .3-1.9A1.7 1.7 0 0 0 3 14H2.8v-4H3a1.7 1.7 0 0 0 1.6-1 1.7 1.7 0 0 0-.3-1.9L4.2 7 7 4.2l.1.1a1.7 1.7 0 0 0 1.9.3A1.7 1.7 0 0 0 10 3v-.2h4V3a1.7 1.7 0 0 0 1 1.6 1.7 1.7 0 0 0 1.9-.3l.1-.1L19.8 7l-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.6 1h.2v4H21a1.7 1.7 0 0 0-1.6 1Z" /></>,
    check: <path d="m5 12 4 4L19 6" />,
    arrow: <><path d="M5 12h14" /><path d="m13 6 6 6-6 6" /></>,
    refresh: <><path d="M20 6v5h-5" /><path d="M19 11a7 7 0 1 0-2 7" /></>,
    package: <><path d="m12 3 8 4.5v9L12 21l-8-4.5v-9L12 3Z" /><path d="m4.5 7.8 7.5 4.3 7.5-4.3M12 12v9" /></>,
    shield: <><path d="M12 3 5 6v5c0 4.6 2.8 8.1 7 10 4.2-1.9 7-5.4 7-10V6l-7-3Z" /><path d="m9 12 2 2 4-5" /></>,
    external: <><path d="M14 4h6v6" /><path d="m20 4-9 9" /><path d="M18 13v6a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h6" /></>,
    monitor: <><rect x="3" y="4" width="18" height="13" rx="2" /><path d="M8 21h8M12 17v4" /></>,
    heart: <path d="M20.8 5.7a5.5 5.5 0 0 0-7.8 0L12 6.8l-1.1-1.1a5.5 5.5 0 0 0-7.8 7.8L12 22l8.8-8.5a5.5 5.5 0 0 0 0-7.8Z" />,
    user: <><circle cx="12" cy="8" r="4" /><path d="M4 21a8 8 0 0 1 16 0" /></>,
    clock: <><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></>,
    download: <><path d="M12 3v12" /><path d="m7 10 5 5 5-5" /><path d="M5 21h14" /></>,
    store: <><path d="M4 7h16l-1.4 12.2a1.8 1.8 0 0 1-1.8 1.6H7.2a1.8 1.8 0 0 1-1.8-1.6L4 7Z" /><path d="M8 10V6a4 4 0 0 1 8 0v4" /></>,
    activity: <path d="M3 12h4l2-6 4 12 2-6h6" />,
  };
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      {paths[name]}
    </svg>
  );
}

function Logo() {
  return <span className="logo-mark" aria-hidden="true"><i /><i /><i /></span>;
}

function artworkCopy(theme: ThemeSummary) {
  return { eyebrow: "RETHEME", title: theme.name || "THEME" };
}

function ThemeArtwork({ theme, compact = false }: { theme: ThemeSummary; compact?: boolean }) {
  const style = {
    "--preview-bg": theme.preview.background,
    "--preview-surface": theme.preview.surface,
    "--preview-accent": theme.preview.accent,
  } as CSSProperties;
  const copy = artworkCopy(theme);
  return (
    <div className={`theme-artwork ${compact ? "is-compact" : ""}`} style={style} aria-hidden="true">
      <div className="art-sidebar"><b /><i /><i /><i /></div>
      <div className="art-main">
        <div className="art-banner"><span>{copy.eyebrow}</span><strong>{copy.title}</strong></div>
        <div className="art-cards"><i /><i /><i /></div>
        <div className="art-composer"><span /></div>
      </div>
    </div>
  );
}

function CloudThemeCard({ theme, installed, installing, onInstall }: { theme: AccountThemeSummary; installed: boolean; installing: boolean; onInstall: () => void }) {
  const { t } = useI18n();
  const preview = theme.preview ?? { background: "#17191f", surface: "#242832", accent: "#8ca9ff" };
  return (
    <article className="cloud-theme-card">
      <button className="cloud-theme-cover" style={{ background: `linear-gradient(145deg, ${preview.background}, ${preview.surface})` }} onClick={() => void openWebsite(`/theme?slug=${encodeURIComponent(theme.slug)}`)} aria-label={t("themes.viewOnline", { name: theme.name })}>
        {theme.coverUrl ? <img src={theme.coverUrl} alt="" /> : <span style={{ color: preview.accent }}>{theme.name}</span>}
      </button>
      <div><strong>{theme.name}</strong><small>{theme.author.name} · v{theme.version}</small></div>
      <button className="pill" disabled={installed || installing} onClick={onInstall}>{installed ? t("common.installed") : installing ? t("common.installing") : t("common.install")}</button>
    </article>
  );
}

function Switch({ checked, disabled = false, label, onChange }: { checked: boolean; disabled?: boolean; label: string; onChange: () => void }) {
  return (
    <button className={`switch ${checked ? "is-on" : ""}`} role="switch" aria-checked={checked} aria-label={label} disabled={disabled} onClick={onChange}>
      <span />
    </button>
  );
}

type SharedPageProps = {
  account: AccountStatus;
  installation: CodexInstallation | null;
  detectError: string;
  detecting: boolean;
  themes: ThemeSummary[];
  themesError: string;
  previewReport: ThemePreviewReport | null;
  previewError: string;
  activeTheme: ThemeSummary | undefined;
  busyThemeId: string;
  uninstallingThemeId: string;
  stopping: boolean;
  testing: boolean;
  onDetect: () => void;
  onApplyTheme: (themeId: string) => void;
  onUninstallTheme: (theme: ThemeSummary) => void;
  onPreviewLocalTheme: () => void;
  onPreviewWallpaperEngine: (projectPath: string, controls: WallpaperControls) => void;
  onRestore: () => void;
  onNavigate: (page: PageId) => void;
};

function OverviewPage(props: SharedPageProps & { runtimeEnvironment: RuntimeEnvironment | null; smokeReport: SmokeTestReport | null; smokeError: string; testing: boolean; onSmokeTest: () => void }) {
  const { t } = useI18n();
  const { installation, previewReport, activeTheme } = props;
  const heartbeat = heartbeatCopy(props.account, t);
  const compatibility = props.runtimeEnvironment?.compatibility;
  const compatibilityTitle = compatibility?.source === "signedRemote"
    ? t("overview.remoteRevision", { revision: compatibility.revision ?? "—" })
    : compatibility ? t("overview.builtInRules") : t("overview.reading");
  const diagnosticPassed = Boolean(props.smokeReport?.compatible);
  const diagnosticFailed = Boolean(props.smokeReport && !props.smokeReport.compatible);
  const diagnosticDetail = props.smokeReport
    ? props.smokeReport.compatible
      ? t("overview.diagnosticPassed", { duration: props.smokeReport.durationMs })
      : t("overview.diagnosticMissing", {
          probes: props.smokeReport.missingAdapterProbes.join(", ") || t("overview.versionOutOfRange"),
        })
    : props.smokeError || t("overview.diagnosticDetail");
  const officialTheme: ThemeSummary = {
    id: "official", name: t("overview.official"), description: "", version: "", author: "",
    preview: { background: "#212327", surface: "#2e3136", accent: "#9aa2ad" }, builtIn: true,
  };
  return (
    <section className="page overview-page" aria-labelledby="overview-title">
      <div className="page-heading">
        <div><h1 id="overview-title">{t("nav.overview")}</h1><p>{t("overview.subtitle")}</p></div>
        <span className={`account-tier-badge ${props.account.pro ? "is-pro" : ""}`} aria-label={t("overview.tier", { tier: props.account.pro ? "PRO" : "FREE" })}><i /> {props.account.pro ? "PRO" : "FREE"}</span>
      </div>

      <div className="overview-grid">
        <article className="feature-panel current-theme-panel">
          <div className="panel-label"><span><Icon name="themes" size={16} /> {t("overview.currentTheme")}</span><b className={previewReport ? "is-live" : ""}>{previewReport ? t("overview.running") : t("overview.official")}</b></div>
          {activeTheme ? <ThemeArtwork theme={activeTheme} /> : <ThemeArtwork theme={officialTheme} />}
          <div className="theme-now">
            <div><h2>{activeTheme?.name ?? t("overview.officialTheme")}</h2><p>{activeTheme ? `${activeTheme.author} · v${activeTheme.version}` : t("overview.noTheme")}</p></div>
            {previewReport ? (
              <button className="button secondary" disabled={props.stopping} onClick={props.onRestore}><Icon name="refresh" size={16} />{props.stopping ? t("common.restoring") : t("overview.restore")}</button>
            ) : (
              <button className="button primary" onClick={() => props.onNavigate("themes")}>{t("overview.choose")} <Icon name="arrow" size={16} /></button>
            )}
          </div>
        </article>

        <div className="status-stack">
          <article className="status-panel">
            <div className="status-icon codex"><Logo /></div>
            <div className="status-copy"><span>{t("overview.app")}</span><strong>{installation ? `${installation.appName} ${installation.version}` : t("overview.waitingDetection")}</strong><p>{installation ? t("overview.appReady") : props.detectError || t("overview.detectingApp")}</p></div>
            <span className={`status-light ${installation ? "is-ok" : ""}`} />
          </article>
          <article className="status-panel">
            <div className="status-icon license"><Icon name="shield" /></div>
            <div className="status-copy"><span>{t("overview.accountStatus")}</span><strong>{heartbeat.title}</strong><p>{heartbeat.detail}</p></div>
            <span className={`status-light ${heartbeat.live ? "is-ok" : ""}`} />
          </article>
          <article className="status-panel">
            <div className="status-icon diagnostic"><Icon name="activity" /></div>
            <div className="status-copy"><span>{t("overview.diagnostic")}</span><strong>{diagnosticPassed ? t("overview.allPassed") : diagnosticFailed || props.smokeError ? t("overview.incompatible") : props.testing ? t("overview.diagnosing") : t("overview.pending")}</strong><p>{diagnosticDetail}</p></div>
            <button className="button ghost panel-action" disabled={!installation || props.testing || Boolean(previewReport)} onClick={props.onSmokeTest}>{props.testing ? t("overview.diagnosing") : t("overview.runDiagnostic")}</button>
          </article>
        </div>
      </div>

      <div className="version-grid">
        <article className="version-card"><span>ReTheme</span><div><strong>v{props.runtimeEnvironment?.appVersion ?? "—"}</strong><small>{t("overview.desktopApp")}</small></div></article>
        <article className="version-card"><span>{t("overview.compatibility")}</span><div><strong>{compatibilityTitle}</strong><small>{compatibility?.adapterId ?? t("overview.waitingMatch")}</small></div></article>
        <button className="version-card support-card" onClick={() => void openGitHubRepository()} aria-label={t("overview.supportAction")}>
          <span>{t("overview.support")}</span><div><strong>GitHub Star</strong><small>{t("overview.openSource")}</small><Icon name="external" size={15} /></div>
        </button>
      </div>
    </section>
  );
}

function ThemesPage(props: SharedPageProps & {
  localPreviewing: boolean;
  wallpaperCatalog: WallpaperEngineCatalog | null;
  wallpaperLoading: boolean;
  wallpaperPreviewing: boolean;
  wallpaperError: string;
  wallpaperPreferences: WallpaperPreferences;
  onWallpaperPreferencesChange: (preferences: WallpaperPreferences) => void;
  onReloadWallpapers: () => void;
  installReport: ThemeInstallReport | null;
  installError: string;
}) {
  const { t } = useI18n();
  const [query, setQuery] = useState("");
  const keyword = query.trim().toLowerCase();
  const themes = keyword ? props.themes.filter((theme) => `${theme.name} ${theme.author} ${theme.description}`.toLowerCase().includes(keyword)) : props.themes;
  const supportedWallpapers = props.wallpaperCatalog?.projects.filter((project) => project.supported) ?? [];
  const unsupportedCount = (props.wallpaperCatalog?.projects.length ?? 0) - supportedWallpapers.length;
  const videoCount = supportedWallpapers.filter((project) => project.projectType === "video").length;
  const sceneCount = supportedWallpapers.filter((project) => project.projectType === "scene").length;
  const webCount = supportedWallpapers.filter((project) => project.projectType === "web").length;
  const wallpapersWithoutEngine = supportedWallpapers.filter((project) => !project.requiresWallpaperEngine);
  const wallpapersWithEngine = supportedWallpapers.filter((project) => project.requiresWallpaperEngine);
  const withoutEngineVideoCount = wallpapersWithoutEngine.filter((project) => project.projectType === "video").length;
  const withoutEngineWebCount = wallpapersWithoutEngine.filter((project) => project.projectType === "web").length;
  const withEngineSceneCount = wallpapersWithEngine.filter((project) => project.projectType === "scene").length;
  const { projectPath: wallpaperPath, wallpaperBrightness, interfaceTransparency } = props.wallpaperPreferences;
  const selectedWithoutEngine = wallpapersWithoutEngine.some((project) => project.projectPath === wallpaperPath) ? wallpaperPath : "";
  const selectedWithEngine = wallpapersWithEngine.some((project) => project.projectPath === wallpaperPath) ? wallpaperPath : "";
  return (
    <section className="page themes-page" aria-labelledby="themes-title">
      <div className="page-toolbar">
        <label className="toolbar-search"><span aria-hidden="true">⌕</span><input type="search" placeholder={t("themes.search")} aria-label={t("themes.search")} value={query} onChange={(event) => setQuery(event.target.value)} /></label>
        <button className="button secondary" title={t("themes.localTitle")} disabled={props.testing || props.localPreviewing || Boolean(props.previewReport) || !props.installation} onClick={props.onPreviewLocalTheme}>{props.localPreviewing ? t("common.loading") : t("themes.loadLocal")}</button>
        <span className="toolbar-spacer" />
        <button className="button primary" disabled={!props.previewReport || props.stopping} onClick={props.onRestore}>{props.stopping ? t("common.restoring") : t("themes.restoreOfficial")}</button>
      </div>

      <div className="themes-scroll">
        <section className="wallpaper-import" aria-label={t("wallpaper.title")}>
          <div className="wallpaper-copy">
            <strong>{t("wallpaper.title")}</strong>
            <small>
              {props.wallpaperCatalog
                ? t("wallpaper.found", { count: supportedWallpapers.length, video: videoCount, scene: sceneCount, web: webCount, unsupported: unsupportedCount })
                : t("wallpaper.hint")}
            </small>
          </div>
          <div className="wallpaper-project-groups">
            <label className="wallpaper-project-group">
              <span>
                <strong>{t("wallpaper.groupWithoutEngine")}</strong>
                <small>{t("wallpaper.groupWithoutEngineDetail", { video: withoutEngineVideoCount, web: withoutEngineWebCount })}</small>
              </span>
              <select
                aria-label={t("wallpaper.selectWithoutEngine")}
                value={selectedWithoutEngine}
                disabled={props.wallpaperLoading || props.wallpaperPreviewing || !wallpapersWithoutEngine.length}
                onChange={(event) => props.onWallpaperPreferencesChange({ ...props.wallpaperPreferences, projectPath: event.target.value })}
              >
                <option value="" disabled>{props.wallpaperLoading ? t("wallpaper.scanning") : t("wallpaper.chooseGroup")}</option>
                {wallpapersWithoutEngine.map((project) => (
                  <option value={project.projectPath} key={project.id}>
                    {formatWallpaperProject(project)}
                  </option>
                ))}
              </select>
            </label>
            <label className="wallpaper-project-group">
              <span>
                <strong>{t("wallpaper.groupWithEngine")}</strong>
                <small>{t("wallpaper.groupWithEngineDetail", { scene: withEngineSceneCount })}</small>
              </span>
              <select
                aria-label={t("wallpaper.selectWithEngine")}
                value={selectedWithEngine}
                disabled={props.wallpaperLoading || props.wallpaperPreviewing || !wallpapersWithEngine.length}
                onChange={(event) => props.onWallpaperPreferencesChange({ ...props.wallpaperPreferences, projectPath: event.target.value })}
              >
                <option value="" disabled>{props.wallpaperLoading ? t("wallpaper.scanning") : t("wallpaper.chooseGroup")}</option>
                {wallpapersWithEngine.map((project) => (
                  <option value={project.projectPath} key={project.id}>
                    {formatWallpaperProject(project)}
                  </option>
                ))}
              </select>
              {selectedWithEngine && (
                <small className="wallpaper-desktop-sync-note">
                  {t("wallpaper.desktopSyncNotice")}
                </small>
              )}
            </label>
          </div>
          <label className="wallpaper-opacity wallpaper-brightness">
            <span>{t("wallpaper.brightness")} <output>{t("wallpaper.brightnessValue", { value: wallpaperBrightness })}</output></span>
            <input
              type="range"
              min="0"
              max="100"
              step="1"
              value={wallpaperBrightness}
              aria-label={t("wallpaper.brightness")}
              disabled={props.wallpaperPreviewing}
              onChange={(event) => props.onWallpaperPreferencesChange({
                ...props.wallpaperPreferences,
                wallpaperBrightness: Number(event.target.value),
              })}
            />
          </label>
          <label className="wallpaper-opacity wallpaper-interface-transparency">
            <span>{t("wallpaper.interfaceTransparency")} <output>{t("wallpaper.interfaceTransparencyValue", { value: interfaceTransparency })}</output></span>
            <input
              type="range"
              min="0"
              max="100"
              step="1"
              value={interfaceTransparency}
              aria-label={t("wallpaper.interfaceTransparency")}
              disabled={props.wallpaperPreviewing}
              onChange={(event) => props.onWallpaperPreferencesChange({
                ...props.wallpaperPreferences,
                interfaceTransparency: Number(event.target.value),
              })}
            />
          </label>
          <button className="button ghost" disabled={props.wallpaperLoading || props.wallpaperPreviewing} onClick={props.onReloadWallpapers}>
            {props.wallpaperLoading ? t("wallpaper.scanning") : t("wallpaper.rescan")}
          </button>
          <button
            className="button secondary"
            disabled={props.testing || !wallpaperPath || props.wallpaperPreviewing || Boolean(props.previewReport) || !props.installation}
            onClick={() => props.onPreviewWallpaperEngine(wallpaperPath, {
              wallpaperBrightness,
              interfaceTransparency,
            })}
          >
            {props.wallpaperPreviewing ? t("wallpaper.opening") : t("wallpaper.apply")}
          </button>
        </section>
        {props.wallpaperError && <div className="notice is-error">{props.wallpaperError}</div>}
        {(props.themesError || props.installError || props.installReport) && (
          <div className={`notice ${props.themesError || props.installError ? "is-error" : ""}`}>
            {props.themesError || props.installError || t("themes.installSuccess", { name: props.installReport?.theme.name ?? "", action: props.installReport?.replaced ? t("themes.updated") : t("themes.added") })}
          </div>
        )}

        <div className="section-head"><h1 id="themes-title">{t("themes.installedTitle")}</h1><small>{t("themes.communityHint")}</small></div>
        <div className="theme-grid">
          {themes.map((theme) => {
            const active = props.previewReport?.themeId === theme.id;
            const busy = props.busyThemeId === theme.id;
            const uninstalling = props.uninstallingThemeId === theme.id;
            return (
              <article className={`theme-card ${active ? "is-active" : ""}`} key={theme.id} title={theme.description}>
                <ThemeArtwork theme={theme} />
                <div className="theme-card-meta">
                  <div className="theme-card-name"><h3>{theme.name}</h3><small>{theme.author} · v{theme.version}</small></div>
                  <div className="theme-card-actions">
                    {!theme.builtIn && (
                      <button className="pill danger" disabled={uninstalling || Boolean(props.busyThemeId) || props.stopping} onClick={() => props.onUninstallTheme(theme)} aria-label={t("themes.uninstallLabel", { name: theme.name })}>{uninstalling ? t("common.uninstalling") : t("common.uninstall")}</button>
                    )}
                    {active ? (
                      <button className="pill is-live" disabled={props.stopping || uninstalling} onClick={props.onRestore} aria-label={t("themes.restoreLabel", { name: theme.name })}><i />{props.stopping ? t("common.restoring") : t("themes.inUse")}</button>
                    ) : (
                      <button className="pill" disabled={props.testing || !props.installation || Boolean(props.busyThemeId) || props.stopping || Boolean(props.previewReport) || Boolean(props.uninstallingThemeId)} onClick={() => props.onApplyTheme(theme.id)} aria-label={t("themes.applyLabel", { name: theme.name })}>{busy ? t("common.applying") : t("common.apply")}</button>
                    )}
                  </div>
                </div>
              </article>
            );
          })}
          <button className="theme-card ghost" onClick={() => void openWebsite("/store")}>
            <span>{t("themes.more")}</span>
            <b>{t("themes.browseStore")}</b>
          </button>
        </div>

        {props.previewError && <p className="page-error">{props.previewError}</p>}
      </div>
    </section>
  );
}

function FavoritesPage({ account, accountSync, syncError, installError, themes, onlineInstallingSlug, onInstallOnline }: { account: AccountStatus; accountSync: AccountSync | null; syncError: string; installError: string; themes: ThemeSummary[]; onlineInstallingSlug: string; onInstallOnline: (slug: string) => void }) {
  const { locale, t } = useI18n();
  const [lockedMessage, setLockedMessage] = useState("");
  const favoriteThemes = accountSync?.themes.favorites.map((theme) => localizeAccountTheme(theme, locale)) ?? [];
  const usedThemes = accountSync?.themes.used.map((theme) => localizeAccountTheme(theme, locale)) ?? [];
  return (
    <section className="page favorites-page" aria-labelledby="cloud-favorites-title">
      <div className="page-heading"><h1 id="cloud-favorites-title">{t("nav.favorites")}</h1><p>{t("favorites.subtitle")}</p></div>
      <div className="cloud-library">
        {installError && <div className="notice is-error">{installError}</div>}
        {account.pro ? syncError ? <div className="notice is-error">{syncError}</div> : accountSync ? (
            favoriteThemes.length ? (
              <div className="cloud-theme-list">
                {favoriteThemes.map((theme) => (
                  <CloudThemeCard key={theme.slug} theme={theme} installed={themes.some((installed) => installed.onlineSlug === theme.slug)} installing={onlineInstallingSlug === theme.slug} onInstall={() => onInstallOnline(theme.slug)} />
                ))}
              </div>
            ) : <div className="cloud-empty">{t("favorites.empty")}</div>
          ) : <div className="cloud-empty">{t("favorites.syncing")}</div> : (
            <button className="cloud-empty cloud-locked" onClick={() => setLockedMessage(t("favorites.noAccess"))}>{lockedMessage || t("favorites.view")}</button>
          )}

          {account.pro && Boolean(usedThemes.length) && (
            <div className="recent-themes"><strong>{t("favorites.recent")}</strong><span>{usedThemes.slice(0, 5).map((theme) => theme.name).join(" · ")}</span></div>
          )}
      </div>
    </section>
  );
}

function AccountPage({ account, onManage }: { account: AccountStatus; onManage: () => void }) {
  const { t } = useI18n();
  return (
    <section className="page account-page" aria-labelledby="account-title">
      <div className="page-heading"><h1 id="account-title">{t("nav.account")}</h1><p>{t("account.subtitle")}</p></div>
      <div className="pro-layout">
        <article className="pro-pass">
          <div className="pass-top"><Logo /><span>RETHEME ACCOUNT</span><b>{account.authenticated ? t("account.signedIn") : t("account.signedOut")}</b></div>
          <div className="pass-copy"><small>{t("account.current")}</small><h2>{account.email || t("account.loginReTheme")}</h2></div>
          <div className="pass-foot"><span>{account.pro ? t("account.proSupporter") : account.authenticated ? t("account.free") : t("account.syncAfterLogin")}</span><span>{account.heartbeatState === "online" ? t("account.deviceOnline") : t("account.currentOffline")}</span></div>
        </article>
        <article className="credit-ticket">
          <span className="ticket-hole top" /><span className="ticket-hole bottom" />
          <small>{account.pro ? t("account.proSupporterUpper") : t("account.status")}</small><strong>{account.pro ? t("account.thanks") : account.authenticated ? t("account.connected") : t("account.signedOut")}</strong><p>{account.pro ? t("account.proDetail") : account.authenticated ? t("account.communityFree") : t("account.manageData")}</p><button className="text-link" onClick={onManage}>{t("account.manage")} <Icon name="arrow" size={15} /></button>
        </article>
      </div>
      <article className="pro-details">
        <div><span className="benefit-icon"><Icon name="user" /></span><strong>{t("account.identity")}</strong><p>{account.email || t("account.identityHint")}</p></div>
        <div><span className="benefit-icon"><Icon name="monitor" /></span><strong>{t("account.currentDevice")}</strong><p>{account.deviceName} · {account.heartbeatState === "online" ? t("common.online") : t("common.offline")}</p></div>
        <div><span className="benefit-icon"><Icon name="check" /></span><strong>{account.pro ? t("account.proBenefit") : t("account.communityThemes")}</strong><p>{account.pro ? t("account.proBenefitDetail") : t("account.communityDetail")}</p></div>
      </article>
      <div className="pro-note"><Icon name="shield" size={17} /><span><b>{account.pro ? t("account.proActive") : account.authenticated ? t("account.connected") : t("account.notLoggedIn")}</b>{t("account.unrestricted")}</span><button className="button ghost" onClick={onManage}>{account.authenticated ? t("account.manage") : t("account.login")} <Icon name="user" size={15} /></button></div>
    </section>
  );
}

function DevicesPage({ account, accountSync, syncError }: { account: AccountStatus; accountSync: AccountSync | null; syncError: string }) {
  const { locale, t } = useI18n();
  return (
    <section className="page devices-page" aria-labelledby="devices-title">
      <div className="page-heading"><h1 id="devices-title">{t("nav.devices")}</h1><p>{t("devices.subtitle")}</p></div>
      <button type="button" className={`device-benefit-banner ${account.pro ? "is-pro" : ""}`} onClick={() => void openWebsite("/pricing")}>
        <span className="device-benefit-icon"><Icon name="monitor" size={22} /></span>
        <div><h2>{account.pro ? t("devices.proEnabled") : t("devices.proAvailable")}</h2><p>{account.pro ? t("devices.proEnabledDetail") : t("devices.proAvailableDetail")}</p></div>
        <em>{account.pro ? "PRO" : t("devices.proBenefit")}</em>
      </button>
      <section className="device-history" aria-labelledby="device-history-title">
        <div className="section-head"><h2 id="device-history-title">{t("devices.loginDevices")}</h2>{account.authenticated && accountSync && <small>{t("devices.historyCount", { count: accountSync.devices.length })}</small>}</div>
        {!account.authenticated ? <div className="cloud-empty">{t("devices.signInToView")}</div> : syncError ? <div className="notice is-error">{syncError}</div> : accountSync ? accountSync.devices.length ? (
          <div className="device-list">
            {accountSync.devices.map((device) => (
              <article className="device-history-row" key={device.id}>
                <span className="benefit-icon"><Icon name="monitor" size={16} /></span>
                <div className="device-history-name"><strong>{device.name}</strong><small>{device.current ? t("devices.current") : device.active ? t("devices.active") : t("devices.history")}</small></div>
                <div className="device-history-time"><small>{t("devices.firstLogin")}</small><span>{device.registeredAt ? new Date(device.registeredAt).toLocaleString(locale) : "—"}</span></div>
                <div className="device-history-time"><small>{t("devices.lastOnline")}</small><span>{device.lastSeenAt ? new Date(device.lastSeenAt).toLocaleString(locale) : "—"}</span></div>
                <em className={device.active ? "is-active" : ""}>{device.current ? t("common.localDevice") : device.active ? t("common.online") : t("devices.signedOut")}</em>
              </article>
            ))}
          </div>
        ) : <div className="cloud-empty">{t("devices.empty")}</div> : <div className="cloud-empty">{t("devices.syncing")}</div>}
      </section>
    </section>
  );
}

function SettingsPage({ preferences, onToggle, installation, onDetect, onRestore, stopping, previewReport, runtimeEnvironment, account, appearance, onAppearanceChange, updateRepository, onUpdateRepositoryChange, update, updateChecked, updateChecking, updateInstalling, updateError, onCheckUpdate, onInstallUpdate }: { preferences: DesktopPreferences; onToggle: (key: keyof DesktopPreferences) => void; installation: CodexInstallation | null; onDetect: () => void; onRestore: () => void; stopping: boolean; previewReport: ThemePreviewReport | null; runtimeEnvironment: RuntimeEnvironment | null; account: AccountStatus; appearance: Appearance; onAppearanceChange: (value: Appearance) => void; updateRepository: string; onUpdateRepositoryChange: (value: string) => void; update: DesktopUpdate | null; updateChecked: boolean; updateChecking: boolean; updateInstalling: boolean; updateError: string; onCheckUpdate: () => void; onInstallUpdate: () => void }) {
  const { preference, setPreference, t } = useI18n();
  const heartbeat = heartbeatCopy(account, t);
  const appearanceOptions: Array<{ value: Appearance; label: string }> = [{ value: "system", label: t("common.system") }, { value: "light", label: t("common.light") }, { value: "dark", label: t("common.dark") }];
  const localeOptions = [{ value: "system" as const, label: t("common.system") }, { value: "zh-CN" as const, label: t("common.chinese") }, { value: "en" as const, label: t("common.english") }];
  return (
    <section className="page settings-page" aria-labelledby="settings-title">
      <div className="page-heading"><h1 id="settings-title">{t("nav.settings")}</h1><p>{t("settings.subtitle")}</p></div>
      <div className="settings-scroll">
        <div className="settings-group">
          <h2>{t("settings.general")}</h2>
          <div className="setting-row"><div><strong>{t("settings.language")}</strong><p>{t("settings.languageDetail")}</p></div>
            <div className="segmented" role="radiogroup" aria-label={t("settings.languageMode")}>
              {localeOptions.map((option) => <button key={option.value} role="radio" aria-checked={preference === option.value} className={preference === option.value ? "is-active" : ""} onClick={() => setPreference(option.value)}>{option.label}</button>)}
            </div>
          </div>
          <div className="setting-row"><div><strong>{t("settings.appearance")}</strong><p>{t("settings.appearanceDetail")}</p></div>
            <div className="segmented" role="radiogroup" aria-label={t("settings.appearanceMode")}>
              {appearanceOptions.map((option) => (
                <button key={option.value} role="radio" aria-checked={appearance === option.value} className={appearance === option.value ? "is-active" : ""} onClick={() => onAppearanceChange(option.value)}>{option.label}</button>
              ))}
            </div>
          </div>
          <div className="setting-row"><div><strong>{t("settings.launchAtLogin")}</strong><p>{t("settings.launchAtLoginDetail")}</p></div><Switch checked={preferences.launchAtLogin} label={t("settings.launchAtLogin")} onChange={() => onToggle("launchAtLogin")} /></div>
          <div className="setting-row"><div><strong>{t("settings.hideToTray")}</strong><p>{t("settings.hideToTrayDetail")}</p></div><Switch checked={preferences.hideToTray} disabled label={t("settings.hideToTray")} onChange={() => onToggle("hideToTray")} /></div>
          <div className="setting-row"><div><strong>{t("settings.autoDetect")}</strong><p>{t("settings.autoDetectDetail")}</p></div><Switch checked={preferences.autoDetectCodex} label={t("settings.autoDetect")} onChange={() => onToggle("autoDetectCodex")} /></div>
          <div className="setting-row"><div><strong>{t("settings.lowMemory")}</strong><p>{t("settings.lowMemoryDetail")}</p></div><Switch checked={preferences.lowMemoryMode} label={t("settings.lowMemory")} onChange={() => onToggle("lowMemoryMode")} /></div>
        </div>
        <div className="settings-group">
          <h2>{t("settings.update")}</h2>
          <div className="setting-row"><div><strong>{t("settings.autoUpdate")}</strong><p>{t("settings.autoUpdateDetail")}</p></div><Switch checked={preferences.autoUpdate} label={t("settings.autoUpdate")} onChange={() => onToggle("autoUpdate")} /></div>
          <div className="setting-row"><div><strong>{t("settings.updateRepository")}</strong><p>{t("settings.updateRepositoryDetail")}</p></div><input className="repository-input" value={updateRepository} onChange={(event) => onUpdateRepositoryChange(event.target.value)} placeholder="owner/retheme-we" aria-label={t("settings.updateRepository")} spellCheck={false} /></div>
          <div className="setting-row"><div><strong>{t("settings.currentVersion")}</strong><p>{update ? t("settings.available", { version: update.version }) : updateChecked ? t("settings.latestVersion", { version: runtimeEnvironment?.appVersion ?? "—" }) : t("settings.stableChannel", { version: runtimeEnvironment?.appVersion ?? "—" })}</p>{update?.body && <p className="update-notes">{update.body}</p>}{updateError && <p className="error-line">{updateError}</p>}</div><button className={`button ${update ? "secondary" : "ghost"}`} disabled={updateChecking || updateInstalling} onClick={update ? onInstallUpdate : onCheckUpdate}>{updateInstalling ? t("settings.installing") : updateChecking ? t("settings.checking") : update ? t("settings.installUpdate") : t("settings.checkNow")}</button></div>
        </div>
        <div className="settings-group">
          <h2>{t("settings.themeService")}</h2>
          <div className="setting-row"><div><strong>{t("overview.accountStatus")}</strong><p>{t("settings.accountSync")}</p></div><span className={`setting-state ${heartbeat.live ? "" : "is-offline"}`}><i /> {heartbeat.title}</span></div>
          <div className="setting-row"><div><strong>{t("settings.restoreOfficial")}</strong><p>{previewReport ? t("settings.themeRunning") : t("settings.noThemeRunning")}</p></div><button className="button secondary" disabled={!previewReport || stopping} onClick={onRestore}><Icon name="refresh" size={15} /> {stopping ? t("common.restoring") : t("settings.restoreNow")}</button></div>
        </div>
        <div className="settings-group">
          <h2>{t("settings.diagnostics")}</h2>
          <div className="setting-row"><div><strong>{t("settings.appPath")}</strong><p className="path-text">{installation?.path ?? t("settings.appNotFound")}</p></div><button className="button ghost" onClick={onDetect}>{t("settings.redetect")}</button></div>
        </div>
      </div>
    </section>
  );
}

function App() {
  const { locale, t } = useI18n();
  const [page, setPage] = useState<PageId>("overview");
  const [installation, setInstallation] = useState<CodexInstallation | null>(null);
  const [detectError, setDetectError] = useState("");
  const [detecting, setDetecting] = useState(false);
  const [themes, setThemes] = useState<ThemeSummary[]>([]);
  const [themesError, setThemesError] = useState("");
  const [localPreviewing, setLocalPreviewing] = useState(false);
  const [wallpaperCatalog, setWallpaperCatalog] = useState<WallpaperEngineCatalog | null>(null);
  const [wallpaperLoading, setWallpaperLoading] = useState(false);
  const [wallpaperPreviewing, setWallpaperPreviewing] = useState(false);
  const [wallpaperError, setWallpaperError] = useState("");
  const [installReport, setInstallReport] = useState<ThemeInstallReport | null>(null);
  const [installError, setInstallError] = useState("");
  const [testing, setTesting] = useState(false);
  const [smokeReport, setSmokeReport] = useState<SmokeTestReport | null>(null);
  const [smokeError, setSmokeError] = useState("");
  const [busyThemeId, setBusyThemeId] = useState("");
  const [uninstallingThemeId, setUninstallingThemeId] = useState("");
  const [stopping, setStopping] = useState(false);
  const [previewReport, setPreviewReport] = useState<ThemePreviewReport | null>(null);
  const [previewStatusReady, setPreviewStatusReady] = useState(false);
  const previewMutation = useRef(0);
  const [previewError, setPreviewError] = useState("");
  const [runtimeEnvironment, setRuntimeEnvironment] = useState<RuntimeEnvironment | null>(null);
  const [accountOpen, setAccountOpen] = useState(false);
  const [oauthMessage, setOAuthMessage] = useState("");
  const [account, setAccount] = useState<AccountStatus>(INITIAL_ACCOUNT);
  const [accountSync, setAccountSync] = useState<AccountSync | null>(null);
  const [syncError, setSyncError] = useState("");
  const [onlineInstallingSlug, setOnlineInstallingSlug] = useState("");
  const [preferences, setPreferences] = useState(loadPreferences);
  const [appearance, setAppearance] = useState<Appearance>(loadAppearance);
  const [updateRepository, setUpdateRepository] = useState(loadUpdateRepository);
  const [wallpaperPreferences, setWallpaperPreferences] = useState(loadWallpaperPreferences);
  const [wallpaperControlsReady, setWallpaperControlsReady] = useState(false);
  const [update, setUpdate] = useState<DesktopUpdate | null>(null);
  const [updateChecked, setUpdateChecked] = useState(false);
  const [updateChecking, setUpdateChecking] = useState(false);
  const [updateInstalling, setUpdateInstalling] = useState(false);
  const [updateError, setUpdateError] = useState("");
  const selfCheckVersion = useRef("");

  function localizedError(error: unknown): string {
    const message = String(error);
    return message.includes("RETHEME_AUTH_REQUIRED") ? t("themes.signInToInstall") : message;
  }

  useEffect(() => {
    void syncTrayLocale(locale);
    void syncThemeLocale(locale);
  }, [locale]);

  useEffect(() => {
    if (appearance === "system") delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = appearance;
    window.localStorage.setItem(APPEARANCE_KEY, appearance);
  }, [appearance]);

  useEffect(() => {
    window.localStorage.setItem(PREFERENCES_KEY, JSON.stringify(preferences));
  }, [preferences]);

  useEffect(() => {
    window.localStorage.setItem(UPDATE_REPOSITORY_KEY, updateRepository.trim());
    setUpdate(null);
    setUpdateChecked(false);
    setUpdateError("");
  }, [updateRepository]);

  useEffect(() => {
    window.localStorage.setItem(WALLPAPER_PREFERENCES_KEY, JSON.stringify(wallpaperPreferences));
  }, [wallpaperPreferences]);

  useEffect(() => {
    if (!isDesktopRuntime()) {
      setWallpaperControlsReady(true);
      return;
    }
    let disposed = false;
    void getWallpaperControlPreferences().then(async (saved) => {
      if (disposed) return;
      if (saved) {
        setWallpaperPreferences((current) => ({ ...current, ...saved }));
      } else {
        await updateWallpaperControls({
          wallpaperBrightness: wallpaperPreferences.wallpaperBrightness,
          interfaceTransparency: wallpaperPreferences.interfaceTransparency,
        });
      }
    }).catch(() => {}).finally(() => {
      if (!disposed) setWallpaperControlsReady(true);
    });
    return () => { disposed = true; };
  }, []);

  useEffect(() => {
    if (!isDesktopRuntime() || !wallpaperControlsReady) return;
    const timer = window.setTimeout(() => {
      void updateWallpaperControls({
        wallpaperBrightness: wallpaperPreferences.wallpaperBrightness,
        interfaceTransparency: wallpaperPreferences.interfaceTransparency,
      }).catch(() => {});
    }, 120);
    return () => window.clearTimeout(timer);
  }, [
    wallpaperControlsReady,
    wallpaperPreferences.wallpaperBrightness,
    wallpaperPreferences.interfaceTransparency,
  ]);

  function enterLowMemoryMode() {
    if (isDesktopRuntime() && preferences.lowMemoryMode) {
      void releaseUiMemory().catch(() => {});
    }
  }

  async function detect() {
    setDetecting(true);
    setDetectError("");
    try {
      setInstallation(await detectCodex());
    } catch (error) {
      setDetectError(String(error));
    } finally {
      setDetecting(false);
    }
  }

  async function checkUpdate(silent = false) {
    if (!isDesktopRuntime()) return;
    const repository = updateRepository.trim();
    if (!repository) {
      if (!silent) setUpdateError(t("settings.configureUpdateRepository"));
      return;
    }
    if (!silent) {
      setUpdateChecked(false);
      setUpdateChecking(true);
    }
    setUpdateError("");
    try {
      const availableUpdate = await checkForUpdate(repository);
      setUpdate(availableUpdate);
      if (!silent && !availableUpdate) setUpdateChecked(true);
    } catch (error) {
      setUpdateError(String(error));
    } finally {
      if (!silent) setUpdateChecking(false);
    }
  }

  async function installUpdate() {
    if (!update) return;
    setUpdateInstalling(true);
    setUpdateError("");
    try {
      if (previewReport) await restoreOfficialTheme();
      await update.install();
    } catch (error) {
      setUpdateError(String(error));
      setUpdateInstalling(false);
    }
  }

  async function loadThemeLibrary() {
    setThemesError("");
    try {
      setThemes(await listThemes());
    } catch (error) {
      setThemesError(String(error));
    }
  }

  async function installOnlineTheme(slug: string) {
    setOnlineInstallingSlug(slug);
    setInstallError("");
    setInstallReport(null);
    try {
      const result = await downloadOnlineTheme(slug);
      setInstallReport(result);
      await loadThemeLibrary();
      if (account.authenticated) setAccountSync(await getAccountSync());
    } catch (error) {
      setInstallError(localizedError(error));
    } finally {
      setOnlineInstallingSlug("");
    }
  }

  async function startTheme(themeId: string) {
    if (testing || localPreviewing || wallpaperPreviewing) return;
    previewMutation.current += 1;
    setBusyThemeId(themeId);
    setPreviewError("");
    try {
      const report = await applyTheme(themeId, locale);
      setPreviewReport(report);
      enterLowMemoryMode();
    } catch (error) {
      setPreviewError(String(error));
    } finally {
      setBusyThemeId("");
    }
  }

  async function previewLocalTheme() {
    if (testing || localPreviewing || wallpaperPreviewing) return;
    previewMutation.current += 1;
    setLocalPreviewing(true);
    setPreviewError("");
    try {
      const report = await chooseAndPreviewLocalTheme(locale);
      if (report) {
        setPreviewReport(report);
        enterLowMemoryMode();
      }
    } catch (error) {
      setPreviewError(String(error));
    } finally {
      setLocalPreviewing(false);
    }
  }

  async function loadWallpaperCatalog() {
    setWallpaperLoading(true);
    setWallpaperError("");
    try {
      const catalog = await getWallpaperEngineCatalog();
      setWallpaperCatalog(catalog);
      const supported = catalog.projects.filter((project) => project.supported);
      setWallpaperPreferences((current) => ({
        ...current,
        projectPath: supported.some((project) => project.projectPath === current.projectPath)
          ? current.projectPath
          : supported[0]?.projectPath ?? "",
      }));
    } catch (error) {
      setWallpaperError(String(error));
    } finally {
      setWallpaperLoading(false);
    }
  }

  async function previewWallpaperEngine(projectPath: string, controls: WallpaperControls) {
    if (testing || localPreviewing || wallpaperPreviewing) return;
    previewMutation.current += 1;
    setWallpaperPreviewing(true);
    setWallpaperError("");
    setPreviewError("");
    try {
      const report = await previewWallpaperEngineProject(projectPath, locale, controls);
      setPreviewReport(report);
      enterLowMemoryMode();
    } catch (error) {
      setWallpaperError(String(error));
    } finally {
      setWallpaperPreviewing(false);
    }
  }

  async function removeTheme(theme: ThemeSummary) {
    if (!window.confirm(t("themes.confirmUninstall", { name: theme.name }))) return;
    previewMutation.current += 1;
    setUninstallingThemeId(theme.id);
    setPreviewError("");
    try {
      await uninstallTheme(theme.id);
      if (previewReport?.themeId === theme.id) setPreviewReport(null);
      await loadThemeLibrary();
    } catch (error) {
      setPreviewError(String(error));
      await loadThemeLibrary();
    } finally {
      setUninstallingThemeId("");
    }
  }

  async function stopTheme() {
    previewMutation.current += 1;
    setStopping(true);
    setPreviewError("");
    try {
      await restoreOfficialTheme();
      setPreviewReport(null);
    } catch (error) {
      setPreviewError(String(error));
    } finally {
      setStopping(false);
    }
  }

  async function smokeTest(_automatic = false) {
    setTesting(true);
    setSmokeError("");
    setSmokeReport(null);
    try {
      const report = await runSmokeTest();
      setSmokeReport(report);
      window.localStorage.setItem(COMPATIBILITY_CACHE_KEY, JSON.stringify({
        appVersion: report.appVersion,
        report,
      } satisfies CachedCompatibilityCheck));
    } catch (error) {
      const message = String(error);
      setSmokeError(message);
      if (installation?.version) {
        window.localStorage.setItem(COMPATIBILITY_CACHE_KEY, JSON.stringify({
          appVersion: installation.version,
          error: message,
        } satisfies CachedCompatibilityCheck));
      }
    } finally {
      setTesting(false);
    }
  }

  useEffect(() => {
    void detect();
    void loadThemeLibrary();
    void loadWallpaperCatalog();
    void getAccountStatus().then(setAccount);
    void getRuntimeEnvironment().then(setRuntimeEnvironment).catch(() => {});
  }, []);

  useEffect(() => {
    if (preferences.autoUpdate) void checkUpdate(true);
  }, [preferences.autoUpdate, updateRepository]);

  useEffect(() => {
    if (!isDesktopRuntime() || !installation || !previewStatusReady || previewReport || testing
      || Boolean(busyThemeId) || localPreviewing || wallpaperPreviewing || stopping) return;
    const cached = loadCompatibilityCheck(installation.version);
    if (cached) {
      setSmokeReport(cached.report ?? null);
      setSmokeError(cached.error ?? "");
      return;
    }
    if (selfCheckVersion.current === installation.version) return;
    const timer = window.setTimeout(() => {
      selfCheckVersion.current = installation.version;
      void smokeTest(true);
    }, AUTOMATIC_SELF_CHECK_DELAY_MS);
    return () => window.clearTimeout(timer);
  }, [installation?.version, previewStatusReady, previewReport, testing, busyThemeId, localPreviewing, wallpaperPreviewing, stopping]);

  useEffect(() => {
    if (!account.authenticated) {
      setAccountSync(null);
      setSyncError("");
      return;
    }
    let disposed = false;
    setSyncError("");
    void getAccountSync().then((result) => {
      if (!disposed) setAccountSync(result);
    }).catch((error) => {
      if (!disposed) setSyncError(String(error));
    });
    return () => { disposed = true; };
  }, [account.authenticated, account.pro, account.deviceGeneration]);

  useEffect(() => {
    if (!isDesktopRuntime()) return;
    let disposed = false;
    const syncPreviewStatus = async () => {
      const mutation = previewMutation.current;
      try {
        const [status, controls] = await Promise.all([
          getThemePreviewStatus(),
          getWallpaperControlPreferences(),
        ]);
        if (!disposed && mutation === previewMutation.current) setPreviewReport(status);
        if (!disposed && controls) {
          setWallpaperPreferences((current) => current.wallpaperBrightness === controls.wallpaperBrightness
            && current.interfaceTransparency === controls.interfaceTransparency
            ? current
            : { ...current, ...controls });
        }
      } catch {}
      finally {
        if (!disposed) setPreviewStatusReady(true);
      }
    };
    void syncPreviewStatus();
    const timer = window.setInterval(() => void syncPreviewStatus(), 1_000);
    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, []);

  useEffect(() => {
    if (!isDesktopRuntime()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    const handleUrls = async (urls: string[]) => {
      for (const rawUrl of urls) {
        try {
          const url = new URL(rawUrl);
          if (url.protocol === "retheme:" && url.hostname === "auth" && url.pathname === "/callback") {
            setAccountOpen(true);
            try {
              const error = url.searchParams.get("error");
              const code = url.searchParams.get("code");
              const state = url.searchParams.get("state");
              if (error) {
                setOAuthMessage(error);
              } else if (code && state) {
                setOAuthMessage(t("oauth.completing"));
                setAccount(await completeOAuthLogin(code, state));
                setOAuthMessage("");
                setAccountOpen(false);
              } else {
                setOAuthMessage(t("oauth.invalidCallback"));
              }
            } catch (error) {
              setOAuthMessage(String(error));
            }
            continue;
          }
          const slug = url.protocol === "retheme:" && url.hostname === "theme"
            ? url.pathname.slice(1)
            : "";
          if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(slug)) continue;
          setPage("themes");
          setInstallError("");
          const result = await downloadOnlineTheme(slug);
          setInstallReport(result);
          setAccount(await getAccountStatus());
          await loadThemeLibrary();
        } catch (error) {
          setInstallError(localizedError(error));
        }
      }
    };
    void import("@tauri-apps/plugin-deep-link").then(async ({ getCurrent, onOpenUrl }) => {
      const current = await getCurrent();
      if (current) await handleUrls(current);
      const cleanup = await onOpenUrl((urls) => void handleUrls(urls));
      if (disposed) cleanup();
      else unlisten = cleanup;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [t]);

  const localizedThemes = useMemo(() => themes.map((theme) => localizeTheme(theme, locale)), [locale, themes]);
  const localizedPreviewReport = useMemo(() => previewReport ? { ...previewReport, theme: localizeTheme(previewReport.theme, locale) } : null, [locale, previewReport]);
  const activeTheme = localizedPreviewReport?.theme ?? localizedThemes.find((theme) => theme.id === localizedPreviewReport?.themeId);
  const navSections: Array<{ key: "workspace" | "accountSection"; items: Array<{ id: PageId; label: string }> }> = [
    { key: "workspace", items: [{ id: "overview", label: t("nav.overview") }, { id: "themes", label: t("nav.themes") }, { id: "favorites", label: t("nav.favorites") }] },
    { key: "accountSection", items: [{ id: "account", label: t("nav.account") }, { id: "devices", label: t("nav.devices") }, { id: "settings", label: t("nav.settings") }] },
  ];
  const sharedProps: SharedPageProps = {
    account,
    installation,
    detectError,
    detecting,
    themes: localizedThemes,
    themesError,
    previewReport: localizedPreviewReport,
    previewError,
    activeTheme,
    busyThemeId,
    uninstallingThemeId,
    stopping,
    testing,
    onDetect: () => void detect(),
    onApplyTheme: (themeId) => void startTheme(themeId),
    onUninstallTheme: (theme) => void removeTheme(theme),
    onPreviewLocalTheme: () => void previewLocalTheme(),
    onPreviewWallpaperEngine: (projectPath, controls) => void previewWallpaperEngine(projectPath, controls),
    onRestore: () => void stopTheme(),
    onNavigate: setPage,
  };

  return (
    <main className="app-shell">
      <aside className="app-sidebar">
        <button className="brand" onClick={() => setPage("overview")} aria-label={t("brand.backOverview")}><Logo /><span><strong>ReTheme WE</strong><small>{t("brand.subtitle")}</small></span></button>
        <nav className="side-nav" aria-label={t("nav.main")}>
          {navSections.map((section) => (
            <div className="nav-section" key={section.key}>
              <span className="nav-label">{t(`nav.${section.key}`)}</span>
              {section.items.map((item) => (
                <button key={item.id} className={`nav-item ${page === item.id ? "is-active" : ""}`} aria-current={page === item.id ? "page" : undefined} onClick={() => setPage(item.id)}>
                  <Icon name={item.id} size={16} />{item.label}
                  {item.id === "themes" && localizedThemes.length > 0 && <em className="nav-badge" aria-hidden="true">{localizedThemes.length}</em>}
                  {item.id === "favorites" && account.pro && Boolean(accountSync?.themes.favorites.length) && <em className="nav-badge" aria-hidden="true">{accountSync?.themes.favorites.length}</em>}
                </button>
              ))}
              {section.key === "workspace" && (
                <button className="nav-item" onClick={() => void openWebsite("/store")}>
                  <Icon name="store" size={16} />{t("nav.store")}<span className="nav-external"><Icon name="external" size={12} /></span>
                </button>
              )}
            </div>
          ))}
        </nav>
        <div className="sidebar-foot">
          <div className={`codex-chip ${installation ? "is-ok" : ""}`}><i />{installation ? t("sidebar.connected") : detecting ? t("sidebar.detecting") : t("sidebar.disconnected")}</div>
          <button className={`account-button ${accountOpen ? "is-open" : ""}`} onClick={() => setAccountOpen(true)} aria-expanded={accountOpen}><span className="avatar">{account.email?.slice(0, 2).toUpperCase() || "RT"}</span><span><strong>{account.email || t("account.loginReTheme")}</strong>{account.authenticated && <small><i className={account.heartbeatState === "online" ? "is-live" : ""} /> {heartbeatCopy(account, t).title}</small>}</span>{account.pro && <em className="pro-tag">PRO</em>}</button>
        </div>
      </aside>

      <div className="app-content">
        {page === "overview" && <OverviewPage {...sharedProps} runtimeEnvironment={runtimeEnvironment} smokeReport={smokeReport} smokeError={smokeError} testing={testing} onSmokeTest={() => void smokeTest(false)} />}
        {page === "themes" && <ThemesPage {...sharedProps} localPreviewing={localPreviewing} wallpaperCatalog={wallpaperCatalog} wallpaperLoading={wallpaperLoading} wallpaperPreviewing={wallpaperPreviewing} wallpaperError={wallpaperError} wallpaperPreferences={wallpaperPreferences} onWallpaperPreferencesChange={setWallpaperPreferences} onReloadWallpapers={() => void loadWallpaperCatalog()} installReport={installReport} installError={installError} />}
        {page === "favorites" && <FavoritesPage account={account} accountSync={accountSync} syncError={syncError} installError={installError} themes={localizedThemes} onlineInstallingSlug={onlineInstallingSlug} onInstallOnline={(slug) => void installOnlineTheme(slug)} />}
        {page === "account" && <AccountPage account={account} onManage={() => setAccountOpen(true)} />}
        {page === "devices" && <DevicesPage account={account} accountSync={accountSync} syncError={syncError} />}
        {page === "settings" && <SettingsPage preferences={preferences} onToggle={(key) => setPreferences((current) => ({ ...current, [key]: !current[key] }))} installation={installation} onDetect={() => void detect()} onRestore={() => void stopTheme()} stopping={stopping} previewReport={previewReport} runtimeEnvironment={runtimeEnvironment} account={account} appearance={appearance} onAppearanceChange={setAppearance} updateRepository={updateRepository} onUpdateRepositoryChange={setUpdateRepository} update={update} updateChecked={updateChecked} updateChecking={updateChecking} updateInstalling={updateInstalling} updateError={updateError} onCheckUpdate={() => void checkUpdate()} onInstallUpdate={() => void installUpdate()} />}
      </div>

      <AccountDialog open={accountOpen} status={account} notice={oauthMessage} onClose={() => setAccountOpen(false)} onChange={setAccount} />

      <footer className="app-statusbar">
        <span><i className={installation ? "is-ok" : ""} /> {isDesktopRuntime() ? t("status.localChannel") : t("status.demoData")}</span>
        <span>{localizedPreviewReport ? `${localizedPreviewReport.source === "wallpaperEngine" ? t("status.wallpaperEngine") : localizedPreviewReport.source === "localDevelopment" ? t("status.localPreview") : t("status.themeRunning")} · ${activeTheme?.name ?? localizedPreviewReport.themeId}` : t("overview.official")}</span>
        <span>{t("status.account", { status: heartbeatCopy(account, t).title })}</span>
        <span>ReTheme {runtimeEnvironment?.appVersion ?? "—"}</span>
      </footer>
    </main>
  );
}

export default App;
