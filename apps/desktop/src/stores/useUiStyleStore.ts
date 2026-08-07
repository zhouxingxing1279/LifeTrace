import { create } from "zustand";
import type { LifeTraceTheme } from "@/src/services/themeFiles";
import { buildThemeCss } from "@/src/services/themeFiles";

export type UiStyle = "classic" | "editorial";

interface UiStyleState {
  uiStyle: UiStyle;
  customThemes: LifeTraceTheme[];
  activeThemeId: string | null;
  setUiStyle: (style: UiStyle) => void;
  toggleUiStyle: () => void;
  importTheme: (theme: LifeTraceTheme) => void;
  removeTheme: (id: string) => void;
  enableTheme: (id: string | null) => void;
}

function readJson<T>(key: string, fallback: T): T {
  try {
    const raw = window.localStorage.getItem(key);
    return raw === null ? fallback : (JSON.parse(raw) as T);
  } catch {
    return fallback;
  }
}

const initialStyle: UiStyle =
  typeof window !== "undefined" && window.localStorage.getItem("lifetrace:ui-style") === "editorial"
    ? "editorial"
    : "classic";
const initialThemes: LifeTraceTheme[] =
  typeof window !== "undefined" ? readJson<LifeTraceTheme[]>("lifetrace:custom-themes", []) : [];
const initialActive: string | null =
  typeof window !== "undefined" ? readJson<string | null>("lifetrace:ui-theme", null) : null;

function applySkin(uiStyle: UiStyle, themes: LifeTraceTheme[], activeThemeId: string | null) {
  if (typeof document === "undefined") return;
  document.documentElement.dataset.uiStyle = uiStyle;
  const theme = activeThemeId && uiStyle === "editorial" ? themes.find(item => item.id === activeThemeId) : undefined;
  const styleId = "lifetrace-theme-overrides";
  const existing = document.getElementById(styleId);
  if (theme) {
    const styleEl = existing ?? (() => {
      const element = document.createElement("style");
      element.id = styleId;
      document.head.appendChild(element);
      return element;
    })();
    styleEl.textContent = buildThemeCss(theme);
  } else {
    existing?.remove();
  }
}

if (typeof window !== "undefined") {
  applySkin(initialStyle, initialThemes, initialActive);
}

export const useUiStyleStore = create<UiStyleState>((set, get) => ({
  uiStyle: initialStyle,
  customThemes: initialThemes,
  activeThemeId: initialActive,

  setUiStyle: (uiStyle) => {
    const activeThemeId = uiStyle === "classic" ? null : get().activeThemeId;
    window.localStorage.setItem("lifetrace:ui-style", uiStyle);
    window.localStorage.setItem("lifetrace:ui-theme", JSON.stringify(activeThemeId));
    applySkin(uiStyle, get().customThemes, activeThemeId);
    set({ uiStyle, activeThemeId });
  },

  toggleUiStyle: () => get().setUiStyle(get().uiStyle === "editorial" ? "classic" : "editorial"),

  importTheme: (theme) => {
    const customThemes = [theme, ...get().customThemes];
    window.localStorage.setItem("lifetrace:custom-themes", JSON.stringify(customThemes));
    applySkin(get().uiStyle, customThemes, get().activeThemeId);
    set({ customThemes });
  },

  removeTheme: (id) => {
    const customThemes = get().customThemes.filter(theme => theme.id !== id);
    const activeThemeId = get().activeThemeId === id ? null : get().activeThemeId;
    window.localStorage.setItem("lifetrace:custom-themes", JSON.stringify(customThemes));
    window.localStorage.setItem("lifetrace:ui-theme", JSON.stringify(activeThemeId));
    applySkin(get().uiStyle, customThemes, activeThemeId);
    set({ customThemes, activeThemeId });
  },

  enableTheme: (id) => {
    const uiStyle: UiStyle = id ? "editorial" : get().uiStyle;
    const activeThemeId = id;
    window.localStorage.setItem("lifetrace:ui-style", uiStyle);
    window.localStorage.setItem("lifetrace:ui-theme", JSON.stringify(activeThemeId));
    applySkin(uiStyle, get().customThemes, activeThemeId);
    set({ uiStyle, activeThemeId });
  },
}));
