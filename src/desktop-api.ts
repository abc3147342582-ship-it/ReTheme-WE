import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  createDemoPreview,
  DEMO_INSTALLATION,
  DEMO_RUNTIME_ENVIRONMENT,
  DEMO_SMOKE_REPORT,
  DEMO_THEMES,
} from "./demo-data";
import type {
  CodexInstallation,
  SmokeTestReport,
  ThemeInstallReport,
  ThemePreviewReport,
  RuntimeEnvironment,
  ThemeSummary,
  AccountStatus,
  AccountSync,
  WallpaperEngineCatalog,
  WallpaperControls,
} from "./types";
import { translate } from "./i18n";

export const isDesktopRuntime = () => "__TAURI_INTERNALS__" in window;

export async function detectCodex(): Promise<CodexInstallation> {
  if (!isDesktopRuntime()) return DEMO_INSTALLATION;
  return invoke<CodexInstallation>("detect_codex");
}

export async function listThemes(): Promise<ThemeSummary[]> {
  if (!isDesktopRuntime()) return DEMO_THEMES;
  return invoke<ThemeSummary[]>("list_themes");
}

export async function runSmokeTest(): Promise<SmokeTestReport> {
  if (!isDesktopRuntime()) return DEMO_SMOKE_REPORT;
  return invoke<SmokeTestReport>("run_cdp_smoke_test");
}

export async function uninstallTheme(themeId: string): Promise<boolean> {
  if (!isDesktopRuntime()) return true;
  return invoke<boolean>("uninstall_theme", { themeId });
}

export async function applyTheme(themeId: string, locale: string): Promise<ThemePreviewReport> {
  if (!isDesktopRuntime()) return createDemoPreview(themeId);
  return invoke<ThemePreviewReport>("start_theme_preview", { themeId, locale });
}

export async function chooseAndPreviewLocalTheme(locale: string): Promise<ThemePreviewReport | null> {
  if (!isDesktopRuntime()) return null;
  const themePath = await open({
    title: translate("file.localThemeTitle"),
    multiple: false,
    directory: true,
  });
  if (!themePath) return null;
  return invoke<ThemePreviewReport>("start_local_theme_preview", { themePath, locale });
}

export async function getWallpaperEngineCatalog(): Promise<WallpaperEngineCatalog> {
  if (!isDesktopRuntime()) return { root: "", projects: [] };
  return invoke<WallpaperEngineCatalog>("wallpaper_engine_catalog");
}

export async function previewWallpaperEngineProject(projectPath: string, locale: string, controls: WallpaperControls): Promise<ThemePreviewReport> {
  if (!isDesktopRuntime()) throw new Error("浏览器预览不能加载 Wallpaper Engine 壁纸");
  return invoke<ThemePreviewReport>("start_wallpaper_engine_preview", { projectPath, locale, ...controls });
}

export async function getWallpaperControlPreferences(): Promise<WallpaperControls | null> {
  if (!isDesktopRuntime()) return null;
  return invoke<WallpaperControls | null>("wallpaper_control_preferences");
}

export async function updateWallpaperControls(controls: WallpaperControls): Promise<WallpaperControls> {
  if (!isDesktopRuntime()) return controls;
  return invoke<WallpaperControls>("update_wallpaper_controls", controls);
}

export async function restoreOfficialTheme(): Promise<boolean> {
  if (!isDesktopRuntime()) return true;
  return invoke<boolean>("stop_theme_preview");
}

export async function getThemePreviewStatus(): Promise<ThemePreviewReport | null> {
  if (!isDesktopRuntime()) return null;
  return invoke<ThemePreviewReport | null>("theme_preview_status");
}

export async function getRuntimeEnvironment(): Promise<RuntimeEnvironment> {
  if (!isDesktopRuntime()) return DEMO_RUNTIME_ENVIRONMENT;
  return invoke<RuntimeEnvironment>("runtime_environment");
}

export type DesktopUpdate = {
  currentVersion: string;
  version: string;
  body?: string;
  install: () => Promise<void>;
};

type GithubUpdateInfo = Omit<DesktopUpdate, "install">;

export async function checkForUpdate(repository: string): Promise<DesktopUpdate | null> {
  if (!isDesktopRuntime()) return null;
  const info = await invoke<GithubUpdateInfo | null>("check_github_update", { repository });
  if (!info) return null;
  return {
    ...info,
    install: async () => {
      const installed = await invoke<boolean>("install_github_update", { repository });
      if (!installed) throw new Error("更新版本已发生变化，请重新检查");
      const { relaunch } = await import("@tauri-apps/plugin-process");
      await relaunch();
    },
  };
}

export async function releaseUiMemory(): Promise<void> {
  if (!isDesktopRuntime()) return;
  await invoke("release_ui_memory");
}

export async function syncTrayLocale(locale: string): Promise<void> {
  if (!isDesktopRuntime()) return;
  await invoke("sync_tray_locale", { locale });
}

export async function syncThemeLocale(locale: string): Promise<void> {
  if (!isDesktopRuntime()) return;
  await invoke("sync_theme_locale", { locale });
}

export async function openWebsite(path = ""): Promise<void> {
  const url = `https://retheme.app${path}`;
  await openExternalUrl(url);
}

export async function openGitHubRepository(): Promise<void> {
  await openExternalUrl("https://github.com/duxweb/ReTheme");
}

async function openExternalUrl(url: string): Promise<void> {
  if (!isDesktopRuntime()) {
    window.open(url, "_blank", "noopener,noreferrer");
    return;
  }
  const { openUrl } = await import("@tauri-apps/plugin-opener");
  await openUrl(url);
}

const DEMO_ACCOUNT_STATUS: AccountStatus = {
  authenticated: true,
  email: "hello@retheme.app",
  pro: true,
  deviceName: "Browser Preview",
  deviceGeneration: 1,
  heartbeatState: "online",
  lastHeartbeatAt: Math.floor(Date.now() / 1000),
  leaseExpiresAt: new Date(Date.now() + 86_400_000).toISOString(),
  entitlements: [],
  trials: [],
};

export async function getAccountStatus(): Promise<AccountStatus> {
  if (!isDesktopRuntime()) return DEMO_ACCOUNT_STATUS;
  return invoke<AccountStatus>("account_status");
}

export async function getAccountSync(): Promise<AccountSync> {
  if (!isDesktopRuntime()) {
    return {
      devices: [{ id: "browser-preview", name: "Browser Preview", current: true, active: true, registeredAt: new Date().toISOString(), lastSeenAt: new Date().toISOString() }],
      themes: { favorites: [], used: [] },
    };
  }
  return invoke<AccountSync>("account_sync");
}

export async function loginAccount(email: string, password: string): Promise<AccountStatus> {
  if (!isDesktopRuntime()) return DEMO_ACCOUNT_STATUS;
  return invoke<AccountStatus>("account_login", { email, password });
}

export type OAuthProvider = "github" | "linuxdo";

export async function startOAuthLogin(provider: OAuthProvider): Promise<void> {
  if (!isDesktopRuntime()) {
    window.open(`https://retheme.app/account`, "_blank", "noopener,noreferrer");
    return;
  }
  const result = await invoke<{ authorizeUrl: string }>("account_oauth_start", { provider });
  const { openUrl } = await import("@tauri-apps/plugin-opener");
  await openUrl(result.authorizeUrl);
}

export async function completeOAuthLogin(code: string, state: string): Promise<AccountStatus> {
  if (!isDesktopRuntime()) return DEMO_ACCOUNT_STATUS;
  return invoke<AccountStatus>("account_oauth_complete", { code, state });
}

export async function requestRegisterCode(email: string): Promise<string | undefined> {
  if (!isDesktopRuntime()) return "123456";
  return (await invoke<string | null>("account_request_register_code", { email })) ?? undefined;
}

export async function registerAccount(email: string, code: string, password: string): Promise<AccountStatus> {
  if (!isDesktopRuntime()) return DEMO_ACCOUNT_STATUS;
  return invoke<AccountStatus>("account_register", { email, code, password });
}

export async function logoutAccount(): Promise<AccountStatus> {
  if (!isDesktopRuntime()) return { ...DEMO_ACCOUNT_STATUS, authenticated: false, email: undefined, pro: false, heartbeatState: "offline" };
  return invoke<AccountStatus>("account_logout");
}

export async function redeemCdk(code: string): Promise<AccountStatus> {
  if (!isDesktopRuntime()) return DEMO_ACCOUNT_STATUS;
  return invoke<AccountStatus>("account_redeem_cdk", { code });
}

export async function downloadOnlineTheme(slug: string): Promise<ThemeInstallReport> {
  if (!isDesktopRuntime()) throw new Error(translate("error.browserDownload"));
  return invoke<ThemeInstallReport>("download_online_theme", { slug });
}
