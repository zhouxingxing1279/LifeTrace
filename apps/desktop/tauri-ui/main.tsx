import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import ClientErrorBoundary from "@/src/components/ClientErrorBoundary";
import DesktopApp from "@/src/components/DesktopApp";
import MobileUploadConnectionStatus from "@/src/components/MobileUploadConnectionStatus";
import { installAppPreferences, setAppThemePreference } from "@/src/services/appPreferences";
import { clientLogger, installGlobalErrorHandlers } from "@/src/services/clientObservability";
import { installDesktopContextMenuPolicy } from "@/src/services/contextMenuPolicy";
import { installGlobalFetchInstrumentation } from "@/src/services/fetchInstrumentation";
import { useLifeStore } from "@/src/stores/useLifeStore";
import { installTauriApiBridge } from "./apiBridge";
import { waitForTauriBackend } from "./backendStartup";
import { installVaultBridge } from "./vaultBridge";
import { installWindowPlacementPersistence, restoreWindowPlacement } from "./windowState";

/* The authenticated cloud workspace reuses the current apps/web feature layer.
 * Compile its Tailwind visual contract first, then keep desktop/local styles in
 * control of native shell and local-only tools. */
import "../../web/src/styles/globals.css";

import "@/app/tokens.css";
import "@/app/globals.css";
import "@/app/hengxu.css";
import "@/app/fitness-app.css";
import "@/app/english.css";
import "@/app/english-desktop-pilot.css";
import "@/app/xunji-import.css";
import "@/app/notes.css";
import "@/app/persist-project.css";
import "@/app/photo-sync.css";
import "@/app/local-vault.css";
import "@/app/settings.css";
import "@/app/account-settings-redesign.css";
import "@/app/ui-menus.css";
import "@/app/auth-shell-fixes.css";
import "@/app/desktop-workspace.css";
import "@/app/execution.css";
import "@/app/execution-calendar.css";
import "@/app/analytics.css";
import "@/app/record-workspace.css";
import "@/app/module-layout-overrides.css";
import "@/app/apple-polish.css";
import "@/app/interaction-performance.css";
import "@/app/desktop-cloud-workspace.css";
import "@/app/desktop-local-tools.css";

declare global {
  interface Window {
    __LIFETRACE_MODULE_STARTED__?: boolean;
  }
}

window.__LIFETRACE_MODULE_STARTED__ = true;

const rootCandidate = document.getElementById("root");
if (!rootCandidate) throw new Error("LifeTrace root element is missing");
const root: HTMLElement = rootCandidate;

function renderStartupStatus(message: string) {
  root.dataset.lifetraceBootStage = message;
  root.innerHTML = '<div class="hx-loading"><span>LT</span><p></p></div>';
  const detail = root.querySelector("p");
  if (detail) detail.textContent = message;
}

function renderStartupFailure(error: unknown) {
  const message = error instanceof Error ? error.message : String(error || "桌面端初始化失败");
  root.dataset.lifetraceBootPending = "false";
  root.dataset.lifetraceBootStage = "failed";
  root.innerHTML = "";
  const panel = document.createElement("div");
  panel.className = "hx-loading";
  panel.setAttribute("role", "alert");
  const badge = document.createElement("span");
  badge.textContent = "!";
  const title = document.createElement("h1");
  title.textContent = "LifeTrace 启动失败";
  const detail = document.createElement("p");
  detail.textContent = message;
  const hint = document.createElement("small");
  hint.textContent = "请保留这段错误信息。若提示 WebView2 请更新 Runtime；若提示数据库迁移或端口占用，可直接据此修复。";
  const retry = document.createElement("button");
  retry.className = "hx-btn primary";
  retry.textContent = "重新启动";
  retry.addEventListener("click", () => window.location.reload());
  panel.append(badge, title, detail, hint, retry);
  root.append(panel);
}

function installThemeStoreSync() {
  useLifeStore.subscribe((state, previous) => {
    if (!state.ready) return;
    if (!previous.ready) {
      const renderedDark = document.documentElement.dataset.theme === "dark";
      if (state.dark && !renderedDark) {
        setAppThemePreference("dark");
        return;
      }
      if (!state.dark && renderedDark) {
        useLifeStore.setState({ dark: true });
        return;
      }
    }
    if (state.dark !== previous.dark) {
      setAppThemePreference(state.dark ? "dark" : "light");
    }
  });
}

async function start() {
  root.dataset.lifetraceBootPending = "true";
  renderStartupStatus("正在初始化桌面环境…");

  try {
    installGlobalErrorHandlers();
    installGlobalFetchInstrumentation();
    installDesktopContextMenuPolicy();
    installAppPreferences();
    installThemeStoreSync();

    renderStartupStatus("正在恢复窗口状态…");
    await restoreWindowPlacement();
    void installWindowPlacementPersistence().catch((error) => {
      clientLogger.warn("desktop.window_persistence_unavailable", undefined, error);
    });

    installTauriApiBridge();
    installVaultBridge();
    renderStartupStatus("正在启动本地 SQLite 服务… 0s");
    clientLogger.info("desktop.start.begin");

    await waitForTauriBackend(45_000, renderStartupStatus);
    clientLogger.info("desktop.backend.ready");
    root.dataset.lifetraceBootStage = "rendering-app";
    createRoot(root).render(
      <StrictMode>
        <ClientErrorBoundary>
          <DesktopApp />
          <MobileUploadConnectionStatus />
        </ClientErrorBoundary>
      </StrictMode>,
    );
    root.dataset.lifetraceBootPending = "false";
    root.dataset.lifetraceBootStage = "ready";
  } catch (error) {
    try {
      clientLogger.fatal("desktop.start.failed", undefined, error);
    } catch {
      // The startup error page must not depend on logging being available.
    }
    renderStartupFailure(error);
  }
}

void start();
