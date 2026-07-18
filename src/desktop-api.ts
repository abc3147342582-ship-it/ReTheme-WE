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

export async function checkForUpdate(): Promise<DesktopUpdate | null> {
  if (!isDesktopRuntime()) return null;
  const { check } = await import("@tauri-apps/plugin-updater");
  const update = await check();
  if (!update) return null;
  return {
    currentVersion: update.currentVersion,
    version: update.version,
    body: update.body,
    install: async () => {
      await update.downloadAndInstall();
      const { relaunch } = await import("@tauri-apps/plugin-process");
      await relaunch();
    },
  };
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
