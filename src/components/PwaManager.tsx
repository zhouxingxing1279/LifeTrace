"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Check, Download, LoaderCircle, RefreshCw, Share, WifiOff, X } from "lucide-react";

interface InstallPromptEvent extends Event {
  prompt: () => Promise<void>;
  userChoice: Promise<{ outcome: "accepted" | "dismissed"; platform: string }>;
}

type ConnectionState = "checking" | "connected" | "disconnected";

const isStandalone = () =>
  window.matchMedia("(display-mode: standalone)").matches ||
  ("standalone" in window.navigator && Boolean((window.navigator as Navigator & { standalone?: boolean }).standalone));

export default function PwaManager() {
  const [installPrompt, setInstallPrompt] = useState<InstallPromptEvent | null>(null);
  const [installed, setInstalled] = useState(() => typeof window !== "undefined" && isStandalone());
  const [isIos] = useState(() => typeof navigator !== "undefined" && /iphone|ipad|ipod/i.test(navigator.userAgent));
  const [showIosGuide, setShowIosGuide] = useState(false);
  const [fitnessMode] = useState(() => typeof window !== "undefined" && window.location.pathname.startsWith("/fitness"));
  const [connection, setConnection] = useState<ConnectionState>("checking");
  const connectionRequest = useRef<AbortController | null>(null);

  const checkConnection = useCallback(async (showProgress = true) => {
    if (!fitnessMode) return;
    connectionRequest.current?.abort();
    const controller = new AbortController();
    connectionRequest.current = controller;
    if (showProgress) setConnection("checking");
    const timeout = window.setTimeout(() => controller.abort(), 5_000);
    try {
      const response = await fetch("/api/health", {
        cache: "no-store",
        headers: { accept: "application/json" },
        signal: controller.signal,
      });
      const payload = response.ok ? await response.json() as { ok?: boolean; service?: string } : null;
      if (connectionRequest.current === controller) {
        setConnection(payload?.ok && payload.service === "lifetrace-upload" ? "connected" : "disconnected");
      }
    } catch {
      if (connectionRequest.current === controller) setConnection("disconnected");
    } finally {
      window.clearTimeout(timeout);
      if (connectionRequest.current === controller) connectionRequest.current = null;
    }
  }, [fitnessMode]);

  useEffect(() => {
    // 新手机端只传输文件：清理旧版本的 Service Worker 与离线缓存。
    if ("serviceWorker" in navigator) {
      void navigator.serviceWorker.getRegistrations().then((registrations) =>
        Promise.all(registrations.map((registration) => registration.unregister())),
      );
    }
    if ("caches" in window) {
      void caches.keys().then((keys) =>
        Promise.all(keys.filter((key) => key.startsWith("hengxu-")).map((key) => caches.delete(key))),
      );
    }

    const onInstallPrompt = (event: Event) => {
      event.preventDefault();
      setInstallPrompt(event as InstallPromptEvent);
    };
    const onInstalled = () => {
      setInstalled(true);
      setInstallPrompt(null);
    };
    const recheckConnection = () => void checkConnection(false);
    const onVisibilityChange = () => {
      if (document.visibilityState === "visible") recheckConnection();
    };

    window.addEventListener("beforeinstallprompt", onInstallPrompt);
    window.addEventListener("appinstalled", onInstalled);
    const initialCheck = fitnessMode
      ? window.setTimeout(() => void checkConnection(), 0)
      : null;
    if (fitnessMode) {
      window.addEventListener("online", recheckConnection);
      window.addEventListener("offline", recheckConnection);
      document.addEventListener("visibilitychange", onVisibilityChange);
    }
    const connectionTimer = fitnessMode
      ? window.setInterval(() => void checkConnection(false), 15_000)
      : null;
    return () => {
      window.removeEventListener("beforeinstallprompt", onInstallPrompt);
      window.removeEventListener("appinstalled", onInstalled);
      window.removeEventListener("online", recheckConnection);
      window.removeEventListener("offline", recheckConnection);
      document.removeEventListener("visibilitychange", onVisibilityChange);
      if (initialCheck !== null) window.clearTimeout(initialCheck);
      if (connectionTimer !== null) window.clearInterval(connectionTimer);
      connectionRequest.current?.abort();
    };
  }, [checkConnection, fitnessMode]);

  const install = async () => {
    if (installPrompt) {
      await installPrompt.prompt();
      const choice = await installPrompt.userChoice;
      if (choice.outcome === "accepted") setInstalled(true);
      setInstallPrompt(null);
    } else if (isIos) {
      setShowIosGuide(true);
    }
  };

  const showInstall = fitnessMode && !installed && (Boolean(installPrompt) || isIos);
  return <>
    {fitnessMode && connection === "checking" && (
      <div className="pwa-status checking" role="status" aria-live="polite">
        <LoaderCircle className="spinning" /> 正在连接电脑上传服务…
      </div>
    )}
    {fitnessMode && connection === "disconnected" && (
      <div className="pwa-status offline" role="alert">
        <WifiOff />
        <span>无法连接电脑上传服务，请确认手机和电脑连接同一 Wi-Fi。</span>
        <button className="pwa-retry" type="button" onClick={() => void checkConnection()} aria-label="重新检测电脑上传服务">
          <RefreshCw /> 重试
        </button>
      </div>
    )}
    {showInstall && <button className="pwa-install" onClick={install} aria-label="安装 Life trace 导入"><Download /><span><strong>安装 Life trace 导入</strong><small>添加到桌面，快速发送数据</small></span></button>}
    {showIosGuide && <div className="pwa-guide-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) setShowIosGuide(false); }}>
      <section className="pwa-guide" role="dialog" aria-modal="true" aria-labelledby="pwa-guide-title">
        <button className="pwa-guide-close" onClick={() => setShowIosGuide(false)} aria-label="关闭安装说明"><X /></button>
        <span className="pwa-guide-mark">LT</span>
        <h2 id="pwa-guide-title">把 Life trace 导入添加到 iPhone</h2>
        <p>使用 Safari 打开当前页面，然后完成下面两步。</p>
        <ol>
          <li><i><Share /></i><span><strong>点击浏览器底部的“分享”</strong><small>图标是一个向上箭头的方框。</small></span></li>
          <li><i><Check /></i><span><strong>选择“添加到主屏幕”</strong><small>确认后即可从桌面快速上传。</small></span></li>
        </ol>
        <button className="hx-btn primary" onClick={() => setShowIosGuide(false)}>我知道了</button>
      </section>
    </div>}
  </>;
}
