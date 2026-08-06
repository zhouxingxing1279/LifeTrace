import assert from "node:assert/strict";
import test from "node:test";

import { lockVaultBeforeLeave, type VaultLeaveApi } from "../src/lib/vaultAutoLock";

function createApi(status: { unlocked: boolean; lockOnBlur: boolean }) {
  let lockCalls = 0;
  const api: VaultLeaveApi = {
    async status() {
      return status;
    },
    async lock() {
      lockCalls += 1;
    },
  };
  return { api, lockCalls: () => lockCalls };
}

test("locks an unlocked vault before leaving when leave-lock is enabled", async () => {
  const fixture = createApi({ unlocked: true, lockOnBlur: true });

  assert.equal(await lockVaultBeforeLeave(fixture.api), true);
  assert.equal(fixture.lockCalls(), 1);
});

test("does not lock when leave-lock is disabled", async () => {
  const fixture = createApi({ unlocked: true, lockOnBlur: false });

  assert.equal(await lockVaultBeforeLeave(fixture.api), false);
  assert.equal(fixture.lockCalls(), 0);
});

test("does not lock an already locked vault", async () => {
  const fixture = createApi({ unlocked: false, lockOnBlur: true });

  assert.equal(await lockVaultBeforeLeave(fixture.api), false);
  assert.equal(fixture.lockCalls(), 0);
});

test("is a no-op when the desktop vault API is unavailable", async () => {
  assert.equal(await lockVaultBeforeLeave(undefined), false);
});
