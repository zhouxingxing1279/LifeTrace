export interface VaultLeaveStatus {
  unlocked: boolean;
  lockOnBlur: boolean;
}

export interface VaultLeaveApi {
  status(): Promise<VaultLeaveStatus>;
  lock(): Promise<unknown>;
}

export async function lockVaultBeforeLeave(api?: VaultLeaveApi): Promise<boolean> {
  if (!api) return false;

  const status = await api.status();
  if (!status.unlocked || !status.lockOnBlur) return false;

  await api.lock();
  return true;
}
