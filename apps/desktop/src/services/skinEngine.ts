export type SkinThemeMode = "light" | "dark";

export type SkinPalette = {
  background: string;
  surface: string;
  foreground: string;
  muted: string;
  border: string;
  primary: string;
  success: string;
  warning: string;
  danger: string;
  info: string;
};

export type SkinVisual = {
  roundness: number;
  shadowStrength: number;
  surfaceOpacity: number;
};

export type SkinBackground = {
  type: "solid" | "gradient";
  from: string;
  to: string;
  angle: number;
  opacity: number;
};

export type SkinDefinition = {
  schemaVersion: 1;
  id: string;
  name: string;
  author: string;
  builtIn: boolean;
  createdAt: string;
  updatedAt: string;
  light: SkinPalette;
  dark: SkinPalette;
  visual: SkinVisual;
  background: SkinBackground;
};

export type SkinDocument = {
  format: "lifetrace-skin";
  formatVersion: 1;
  skin: SkinDefinition;
};

type SkinStorage = Pick<Storage, "getItem" | "setItem" | "removeItem">;

export const CUSTOM_SKINS_STORAGE_KEY = "lifetrace.custom-skins.v1";
export const ACTIVE_SKIN_STORAGE_KEY = "lifetrace.active-skin.v1";
export const ACTIVE_SKIN_STYLE_ID = "lifetrace-active-skin";
export const DEFAULT_SKIN_ID = "lifetrace";

const ISO_BUILTIN_DATE = "2026-08-08T00:00:00.000Z";
const HEX_COLOR = /^#[0-9a-fA-F]{6}$/;

const DEFAULT_LIGHT: SkinPalette = {
  background: "#F4F5F4",
  surface: "#FFFFFF",
  foreground: "#1B201D",
  muted: "#5F6862",
  border: "#E0E3E0",
  primary: "#1F6F56",
  success: "#1E7A4C",
  warning: "#966A1C",
  danger: "#B0413F",
  info: "#2F6FB3",
};

const DEFAULT_DARK: SkinPalette = {
  background: "#171A18",
  surface: "#1E221F",
  foreground: "#E6E9E6",
  muted: "#A2AAA4",
  border: "#2C312D",
  primary: "#5CB491",
  success: "#4FBF86",
  warning: "#D3A34E",
  danger: "#E07B76",
  info: "#6FA8E8",
};

const DEFAULT_VISUAL: SkinVisual = {
  roundness: 42,
  shadowStrength: 34,
  surfaceOpacity: 96,
};

const DEFAULT_BACKGROUND: SkinBackground = {
  type: "solid",
  from: "#F4F5F4",
  to: "#E8EEE9",
  angle: 135,
  opacity: 18,
};

function builtin(
  id: string,
  name: string,
  light: SkinPalette,
  dark: SkinPalette,
  visual: Partial<SkinVisual> = {},
  background: Partial<SkinBackground> = {},
): SkinDefinition {
  return {
    schemaVersion: 1,
    id,
    name,
    author: "LifeTrace",
    builtIn: true,
    createdAt: ISO_BUILTIN_DATE,
    updatedAt: ISO_BUILTIN_DATE,
    light,
    dark,
    visual: { ...DEFAULT_VISUAL, ...visual },
    background: { ...DEFAULT_BACKGROUND, ...background },
  };
}

export const BUILTIN_SKINS: readonly SkinDefinition[] = [
  builtin(DEFAULT_SKIN_ID, "LifeTrace", DEFAULT_LIGHT, DEFAULT_DARK),
  builtin(
    "forest",
    "Forest",
    {
      background: "#F3F5EF",
      surface: "#FBFCF8",
      foreground: "#20261F",
      muted: "#687166",
      border: "#DCE2D7",
      primary: "#397A55",
      success: "#377A50",
      warning: "#987128",
      danger: "#AF4A45",
      info: "#3C6D8C",
    },
    {
      background: "#121914",
      surface: "#19221C",
      foreground: "#E8EFE9",
      muted: "#9EACA1",
      border: "#2A372D",
      primary: "#68B984",
      success: "#60B17C",
      warning: "#D3A555",
      danger: "#DD7B73",
      info: "#74A6C5",
    },
    { roundness: 52, shadowStrength: 24 },
    { type: "gradient", from: "#DDE8D8", to: "#EEF2E6", opacity: 26 },
  ),
  builtin(
    "midnight",
    "Midnight",
    {
      background: "#F2F4F8",
      surface: "#FFFFFF",
      foreground: "#202331",
      muted: "#687086",
      border: "#DEE2EA",
      primary: "#5367C8",
      success: "#2F7B68",
      warning: "#9A6D25",
      danger: "#B54A58",
      info: "#426FB5",
    },
    {
      background: "#0F1320",
      surface: "#171C2B",
      foreground: "#EBEEF8",
      muted: "#9BA4BE",
      border: "#2A3247",
      primary: "#8294F0",
      success: "#62B79D",
      warning: "#D9AD61",
      danger: "#E27E8C",
      info: "#78A7EA",
    },
    { roundness: 48, shadowStrength: 52, surfaceOpacity: 93 },
    { type: "gradient", from: "#151A2D", to: "#27214A", angle: 145, opacity: 44 },
  ),
  builtin(
    "paper",
    "Paper",
    {
      background: "#F4F0E8",
      surface: "#FCFAF5",
      foreground: "#2B2924",
      muted: "#716D64",
      border: "#DED8CC",
      primary: "#7B6040",
      success: "#507455",
      warning: "#9A702E",
      danger: "#A84E47",
      info: "#536F8A",
    },
    {
      background: "#1A1815",
      surface: "#24211D",
      foreground: "#EEE9DF",
      muted: "#AAA298",
      border: "#38332C",
      primary: "#C39A68",
      success: "#78AA7D",
      warning: "#D4A85B",
      danger: "#DA7A70",
      info: "#84A5C1",
    },
    { roundness: 24, shadowStrength: 14, surfaceOpacity: 98 },
    { type: "solid", from: "#F4F0E8", to: "#F4F0E8", opacity: 0 },
  ),
];

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function storageOrUndefined(storage?: SkinStorage): SkinStorage | undefined {
  if (storage) return storage;
  return typeof window !== "undefined" ? window.localStorage : undefined;
}

function clamp(value: unknown, min: number, max: number, fallback: number): number {
  const parsed = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(parsed)) return fallback;
  return Math.min(max, Math.max(min, parsed));
}

export function normalizeHex(value: unknown, fallback: string): string {
  return typeof value === "string" && HEX_COLOR.test(value) ? value.toUpperCase() : fallback;
}

function normalizePalette(value: unknown, fallback: SkinPalette): SkinPalette {
  const candidate = value && typeof value === "object" ? value as Partial<SkinPalette> : {};
  return {
    background: normalizeHex(candidate.background, fallback.background),
    surface: normalizeHex(candidate.surface, fallback.surface),
    foreground: normalizeHex(candidate.foreground, fallback.foreground),
    muted: normalizeHex(candidate.muted, fallback.muted),
    border: normalizeHex(candidate.border, fallback.border),
    primary: normalizeHex(candidate.primary, fallback.primary),
    success: normalizeHex(candidate.success, fallback.success),
    warning: normalizeHex(candidate.warning, fallback.warning),
    danger: normalizeHex(candidate.danger, fallback.danger),
    info: normalizeHex(candidate.info, fallback.info),
  };
}

function normalizeVisual(value: unknown): SkinVisual {
  const candidate = value && typeof value === "object" ? value as Partial<SkinVisual> : {};
  return {
    roundness: clamp(candidate.roundness, 0, 100, DEFAULT_VISUAL.roundness),
    shadowStrength: clamp(candidate.shadowStrength, 0, 100, DEFAULT_VISUAL.shadowStrength),
    surfaceOpacity: clamp(candidate.surfaceOpacity, 72, 100, DEFAULT_VISUAL.surfaceOpacity),
  };
}

function normalizeBackground(value: unknown): SkinBackground {
  const candidate = value && typeof value === "object" ? value as Partial<SkinBackground> : {};
  return {
    type: candidate.type === "gradient" ? "gradient" : "solid",
    from: normalizeHex(candidate.from, DEFAULT_BACKGROUND.from),
    to: normalizeHex(candidate.to, DEFAULT_BACKGROUND.to),
    angle: clamp(candidate.angle, 0, 360, DEFAULT_BACKGROUND.angle),
    opacity: clamp(candidate.opacity, 0, 70, DEFAULT_BACKGROUND.opacity),
  };
}

function normalizeId(value: unknown): string {
  const raw = typeof value === "string" ? value : "skin";
  const safe = raw.toLowerCase().replace(/[^a-z0-9_-]+/g, "-").replace(/^-+|-+$/g, "");
  return safe || "skin";
}

export function normalizeSkin(value: unknown, fallback?: SkinDefinition): SkinDefinition {
  const base = fallback ?? BUILTIN_SKINS[0];
  const candidate = value && typeof value === "object" ? value as Partial<SkinDefinition> : {};
  const createdAt = typeof candidate.createdAt === "string" ? candidate.createdAt : new Date().toISOString();
  const updatedAt = typeof candidate.updatedAt === "string" ? candidate.updatedAt : createdAt;
  return {
    schemaVersion: 1,
    id: normalizeId(candidate.id ?? base.id),
    name: typeof candidate.name === "string" && candidate.name.trim() ? candidate.name.trim().slice(0, 48) : base.name,
    author: typeof candidate.author === "string" && candidate.author.trim() ? candidate.author.trim().slice(0, 48) : "User",
    builtIn: candidate.builtIn === true,
    createdAt,
    updatedAt,
    light: normalizePalette(candidate.light, base.light),
    dark: normalizePalette(candidate.dark, base.dark),
    visual: normalizeVisual(candidate.visual),
    background: normalizeBackground(candidate.background),
  };
}

export function readCustomSkins(storage?: SkinStorage): SkinDefinition[] {
  const source = storageOrUndefined(storage);
  if (!source) return [];
  try {
    const raw = source.getItem(CUSTOM_SKINS_STORAGE_KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter((item) => item && typeof item === "object")
      .map((item) => ({ ...normalizeSkin(item), builtIn: false }));
  } catch {
    return [];
  }
}

export function writeCustomSkins(skins: SkinDefinition[], storage?: SkinStorage): void {
  const target = storageOrUndefined(storage);
  if (!target) return;
  const normalized = skins.map((skin) => ({ ...normalizeSkin(skin), builtIn: false }));
  target.setItem(CUSTOM_SKINS_STORAGE_KEY, JSON.stringify(normalized));
}

export function getSkinLibrary(storage?: SkinStorage): SkinDefinition[] {
  return [...BUILTIN_SKINS.map(clone), ...readCustomSkins(storage)];
}

export function readActiveSkinId(storage?: SkinStorage): string {
  const source = storageOrUndefined(storage);
  const id = source?.getItem(ACTIVE_SKIN_STORAGE_KEY);
  return id ? normalizeId(id) : DEFAULT_SKIN_ID;
}

export function writeActiveSkinId(id: string, storage?: SkinStorage): void {
  storageOrUndefined(storage)?.setItem(ACTIVE_SKIN_STORAGE_KEY, normalizeId(id));
}

export function resolveSkin(id: string, storage?: SkinStorage): SkinDefinition {
  const normalizedId = normalizeId(id);
  return getSkinLibrary(storage).find((skin) => skin.id === normalizedId) ?? clone(BUILTIN_SKINS[0]);
}

export function saveCustomSkin(skin: SkinDefinition, storage?: SkinStorage): SkinDefinition {
  const normalized = { ...normalizeSkin(skin), builtIn: false, updatedAt: new Date().toISOString() };
  const skins = readCustomSkins(storage);
  const index = skins.findIndex((item) => item.id === normalized.id);
  if (index >= 0) skins[index] = normalized;
  else skins.push(normalized);
  writeCustomSkins(skins, storage);
  return normalized;
}

export function deleteCustomSkin(id: string, storage?: SkinStorage): void {
  const normalizedId = normalizeId(id);
  writeCustomSkins(readCustomSkins(storage).filter((skin) => skin.id !== normalizedId), storage);
  if (readActiveSkinId(storage) === normalizedId) writeActiveSkinId(DEFAULT_SKIN_ID, storage);
}

export function createSkinId(name: string, existing: Iterable<string> = []): string {
  const used = new Set(existing);
  const base = normalizeId(name) || "custom-skin";
  if (!used.has(base) && !BUILTIN_SKINS.some((skin) => skin.id === base)) return base;
  let counter = 2;
  while (used.has(`${base}-${counter}`) || BUILTIN_SKINS.some((skin) => skin.id === `${base}-${counter}`)) counter += 1;
  return `${base}-${counter}`;
}

export function cloneSkin(source: SkinDefinition, name = `${source.name} Copy`, storage?: SkinStorage): SkinDefinition {
  const now = new Date().toISOString();
  const existing = getSkinLibrary(storage).map((skin) => skin.id);
  return {
    ...clone(source),
    id: createSkinId(name, existing),
    name,
    author: "User",
    builtIn: false,
    createdAt: now,
    updatedAt: now,
  };
}

function rgb(hex: string): [number, number, number] {
  const value = normalizeHex(hex, "#000000").slice(1);
  return [Number.parseInt(value.slice(0, 2), 16), Number.parseInt(value.slice(2, 4), 16), Number.parseInt(value.slice(4, 6), 16)];
}

function toHex(value: number): string {
  return Math.round(Math.min(255, Math.max(0, value))).toString(16).padStart(2, "0").toUpperCase();
}

export function mixHex(a: string, b: string, weight: number): string {
  const [ar, ag, ab] = rgb(a);
  const [br, bg, bb] = rgb(b);
  const w = Math.min(1, Math.max(0, weight));
  return `#${toHex(ar + (br - ar) * w)}${toHex(ag + (bg - ag) * w)}${toHex(ab + (bb - ab) * w)}`;
}

function rgba(hex: string, alpha: number): string {
  const [r, g, b] = rgb(hex);
  return `rgba(${r}, ${g}, ${b}, ${Math.min(1, Math.max(0, alpha)).toFixed(3)})`;
}

function luminance(hex: string): number {
  const values = rgb(hex).map((channel) => {
    const normalized = channel / 255;
    return normalized <= 0.03928 ? normalized / 12.92 : ((normalized + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * values[0] + 0.7152 * values[1] + 0.0722 * values[2];
}

export function contrastRatio(a: string, b: string): number {
  const first = luminance(a);
  const second = luminance(b);
  const light = Math.max(first, second);
  const dark = Math.min(first, second);
  return (light + 0.05) / (dark + 0.05);
}

function contrastFor(color: string): string {
  return contrastRatio(color, "#FFFFFF") >= contrastRatio(color, "#101412") ? "#FFFFFF" : "#101412";
}

function paletteCss(palette: SkinPalette, theme: SkinThemeMode, visual: SkinVisual, gradient: boolean): string {
  const dark = theme === "dark";
  const surfaceOpacity = gradient ? visual.surfaceOpacity / 100 : 1;
  const appOpacity = gradient ? Math.min(0.98, Math.max(0.78, surfaceOpacity - 0.035)) : 1;
  const subtle = mixHex(palette.surface, palette.foreground, dark ? 0.07 : 0.055);
  const hover = mixHex(palette.surface, palette.foreground, dark ? 0.10 : 0.08);
  const active = mixHex(palette.surface, palette.foreground, dark ? 0.14 : 0.12);
  const borderStrong = mixHex(palette.border, palette.foreground, dark ? 0.18 : 0.13);
  const faint = mixHex(palette.muted, palette.surface, 0.35);
  const primaryHover = mixHex(palette.primary, dark ? "#FFFFFF" : "#000000", 0.11);
  const primaryActive = mixHex(palette.primary, dark ? "#000000" : "#000000", dark ? 0.08 : 0.20);
  const primarySoft = mixHex(palette.surface, palette.primary, dark ? 0.20 : 0.13);
  const sidebar = mixHex(palette.surface, palette.background, dark ? 0.30 : 0.42);
  const uiSurface = surfaceOpacity < 1 ? rgba(palette.surface, surfaceOpacity) : palette.surface;
  const uiBackground = appOpacity < 1 ? rgba(palette.background, appOpacity) : palette.background;

  return [
    `--ui-bg-app:${uiBackground}`,
    `--ui-bg-surface:${uiSurface}`,
    `--ui-bg-subtle:${subtle}`,
    `--ui-bg-hover:${hover}`,
    `--ui-bg-active:${active}`,
    `--ui-bg-inset:${mixHex(palette.surface, palette.background, 0.45)}`,
    `--ui-border:${palette.border}`,
    `--ui-border-strong:${borderStrong}`,
    `--ui-foreground:${palette.foreground}`,
    `--ui-muted:${palette.muted}`,
    `--ui-faint:${faint}`,
    `--ui-primary:${palette.primary}`,
    `--ui-primary-hover:${primaryHover}`,
    `--ui-primary-active:${primaryActive}`,
    `--ui-primary-soft:${primarySoft}`,
    `--ui-primary-contrast:${contrastFor(palette.primary)}`,
    `--ui-success:${palette.success}`,
    `--ui-success-soft:${mixHex(palette.surface, palette.success, dark ? 0.20 : 0.12)}`,
    `--ui-warning:${palette.warning}`,
    `--ui-warning-soft:${mixHex(palette.surface, palette.warning, dark ? 0.20 : 0.12)}`,
    `--ui-danger:${palette.danger}`,
    `--ui-danger-soft:${mixHex(palette.surface, palette.danger, dark ? 0.20 : 0.12)}`,
    `--ui-info:${palette.info}`,
    `--ui-info-soft:${mixHex(palette.surface, palette.info, dark ? 0.20 : 0.12)}`,
    `--ui-focus:${palette.primary}`,
    `--ui-focus-ring:${rgba(palette.primary, dark ? 0.26 : 0.18)}`,
    `--ui-sidebar-bg:${surfaceOpacity < 1 ? rgba(sidebar, Math.min(1, surfaceOpacity + 0.015)) : sidebar}`,
    `--ui-sidebar-border:${palette.border}`,
    `--ui-sidebar-fg:${palette.foreground}`,
    `--ui-sidebar-muted:${palette.muted}`,
    `--ui-sidebar-hover:${hover}`,
    `--ui-sidebar-active-bg:${primarySoft}`,
    `--ui-sidebar-active-fg:${palette.primary}`,
    `--ui-topbar-bg:${uiBackground}`,
    `--ui-topbar-border:${palette.border}`,
    `--skin-app-background:${uiBackground}`,
    `--hx-deep:${mixHex(palette.background, palette.primary, dark ? 0.12 : 0.18)}`,
    `--hx-deep2:${mixHex(palette.surface, palette.primary, dark ? 0.18 : 0.24)}`,
    `--hx-lime:${mixHex(palette.primary, palette.surface, dark ? 0.35 : 0.48)}`,
  ].join(";");
}

function visualCss(visual: SkinVisual, background: SkinBackground): string {
  const rounded = visual.roundness / 100;
  const shadow = visual.shadowStrength / 100;
  const radiusSm = 4 + rounded * 5;
  const radiusMd = 6 + rounded * 6;
  const radiusLg = 8 + rounded * 8;
  const radiusXl = 10 + rounded * 10;
  const wallpaper = background.type === "gradient"
    ? `linear-gradient(${background.angle}deg, ${rgba(background.from, background.opacity / 100)}, ${rgba(background.to, background.opacity / 100)})`
    : "none";
  return [
    `--ui-radius-sm:${radiusSm.toFixed(1)}px`,
    `--ui-radius-md:${radiusMd.toFixed(1)}px`,
    `--ui-radius-lg:${radiusLg.toFixed(1)}px`,
    `--ui-radius-xl:${radiusXl.toFixed(1)}px`,
    `--ui-shadow-sm:0 1px 2px rgba(12,18,15,${(0.02 + shadow * 0.07).toFixed(3)})`,
    `--ui-shadow-md:0 4px 14px rgba(12,18,15,${(0.035 + shadow * 0.10).toFixed(3)})`,
    `--ui-shadow-lg:0 14px 34px rgba(12,18,15,${(0.06 + shadow * 0.15).toFixed(3)})`,
    `--ui-shadow-menu:0 10px 28px rgba(8,14,11,${(0.07 + shadow * 0.17).toFixed(3)})`,
    `--ui-shadow-dialog:0 22px 60px rgba(8,14,11,${(0.10 + shadow * 0.22).toFixed(3)})`,
    `--ui-shadow-popover:0 6px 18px rgba(8,14,11,${(0.05 + shadow * 0.14).toFixed(3)})`,
    `--skin-wallpaper-image:${wallpaper}`,
  ].join(";");
}

export function skinToCss(skinValue: SkinDefinition): string {
  const skin = normalizeSkin(skinValue, BUILTIN_SKINS[0]);
  const gradient = skin.background.type === "gradient";
  return `:root{${visualCss(skin.visual, skin.background)}}\n:root[data-theme="light"]{${paletteCss(skin.light, "light", skin.visual, gradient)}}\n:root[data-theme="dark"]{${paletteCss(skin.dark, "dark", skin.visual, gradient)}}`;
}

export function applySkin(skinValue: SkinDefinition, doc?: Document): SkinDefinition {
  const skin = normalizeSkin(skinValue, BUILTIN_SKINS[0]);
  const target = doc ?? (typeof document !== "undefined" ? document : undefined);
  if (!target) return skin;
  let style = target.getElementById(ACTIVE_SKIN_STYLE_ID) as HTMLStyleElement | null;
  if (!style) {
    style = target.createElement("style");
    style.id = ACTIVE_SKIN_STYLE_ID;
    target.head.append(style);
  }
  style.textContent = skinToCss(skin);
  target.documentElement.dataset.skinId = skin.id;
  target.documentElement.dataset.skinBackground = skin.background.type;
  return skin;
}

export function applyActiveSkin(storage?: SkinStorage, doc?: Document): SkinDefinition {
  return applySkin(resolveSkin(readActiveSkinId(storage), storage), doc);
}

export function installSkinEngine(storage?: SkinStorage, doc?: Document): SkinDefinition {
  const target = storageOrUndefined(storage);
  const id = readActiveSkinId(target);
  const resolved = resolveSkin(id, target);
  if (resolved.id !== id) writeActiveSkinId(resolved.id, target);
  return applySkin(resolved, doc);
}

export function exportSkinDocument(skin: SkinDefinition): string {
  const normalized = { ...normalizeSkin(skin), builtIn: false };
  const documentValue: SkinDocument = {
    format: "lifetrace-skin",
    formatVersion: 1,
    skin: normalized,
  };
  return JSON.stringify(documentValue, null, 2);
}

export function importSkinDocument(text: string, storage?: SkinStorage): SkinDefinition {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new Error("皮肤文件不是有效的 JSON");
  }
  if (!parsed || typeof parsed !== "object") throw new Error("皮肤文件格式无效");
  const doc = parsed as Partial<SkinDocument>;
  if (doc.format !== "lifetrace-skin" || doc.formatVersion !== 1 || !doc.skin) {
    throw new Error("不是受支持的 LifeTrace 皮肤文件");
  }
  const imported = { ...normalizeSkin(doc.skin), builtIn: false };
  const existing = getSkinLibrary(storage).map((skin) => skin.id);
  if (existing.includes(imported.id)) imported.id = createSkinId(imported.name, existing);
  imported.createdAt = new Date().toISOString();
  imported.updatedAt = imported.createdAt;
  return imported;
}
