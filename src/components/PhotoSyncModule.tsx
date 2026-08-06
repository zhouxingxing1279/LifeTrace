"use client";

import { useState } from "react";
import { Cloud, LockKeyhole } from "lucide-react";
import PhotoSyncDashboard from "@/src/components/PhotoSyncDashboard";
import LocalVaultModule from "@/src/components/LocalVaultModule";

export default function PhotoSyncModule() {
  const [mode, setMode] = useState<"sync" | "vault">("sync");

  return <div className="photo-album-shell">
    <div className="photo-album-tabs" role="tablist" aria-label="相册模式">
      <button role="tab" aria-selected={mode === "sync"} className={mode === "sync" ? "active" : ""} onClick={() => setMode("sync")}><Cloud/>同步相册</button>
      <button role="tab" aria-selected={mode === "vault"} className={mode === "vault" ? "active" : ""} onClick={() => setMode("vault")}><LockKeyhole/>私密相册</button>
    </div>
    {mode === "sync" ? <PhotoSyncDashboard/> : <LocalVaultModule/>}
  </div>;
}
