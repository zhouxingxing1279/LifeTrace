"use client";

import { useEffect, useRef, useState } from "react";
import { Cloud, LockKeyhole } from "lucide-react";
import PhotoSyncDashboard from "@/src/components/PhotoSyncDashboard";
import LocalVaultModule from "@/src/components/LocalVaultModule";
import { lockVaultBeforeLeave } from "@/src/lib/vaultAutoLock";

export default function PhotoSyncModule() {
  const [mode, setMode] = useState<"sync" | "vault">("sync");
  const modeRef = useRef(mode);

  useEffect(() => {
    modeRef.current = mode;
  }, [mode]);

  useEffect(() => () => {
    if (modeRef.current !== "vault" || typeof window === "undefined") return;
    void lockVaultBeforeLeave(window.vaultApi).catch(() => undefined);
  }, []);

  const switchMode = async (nextMode: "sync" | "vault") => {
    if (nextMode === mode) return;

    if (mode === "vault" && nextMode === "sync") {
      try {
        await lockVaultBeforeLeave(typeof window === "undefined" ? undefined : window.vaultApi);
      } catch (cause) {
        console.error("Failed to lock the private vault before leaving its tab", cause);
        window.alert("私密相册锁定失败，请重试后再离开此页签。");
        return;
      }
    }

    setMode(nextMode);
  };

  return <div className="photo-album-shell">
    <div className="photo-album-tabs" role="tablist" aria-label="相册模式">
      <button role="tab" aria-selected={mode === "sync"} className={mode === "sync" ? "active" : ""} onClick={() => void switchMode("sync")}><Cloud/>同步相册</button>
      <button role="tab" aria-selected={mode === "vault"} className={mode === "vault" ? "active" : ""} onClick={() => void switchMode("vault")}><LockKeyhole/>私密相册</button>
    </div>
    {mode === "sync" ? <PhotoSyncDashboard/> : <LocalVaultModule/>}
  </div>;
}
