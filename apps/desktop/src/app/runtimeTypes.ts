import type { AuthPhase } from "@/src/stores/useCloudAuthStore";
import type { SyncStatusView } from "@/src/services/cloudSync";

export type DesktopPlatformCapabilities = {
  localDatabase: boolean;
  cloudSync: boolean;
  credentialVault: boolean;
  fileDialogs: boolean;
  noteFiles: boolean;
  photoSync: boolean;
};

export type DesktopRuntimeValue = {
  online: boolean;
  authenticated: boolean;
  cloudPhase: AuthPhase;
  profileId?: string;
  syncStatus?: SyncStatusView;
  syncRunning: boolean;
  capabilities: DesktopPlatformCapabilities;
  refreshSyncStatus(): Promise<void>;
  syncNow(forceSnapshot?: boolean): Promise<void>;
};
