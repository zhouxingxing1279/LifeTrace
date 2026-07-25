"use client";

import { useEffect, useState } from "react";
import { Check, Download, Share, WifiOff, X } from "lucide-react";

interface InstallPromptEvent extends Event {
  prompt: () => Promise<void>;
  userChoice: Promise<{ outcome: "accepted" | "dismissed"; platform: string }>;
}

const isStandalone = () =>
  window.matchMedia("(display-mode: standalone)").matches ||
  ("standalone" in window.navigator && Boolean((window.navigator as Navigator & { standalone?: boolean }).standalone));

export default function PwaManager() {
  const [installPrompt, setInstallPrompt] = useState<InstallPromptEvent | null>(null);
  const [installed, setInstalled] = useState(false);
  const [isIos, setIsIos] = useState(false);
  const [showIosGuide, setShowIosGuide] = useState(false);
  const [online, setOnline] = useState(true);
  const [fitnessMode, setFitnessMode] = useState(false);

  useEffect(() => {
    const fitnessRoute = window.location.pathname.startsWith("/fitness");
    setInstalled(isStandalone());
    setOnline(navigator.onLine);
    setIsIos(/iphone|ipad|ipod/i.test(navigator.userAgent));
    setFitnessMode(fitnessRoute);

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
    const onOnline = () => setOnline(true);
    const onOffline = () => setOnline(false);

    window.addEventListener("beforeinstallprompt", onInstallPrompt);
    window.addEventListener("appinstalled", onInstalled);
    window.addEventListener("online", onOnline);
    window.addEventListener("offline", onOffline);
    return () => {
      window.removeEventListener("beforeinstallprompt", onInstallPrompt);
      window.removeEventListener("appinstalled", onInstalled);
      window.removeEventListener("online", onOnline);
      window.removeEventListener("offline", onOffline);
    };
  }, []);

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
    {!online && <div className="pwa-status offline" role="status"><WifiOff /> 手机需要连接电脑所在网络才能上传文件。</div>}
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
