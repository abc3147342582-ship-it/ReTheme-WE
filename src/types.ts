export type PageId = "overview" | "themes" | "favorites" | "account" | "devices" | "settings";

export type CodexInstallation = {
  appName: string;
  path: string;
  executable: string;
  bundleId: string;
  version: string;
};

export type SmokeTestReport = {
  appVersion: string;
  browserVersion: string;
  adapterId: string;
  versionMatched: boolean;
  adapterCompatible: boolean;
  missingAdapterProbes: string[];
  compatible: boolean;
  port: number;
  targetTitle: string;
  targetUrl: string;
  loopbackOnly: boolean;
  probeApplied: boolean;
  probeRemoved: boolean;
  durationMs: number;
};

export type ThemeSummary = {
  id: string;
  name: string;
  description: string;
  version: string;
  author: string;
  preview: {
    background: string;
    surface: string;
    accent: string;
  };
  builtIn: boolean;
  onlineSlug?: string;
  requiresAccount?: boolean;
  locales?: Record<string, {
    name?: string;
    description?: string;
  }>;
};

export type AppLocale = "zh-CN" | "en";

export type ThemePreviewReport = {
  themeId: string;
  theme: ThemeSummary;
  source: "installed" | "localDevelopment" | "wallpaperEngine";
  expiresAt?: number;
  appVersion: string;
  port: number;
  appliedSlots: string[];
  loopbackOnly: boolean;
};

export type WallpaperEngineProject = {
  id: string;
  title: string;
  projectType: string;
  projectPath: string;
  mediaPath?: string;
  mediaSizeBytes?: number;
  requiresWallpaperEngine: boolean;
  supported: boolean;
  reason?: string;
};

export type WallpaperEngineCatalog = {
  root: string;
  projects: WallpaperEngineProject[];
};

export type WallpaperControls = {
  wallpaperBrightness: number;
  interfaceTransparency: number;
};

export type RuntimeEnvironment = {
  appVersion: string;
  themeRuntimeVersion: string;
  compatibility?: {
    adapterId: string;
    revision?: number;
    source: "signedRemote" | "builtIn";
  };
};

export type ThemeInstallReport = {
  theme: ThemeSummary;
  replaced: boolean;
  packageDigest: string;
  signatureVerified: boolean;
};

export type AccountEntitlement = {
  id: number;
  type: string;
  themeId?: number;
  themeSlug?: string;
  source: { type: string; id: string };
  meta?: Record<string, unknown>;
  grantedAt?: string;
};

export type AccountTrial = {
  themeId: number;
  themeSlug?: string;
  startedAt: string;
  expiresAt: string;
  expiresAtTimestamp: number;
  remainingSeconds: number;
};

export type AccountStatus = {
  authenticated: boolean;
  email?: string;
  pro: boolean;
  deviceName: string;
  deviceGeneration?: number;
  heartbeatState: "offline" | "online" | "grace" | "replaced";
  lastHeartbeatAt?: number;
  leaseExpiresAt?: string;
  entitlements: AccountEntitlement[];
  trials: AccountTrial[];
  error?: string;
};

export type AccountDeviceSummary = {
  id: string;
  name: string;
  current: boolean;
  active: boolean;
  registeredAt?: string;
  lastSeenAt?: string;
};

export type AccountThemeSummary = {
  slug: string;
  name: string;
  description?: string;
  locales?: Record<string, {
    name?: string;
    description?: string;
  }>;
  version: string;
  author: { id: string; name: string };
  preview?: {
    background: string;
    surface: string;
    accent: string;
  };
  coverUrl?: string;
  firstUsedAt?: string;
  lastUsedAt?: string;
  useCount?: number;
};

export type AccountSync = {
  devices: AccountDeviceSummary[];
  themes: {
    favorites: AccountThemeSummary[];
    used: AccountThemeSummary[];
  };
};

export type DesktopPreferences = {
  launchAtLogin: boolean;
  hideToTray: boolean;
  autoUpdate: boolean;
  autoDetectCodex: boolean;
  lowMemoryMode: boolean;
};
