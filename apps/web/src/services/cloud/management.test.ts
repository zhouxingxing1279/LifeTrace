import { describe, expect, it } from "vitest";
import { dedupeDeviceInstallations, dedupeManagedSessions } from "./management";
import type { DeviceInstallation, ManagedSession } from "./types";

function device(overrides: Partial<DeviceInstallation> = {}): DeviceInstallation {
  return {
    id: "device-row-1",
    externalDeviceId: "external-1",
    deviceGroupId: null,
    deviceName: "Chrome on Windows",
    appId: "lifetrace-web",
    platform: "web",
    status: "active",
    clientVersion: "0.4.0",
    firstSeenAt: "2026-08-25T07:00:00.000Z",
    lastSeenAt: "2026-08-25T07:00:00.000Z",
    lastLoginAt: null,
    lastSyncAt: null,
    revokedAt: null,
    current: false,
    ...overrides,
  };
}

function session(overrides: Partial<ManagedSession> = {}): ManagedSession {
  return {
    id: "session-1",
    appId: "lifetrace-web",
    deviceId: "browser-device-1",
    sessionType: "web",
    status: "active",
    scopes: ["sessions:read"],
    publicDevice: false,
    createdAt: "2026-08-25T07:00:00.000Z",
    lastSeenAt: "2026-08-25T07:00:00.000Z",
    idleExpiresAt: "2026-08-26T07:00:00.000Z",
    absoluteExpiresAt: "2026-09-25T07:00:00.000Z",
    revokedAt: null,
    current: false,
    ...overrides,
  };
}

describe("dedupeDeviceInstallations", () => {
  it("collapses visually identical devices even when ids and timestamps differ", () => {
    const result = dedupeDeviceInstallations([
      device({ id: "old", externalDeviceId: "old-external", lastSeenAt: "2026-08-25T07:00:00.000Z" }),
      device({ id: "new", externalDeviceId: "new-external", lastSeenAt: "2026-08-25T08:00:00.000Z" }),
    ]);

    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("new");
  });

  it("prefers the current device over a newer duplicate row", () => {
    const result = dedupeDeviceInstallations([
      device({ id: "current", current: true, lastSeenAt: "2026-08-25T07:00:00.000Z" }),
      device({ id: "newer", current: false, lastSeenAt: "2026-08-25T09:00:00.000Z" }),
    ]);

    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("current");
  });

  it("keeps devices with different visible content", () => {
    const result = dedupeDeviceInstallations([
      device({ id: "web", platform: "web" }),
      device({ id: "android", platform: "android" }),
    ]);

    expect(result).toHaveLength(2);
  });
});

describe("dedupeManagedSessions", () => {
  it("collapses identical security rows and keeps the latest session", () => {
    const result = dedupeManagedSessions([
      session({ id: "old", lastSeenAt: "2026-08-25T07:00:00.000Z" }),
      session({ id: "new", lastSeenAt: "2026-08-25T08:00:00.000Z" }),
    ]);

    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("new");
  });

  it("prefers a current session over a newer duplicate", () => {
    const result = dedupeManagedSessions([
      session({ id: "current", current: true, lastSeenAt: "2026-08-25T07:00:00.000Z" }),
      session({ id: "newer", current: false, lastSeenAt: "2026-08-25T09:00:00.000Z" }),
    ]);

    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("current");
  });

  it("keeps public and private sessions separate because their displayed content differs", () => {
    const result = dedupeManagedSessions([
      session({ id: "private", publicDevice: false }),
      session({ id: "public", publicDevice: true }),
    ]);

    expect(result).toHaveLength(2);
  });
});
