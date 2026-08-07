"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { LoaderCircle, RefreshCw, WifiOff } from "lucide-react";

type ConnectionState = "checking" | "connected" | "disconnected";

export default function MobileUploadConnectionStatus() {
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
    if (!fitnessMode) return;
    const recheckConnection = () => void checkConnection(false);
    const onVisibilityChange = () => {
      if (document.visibilityState === "visible") recheckConnection();
    };
    const initialCheck = window.setTimeout(() => void checkConnection(), 0);
    const connectionTimer = window.setInterval(() => void checkConnection(false), 15_000);
    window.addEventListener("online", recheckConnection);
    window.addEventListener("offline", recheckConnection);
    document.addEventListener("visibilitychange", onVisibilityChange);
    return () => {
      window.removeEventListener("online", recheckConnection);
      window.removeEventListener("offline", recheckConnection);
      document.removeEventListener("visibilitychange", onVisibilityChange);
      window.clearTimeout(initialCheck);
      window.clearInterval(connectionTimer);
      connectionRequest.current?.abort();
    };
  }, [checkConnection, fitnessMode]);

  if (!fitnessMode || connection === "connected") return null;

  if (connection === "checking") {
    return <div className="mobile-upload-status checking" role="status" aria-live="polite">
      <LoaderCircle className="spinning" /> 正在连接电脑上传服务…
    </div>;
  }

  return <div className="mobile-upload-status offline" role="alert">
    <WifiOff />
    <span>无法连接电脑上传服务，请确认电脑端已开启手机上传，并且手机和电脑连接同一 Wi-Fi。</span>
    <button className="mobile-upload-retry" type="button" onClick={() => void checkConnection()} aria-label="重新检测电脑上传服务">
      <RefreshCw /> 重试
    </button>
  </div>;
}
