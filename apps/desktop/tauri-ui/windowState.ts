import {
  getCurrentWindow,
  monitorFromPoint,
  PhysicalPosition,
  PhysicalSize,
  primaryMonitor,
} from "@tauri-apps/api/window";
import { fitWindowToWorkArea } from "./windowFit";

const WINDOW_STATE_KEY = "lifetrace:desktop:window-state:v1";
const WINDOW_MARGIN = 30;
const MIN_WIDTH = 1050;
const MIN_HEIGHT = 680;

type StoredWindowState = {
  x: number;
  y: number;
  width: number;
  height: number;
  maximized: boolean;
};

function finite(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function readStoredWindowState(): StoredWindowState | null {
  try {
    const raw = window.localStorage.getItem(WINDOW_STATE_KEY);
    if (!raw) return null;
    const value = JSON.parse(raw) as Partial<StoredWindowState>;
    if (!finite(value.x) || !finite(value.y) || !finite(value.width) || !finite(value.height)) return null;
    if (value.width <= 0 || value.height <= 0) return null;
    return {
      x: value.x,
      y: value.y,
      width: value.width,
      height: value.height,
      maximized: value.maximized === true,
    };
  } catch {
    return null;
  }
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(maximum, Math.max(minimum, value));
}

/** Restore the last desktop placement, while keeping the window visible if monitors changed. */
export async function restoreWindowPlacement(): Promise<void> {
  const stored = readStoredWindowState();
  if (!stored) {
    await fitWindowToWorkArea();
    return;
  }

  try {
    const appWindow = getCurrentWindow();
    const monitor = await monitorFromPoint(stored.x + 24, stored.y + 24) ?? await primaryMonitor();
    if (!monitor) {
      await fitWindowToWorkArea();
      return;
    }

    const scale = monitor.scaleFactor || 1;
    const work = monitor.workArea;
    const margin = Math.round(WINDOW_MARGIN * scale);
    const availableWidth = Math.max(1, work.size.width - margin * 2);
    const availableHeight = Math.max(1, work.size.height - margin * 2);
    const width = Math.min(availableWidth, Math.max(Math.round(MIN_WIDTH * scale), stored.width));
    const height = Math.min(availableHeight, Math.max(Math.round(MIN_HEIGHT * scale), stored.height));
    const minX = work.position.x + margin;
    const minY = work.position.y + margin;
    const maxX = work.position.x + work.size.width - margin - width;
    const maxY = work.position.y + work.size.height - margin - height;
    const x = clamp(stored.x, Math.min(minX, maxX), Math.max(minX, maxX));
    const y = clamp(stored.y, Math.min(minY, maxY), Math.max(minY, maxY));

    await appWindow.setSize(new PhysicalSize(width, height));
    await appWindow.setPosition(new PhysicalPosition(x, y));
    if (stored.maximized) await appWindow.maximize();
  } catch (error) {
    console.warn("[LifeTrace] stored window placement could not be restored", error);
    await fitWindowToWorkArea();
  }
}

async function persistNormalPlacement(maximized: boolean): Promise<void> {
  const appWindow = getCurrentWindow();
  const existing = readStoredWindowState();
  if (maximized && existing) {
    window.localStorage.setItem(WINDOW_STATE_KEY, JSON.stringify({ ...existing, maximized: true }));
    return;
  }
  if (maximized) return;

  const [position, size] = await Promise.all([appWindow.outerPosition(), appWindow.outerSize()]);
  const next: StoredWindowState = {
    x: position.x,
    y: position.y,
    width: size.width,
    height: size.height,
    maximized: false,
  };
  window.localStorage.setItem(WINDOW_STATE_KEY, JSON.stringify(next));
}

/** Track user-driven window movement/resize for the lifetime of the desktop app. */
export async function installWindowPlacementPersistence(): Promise<() => void> {
  const appWindow = getCurrentWindow();
  let timer: number | undefined;

  const scheduleSave = () => {
    if (timer !== undefined) window.clearTimeout(timer);
    timer = window.setTimeout(() => {
      void appWindow.isMaximized()
        .then((maximized) => persistNormalPlacement(maximized))
        .catch((error) => console.warn("[LifeTrace] window placement save skipped", error));
    }, 250);
  };

  const [unlistenMove, unlistenResize] = await Promise.all([
    appWindow.onMoved(scheduleSave),
    appWindow.onResized(scheduleSave),
  ]);

  return () => {
    if (timer !== undefined) window.clearTimeout(timer);
    unlistenMove();
    unlistenResize();
  };
}
