import { describe, expect, it } from "vitest";
import { CloudDataStore } from "./api";

describe("CloudDataStore sync client identity", () => {
  it("uses the authenticated desktop application identity when supplied", async () => {
    let requestBody: Record<string, unknown> | undefined;
    const fetcher = async (_input: RequestInfo | URL, init?: RequestInit) => {
      requestBody = JSON.parse(String(init?.body)) as Record<string, unknown>;
      return new Response(JSON.stringify({
        snapshotId: "snapshot-1",
        snapshotCursor: "cursor-1",
        items: [],
        nextPageToken: null,
        completed: true,
      }), { status: 200, headers: { "content-type": "application/json" } });
    };
    const store = new CloudDataStore("user-1", "device-1", "", fetcher, {
      appId: "lifetrace-desktop",
      clientVersion: "0.3.3",
      platform: "windows",
    });

    await store.load();

    expect(requestBody?.client).toMatchObject({
      appId: "lifetrace-desktop",
      clientVersion: "0.3.3",
      platform: "windows",
      deviceId: "device-1",
    });
  });
});
