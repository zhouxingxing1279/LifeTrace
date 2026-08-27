import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { cloudSync, type SyncStatusView } from "@/src/services/cloudSync";
import { clientLogger } from "@/src/services/clientObservability";
import { useCloudAuthStore } from "@/src/stores/useCloudAuthStore";
import type { DesktopPlatformCapabilities, DesktopRuntimeValue } from "./runtimeTypes";

const DesktopRuntimeContext = createContext<DesktopRuntimeValue | undefined>(undefined);
const SYNC_STATUS_POLL_MS = 15_000;

function detectCapabilities(): DesktopPlatformCapabilities {
  return {
    localDatabase: true,
    cloudSync: Boolean(window.syncApi),
    credentialVault: Boolean(window.cloudCredentialApi),
    fileDialogs: "__TAURI_INTERNALS__" in window,
    noteFiles: Boolean(window.noteApi),
    photoSync: Boolean(window.photoSyncApi),
  };
}

export function DesktopRuntimeProvider({ children }: { children: ReactNode }) {
  const authenticated = useCloudAuthStore((state) => state.authenticated);
  const cloudPhase = useCloudAuthStore((state) => state.phase);
  const profileId = useCloudAuthStore((state) => state.binding?.profileId);
  const [online, setOnline] = useState(() => navigator.onLine);
  const [syncStatus, setSyncStatus] = useState<SyncStatusView>();
  const [syncRunning, setSyncRunning] = useState(false);
  const capabilities = useMemo(() => detectCapabilities(), []);

  useEffect(() => {
    const wentOnline = () => setOnline(true);
    const wentOffline = () => setOnline(false);
    window.addEventListener("online", wentOnline);
    window.addEventListener("offline", wentOffline);
    return () => {
      window.removeEventListener("online", wentOnline);
      window.removeEventListener("offline", wentOffline);
    };
  }, []);

  const refreshSyncStatus = useCallback(async () => {
    if (!window.syncApi) {
      setSyncStatus(undefined);
      return;
    }
    try {
      setSyncStatus(await cloudSync.status());
    } catch (error) {
      clientLogger.warn("desktop.runtime.sync_status_failed", undefined, error);
    }
  }, []);

  const syncNow = useCallback(async (forceSnapshot = false) => {
    if (!window.syncApi || !authenticated || cloudPhase !== "authenticated") return;
    setSyncRunning(true);
    try {
      await cloudSync.now(forceSnapshot);
      await refreshSyncStatus();
    } catch (error) {
      clientLogger.warn("desktop.runtime.sync_failed", { forceSnapshot }, error);
      await refreshSyncStatus();
    } finally {
      setSyncRunning(false);
    }
  }, [authenticated, cloudPhase, refreshSyncStatus]);

  useEffect(() => {
    void refreshSyncStatus();
    const timer = window.setInterval(() => void refreshSyncStatus(), SYNC_STATUS_POLL_MS);
    return () => window.clearInterval(timer);
  }, [profileId, refreshSyncStatus]);

  useEffect(() => {
    if (!authenticated || cloudPhase !== "authenticated") return;
    // Cloud is a background replication target. A failed sync never blocks the
    // local Desktop runtime from rendering or accepting local mutations.
    void syncNow(false);
  }, [authenticated, cloudPhase, syncNow]);

  const value = useMemo<DesktopRuntimeValue>(() => ({
    online,
    authenticated,
    cloudPhase,
    profileId,
    syncStatus,
    syncRunning,
    capabilities,
    refreshSyncStatus,
    syncNow,
  }), [
    authenticated,
    capabilities,
    cloudPhase,
    online,
    profileId,
    refreshSyncStatus,
    syncNow,
    syncRunning,
    syncStatus,
  ]);

  return <DesktopRuntimeContext.Provider value={value}>{children}</DesktopRuntimeContext.Provider>;
}

export function useDesktopRuntime(): DesktopRuntimeValue {
  const value = useContext(DesktopRuntimeContext);
  if (!value) throw new Error("useDesktopRuntime must be used inside DesktopRuntimeProvider");
  return value;
}
