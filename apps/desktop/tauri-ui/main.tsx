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
import { installUiPreviewMailMocks } from "./uiPreviewMailMocks";
import { installUiPreviewMocks } from "./uiPreviewMocks";
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
import "./uiPreviewFrame.css";

const uiPreview = import.meta.env.VITE_UI_PREVIEW === "1";
const uiPreviewFrame = new URLSearchParams(window.location.search).get("__appFrame") === "1";

const root = document.getElementById("root");
if (!root) throw new Error("LifeTrace root element is missing");

function renderUiPreviewHost() {
  document.documentElement.dataset.uiPreviewHost = "true";

  const appUrl = new URL(window.location.href);
  appUrl.searchParams.set("__appFrame", "1");

  const frame = document.createElement("iframe");
  frame.className = "lt-ui-preview-window";
  frame.title = "LifeTrace Windows App UI Preview — 1460 × 850";
  frame.src = appUrl.toString();
  frame.setAttribute("aria-label", "LifeTrace Windows App UI Preview");

  root.replaceChildren(frame);
}

async function start() {
  if (uiPreview) {
    root!.innerHTML = '<div class="hx-loading"><span>LT</span><p>正在加载 UI Preview…</p></div>';
    clientLogger.info("ui_preview.start.begin");
  } else {
    void fitWindowToWorkArea();
    installTauriApiBridge();
    installVaultBridge();
    root!.innerHTML = '<div class="hx-loading"><span>LT</span><p>正在启动本地 SQLite 服务…</p></div>';
    clientLogger.info("desktop.start.begin");
  }

  try {
    if (!uiPreview) {
      await waitForTauriBackend();
      clientLogger.info("desktop.backend.ready");
    }

    createRoot(root!).render(
      <StrictMode>
        <ClientErrorBoundary>
          <DesktopApp />
          <MobileUploadConnectionStatus />
        </ClientErrorBoundary>
      </StrictMode>,
    );

    if (uiPreview) clientLogger.info("ui_preview.start.ready");
  } catch (error) {
    clientLogger.fatal(uiPreview ? "ui_preview.start.failed" : "desktop.start.failed", undefined, error);
    const message = error instanceof Error ? error.message : uiPreview ? "UI Preview 启动失败" : "本地服务启动失败";
    root!.innerHTML = "";
    const panel = document.createElement("div");
    panel.className = "hx-loading";
    const badge = document.createElement("span");
    badge.textContent = "!";
    const title = document.createElement("h1");
    title.textContent = uiPreview ? "LifeTrace UI Preview 启动失败" : "LifeTrace 启动失败";
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

if (uiPreview && !uiPreviewFrame) {
  renderUiPreviewHost();
} else {
  if (uiPreview) {
    installUiPreviewMocks();
    installUiPreviewMailMocks();
  } else {
    installDesktopContextMenuPolicy();
  }

  installGlobalFetchInstrumentation();
  installGlobalErrorHandlers();
  installAppPreferences();
  void start();
}
