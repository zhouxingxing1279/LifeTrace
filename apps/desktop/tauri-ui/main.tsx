import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import ClientErrorBoundary from "@/src/components/ClientErrorBoundary";
import DesktopApp from "@/src/components/DesktopApp";
import MobileUploadConnectionStatus from "@/src/components/MobileUploadConnectionStatus";
import { installAppPreferences } from "@/src/services/appPreferences";
import { clientLogger, installGlobalErrorHandlers } from "@/src/services/clientObservability";
import { installDesktopContextMenuPolicy } from "@/src/services/contextMenuPolicy";
import { installGlobalFetchInstrumentation } from "@/src/services/fetchInstrumentation";
import { installTauriApiBridge, waitForTauriBackend } from "./apiBridge";
import { installVaultBridge } from "./vaultBridge";
import { fitWindowToWorkArea } from "./windowFit";

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

installGlobalFetchInstrumentation();
installGlobalErrorHandlers();
installDesktopContextMenuPolicy();

const root = document.getElementById("root");
if (!root) throw new Error("LifeTrace root element is missing");

installAppPreferences();

async function start() {
  void fitWindowToWorkArea();
  installTauriApiBridge();
  installVaultBridge();
  root!.innerHTML = '<div class="hx-loading"><span>LT</span><p>正在启动本地 SQLite 服务…</p></div>';
  clientLogger.info("desktop.start.begin");
  try {
    await waitForTauriBackend();
    clientLogger.info("desktop.backend.ready");
    createRoot(root!).render(
      <StrictMode>
        <ClientErrorBoundary>
          <DesktopApp />
          <MobileUploadConnectionStatus />
        </ClientErrorBoundary>
      </StrictMode>,
    );
  } catch (error) {
    clientLogger.fatal("desktop.start.failed", undefined, error);
    const message = error instanceof Error ? error.message : "本地服务启动失败";
    root!.innerHTML = "";
    const panel = document.createElement("div");
    panel.className = "hx-loading";
    const badge = document.createElement("span");
    badge.textContent = "!";
    const title = document.createElement("h1");
    title.textContent = "LifeTrace 启动失败";
    const detail = document.createElement("p");
    detail.textContent = message;
    const retry = document.createElement("button");
    retry.className = "hx-btn primary";
    retry.textContent = "重新启动";
    retry.addEventListener("click", () => window.location.reload());
    panel.append(badge, title, detail, retry);
    root!.append(panel);
  }
}

void start();