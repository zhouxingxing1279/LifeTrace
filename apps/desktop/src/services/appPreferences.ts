export type ThemePreference = "system" | "light" | "dark";
export type DensityPreference = "comfortable" | "compact";
export type FontScalePreference = "small" | "normal" | "large";

export type AppPreferences = {
  theme: ThemePreference;
  density: DensityPreference;
  fontScale: FontScalePreference;
  reduceMotion: boolean;
};

type StorageReader = Pick<Storage, "getItem">;
type StorageWriter = Pick<Storage, "setItem">;

export const APP_PREFERENCES_STORAGE_KEY = "lifetrace.app-preferences.v1";

export const DEFAULT_APP_PREFERENCES: AppPreferences = {
  theme: "system",
  density: "comfortable",
  fontScale: "normal",
  reduceMotion: false,
};

const isTheme = (value: unknown): value is ThemePreference =>
  value === "system" || value === "light" || value === "dark";
const isDensity = (value: unknown): value is DensityPreference =>
  value === "comfortable" || value === "compact";
const isFontScale = (value: unknown): value is FontScalePreference =>
  value === "small" || value === "normal" || value === "large";

export function normalizeAppPreferences(value: unknown): AppPreferences {
  if (!value || typeof value !== "object") return { ...DEFAULT_APP_PREFERENCES };
  const candidate = value as Partial<AppPreferences>;
  return {
    theme: isTheme(candidate.theme) ? candidate.theme : DEFAULT_APP_PREFERENCES.theme,
    density: isDensity(candidate.density) ? candidate.density : DEFAULT_APP_PREFERENCES.density,
    fontScale: isFontScale(candidate.fontScale) ? candidate.fontScale : DEFAULT_APP_PREFERENCES.fontScale,
    reduceMotion: typeof candidate.reduceMotion === "boolean"
      ? candidate.reduceMotion
      : DEFAULT_APP_PREFERENCES.reduceMotion,
  };
}

export function readAppPreferences(storage?: StorageReader): AppPreferences {
  const source = storage ?? (typeof window !== "undefined" ? window.localStorage : undefined);
  if (!source) return { ...DEFAULT_APP_PREFERENCES };
  try {
    const raw = source.getItem(APP_PREFERENCES_STORAGE_KEY);
    return raw ? normalizeAppPreferences(JSON.parse(raw)) : { ...DEFAULT_APP_PREFERENCES };
  } catch {
    return { ...DEFAULT_APP_PREFERENCES };
  }
}

export function writeAppPreferences(preferences: AppPreferences, storage?: StorageWriter): void {
  const target = storage ?? (typeof window !== "undefined" ? window.localStorage : undefined);
  if (!target) return;
  target.setItem(APP_PREFERENCES_STORAGE_KEY, JSON.stringify(normalizeAppPreferences(preferences)));
}

function resolveTheme(theme: ThemePreference): "light" | "dark" {
  if (theme !== "system") return theme;
  return typeof window !== "undefined" && window.matchMedia?.("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

export function applyAppPreferences(
  preferences: AppPreferences,
  root?: HTMLElement,
): void {
  const target = root ?? (typeof document !== "undefined" ? document.documentElement : undefined);
  if (!target) return;
  const normalized = normalizeAppPreferences(preferences);
  const resolvedTheme = resolveTheme(normalized.theme);
  target.dataset.themePreference = normalized.theme;
  target.dataset.theme = resolvedTheme;
  target.dataset.density = normalized.density;
  target.dataset.fontScale = normalized.fontScale;
  target.dataset.reduceMotion = String(normalized.reduceMotion);
  target.style.colorScheme = resolvedTheme;
  target.style.fontSize = normalized.fontScale === "small" ? "14px" : normalized.fontScale === "large" ? "17px" : "16px";
}

const PREFERENCE_STYLE_ID = "lifetrace-app-preference-styles";
let mediaListenerInstalled = false;

export function installAppPreferences(): AppPreferences {
  const preferences = readAppPreferences();
  applyAppPreferences(preferences);
  if (typeof document !== "undefined" && !document.getElementById(PREFERENCE_STYLE_ID)) {
    const style = document.createElement("style");
    style.id = PREFERENCE_STYLE_ID;
    style.textContent = `
      :root[data-theme="dark"] {
        --hx-bg:#101512;--hx-paper:#151b18;--hx-panel:#19211d;--hx-soft:#202a25;
        --hx-ink:#ecf3ef;--hx-muted:#9aaba2;--hx-line:#2c3832;--hx-deep:#0b100e;
        --hx-deep2:#20372f;--hx-accent:#59b88e;--hx-accent2:#7bcaa5;
        --hx-accent-soft:#203b30;--hx-lime:#b9d58d;--hx-danger:#e07972;
        --hx-shadow:0 16px 55px rgba(0,0,0,.28);
      }
      :root[data-theme="dark"] body,
      :root[data-theme="dark"] .hx-shell { background:var(--hx-bg); color:var(--hx-ink); }
      :root[data-theme="dark"] .hx-panel,
      :root[data-theme="dark"] .hx-metric,
      :root[data-theme="dark"] .hx-habit-card,
      :root[data-theme="dark"] .hx-account-card,
      :root[data-theme="dark"] .hx-btn.secondary,
      :root[data-theme="dark"] input,
      :root[data-theme="dark"] select,
      :root[data-theme="dark"] textarea { background:var(--hx-panel); color:var(--hx-ink); border-color:var(--hx-line); }
      :root[data-theme="dark"] .hx-segmented,
      :root[data-theme="dark"] .hx-row-icon,
      :root[data-theme="dark"] .hx-account-mini > div { background:var(--hx-soft); }
      :root[data-density="compact"] .hx-main { padding-bottom:32px; }
      :root[data-density="compact"] .hx-panel-head { min-height:56px; padding:12px 16px; }
      :root[data-density="compact"] .hx-panel-body { padding:15px 16px; }
      :root[data-density="compact"] .hx-row { padding:8px 0; }
      :root[data-density="compact"] .hx-toolbar { margin:10px 0; }
      :root[data-reduce-motion="true"] *,
      :root[data-reduce-motion="true"] *::before,
      :root[data-reduce-motion="true"] *::after {
        animation-duration:.01ms!important;animation-iteration-count:1!important;
        scroll-behavior:auto!important;transition-duration:.01ms!important;
      }
    `;
    document.head.append(style);
  }
  if (typeof window !== "undefined" && !mediaListenerInstalled && window.matchMedia) {
    mediaListenerInstalled = true;
    window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
      const current = readAppPreferences();
      if (current.theme === "system") applyAppPreferences(current);
    });
  }
  return preferences;
}
