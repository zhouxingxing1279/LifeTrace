export type SessionBindingResult = {
  profileId: string;
  cloudUserId: string;
  bindingRequired: boolean;
  alreadyBound: boolean;
};

export type LocalProfile = {
  id: string;
  displayName: string;
  cloudUserId?: string | null;
  cloudBindingState: string;
  active: boolean;
  createdAt: string;
  updatedAt: string;
};

export type SyncStatusView = {
  profileId: string;
  cloudUserId?: string | null;
  cloudBindingState: string;
  phase: string;
  pendingCount: number;
  conflictCount: number;
  lastSuccessAt?: string | null;
  nextRetryAt?: string | null;
  lastErrorCode?: string | null;
  lastErrorMessage?: string | null;
};

export type SyncRunReport = {
  pushed: number;
  pulled: number;
  confirmedByPull: number;
  conflicts: number;
  snapshotInitialized: boolean;
};

export type SyncConflictView = {
  conflictId: string;
  entityType: string;
  entityId: string;
  kind: string;
  localPayload?: unknown;
  remotePayload?: unknown;
  serverDeleted: boolean;
};

function api() {
  if (!window.syncApi) throw new Error("桌面同步服务不可用");
  return window.syncApi;
}

export const cloudSync = {
  setSession(origin: string, accessToken: string, deviceId: string) {
    return api().setSession(origin, accessToken, deviceId);
  },
  clearSession() { return api().clearSession(); },
  bindCurrentProfile() { return api().bindCurrentProfile(); },
  createCloudProfile(displayName: string) { return api().createCloudProfile(displayName); },
  profiles() { return api().profiles(); },
  setActiveProfile(profileId: string) { return api().setActiveProfile(profileId); },
  status() { return api().status(); },
  now(forceSnapshot = false) { return api().now(forceSnapshot); },
  conflicts() { return api().conflicts(); },
  resolveConflict(conflictId: string, resolution: "accept_remote"|"keep_local"|"discard") {
    return api().resolveConflict(conflictId, resolution);
  },
};
